use super::*;
use crate::{
    icp::{IcpCanisterStatusReport, IcpCli},
    install_root::read_named_fleet_install_state_from_root,
    installed_fleet::{InstalledFleetRequest, resolve_installed_fleet_from_root},
    registry::RegistryEntry,
    release_set::{
        ConfiguredPoolExpectation, ROOT_RELEASE_SET_MANIFEST_FILE, configured_fleet_name,
        configured_fleet_roles, configured_pool_expectations, load_root_release_set_manifest,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

///
/// LocalInventoryRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalInventoryRequest {
    pub deployment_name: String,
    pub network: String,
    pub workspace_root: PathBuf,
    pub icp_root: PathBuf,
    pub config_path: Option<PathBuf>,
    pub observed_at: String,
}

///
/// LocalArtifactManifestRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalArtifactManifestRequest {
    pub network: String,
    pub workspace_root: PathBuf,
    pub icp_root: PathBuf,
    pub config_path: Option<PathBuf>,
}

///
/// DeploymentTruthError
///
#[derive(Debug, ThisError)]
pub enum DeploymentTruthError {
    #[error("failed to read local deployment state: {0}")]
    LocalState(String),
}

/// Collect read-only local deployment facts without querying or mutating IC state.
pub fn collect_local_deployment_inventory(
    request: &LocalInventoryRequest,
) -> Result<DeploymentInventoryV1, DeploymentTruthError> {
    let config = deployment_config_path(&request.workspace_root, request.config_path.as_deref());
    let mut unresolved_observations = Vec::new();
    let mut roles = Vec::new();

    let fleet_name = match configured_fleet_name(&config) {
        Ok(fleet) => fleet,
        Err(err) => {
            unresolved_observations.push(observation_gap(
                "local_config.fleet_name",
                format!(
                    "could not resolve fleet name from {}: {err}",
                    config.display()
                ),
            ));
            request.deployment_name.clone()
        }
    };

    match configured_fleet_roles(&config) {
        Ok(configured_roles) => {
            roles = deployment_truth_roles_with_implicit_wasm_store(configured_roles);
        }
        Err(err) => unresolved_observations.push(observation_gap(
            "local_config.roles",
            format!(
                "could not resolve configured roles from {}: {err}",
                config.display()
            ),
        )),
    }
    let pool_expectations = configured_pool_expectations(&config).unwrap_or_else(|err| {
        unresolved_observations.push(observation_gap(
            "local_config.pools",
            format!(
                "could not resolve configured pool expectations from {}: {err}",
                config.display()
            ),
        ));
        Vec::new()
    });

    let install_state = read_named_fleet_install_state_from_root(
        &request.icp_root,
        &request.network,
        &request.deployment_name,
    )
    .map_err(|err| DeploymentTruthError::LocalState(err.to_string()))?;
    let raw_config_sha256 = observe_config_sha256(&config, &mut unresolved_observations);
    let canonical_runtime_config_digest =
        observe_canonical_runtime_config_digest(&config, &mut unresolved_observations);
    let deployment_manifest_digest = observe_deployment_manifest_digest(
        &request.icp_root,
        &request.network,
        &mut unresolved_observations,
    );
    let observed_artifacts = collect_observed_artifacts(
        &request.icp_root,
        &request.network,
        &roles,
        &mut unresolved_observations,
    );
    let (observed_canisters, observed_pool) = install_state_observations(
        install_state.as_ref(),
        request,
        &fleet_name,
        &pool_expectations,
        &mut unresolved_observations,
    );
    let observed_identity = Some(local_inventory_identity(
        request,
        InventoryIdentityFacts {
            fleet_name: &fleet_name,
            root_principal: install_state
                .as_ref()
                .map(|state| state.root_canister_id.clone()),
            deployment_manifest_digest,
            canonical_runtime_config_digest: canonical_runtime_config_digest.clone(),
            observed_canisters: &observed_canisters,
            observed_artifacts: &observed_artifacts,
            observed_pool: &observed_pool,
        },
    ));

    Ok(DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: format!("local:{}:{}", request.network, request.deployment_name),
        observed_at: request.observed_at.clone(),
        observed_identity,
        local_config: LocalDeploymentConfigV1 {
            config_path: Some(config.display().to_string()),
            raw_config_sha256,
            canonical_embedded_config_sha256: canonical_runtime_config_digest,
        },
        observed_canisters,
        observed_pool,
        observed_artifacts,
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations,
    })
}

fn install_state_observations(
    install_state: Option<&crate::install_root::InstallState>,
    request: &LocalInventoryRequest,
    fleet_name: &str,
    pool_expectations: &[ConfiguredPoolExpectation],
    unresolved_observations: &mut Vec<DeploymentObservationGapV1>,
) -> (Vec<ObservedCanisterV1>, Vec<ObservedPoolCanisterV1>) {
    let Some(state) = install_state else {
        return (Vec::new(), Vec::new());
    };
    let mut observed_canisters = install_state_observed_canisters(
        state,
        &request.icp_root,
        &request.network,
        unresolved_observations,
    );
    let observed_pool = install_state_registry_observations(
        state,
        request,
        fleet_name,
        pool_expectations,
        &mut observed_canisters,
        unresolved_observations,
    );
    (observed_canisters, observed_pool)
}

struct InventoryIdentityFacts<'a> {
    fleet_name: &'a str,
    root_principal: Option<String>,
    deployment_manifest_digest: Option<String>,
    canonical_runtime_config_digest: Option<String>,
    observed_canisters: &'a [ObservedCanisterV1],
    observed_artifacts: &'a [ObservedArtifactV1],
    observed_pool: &'a [ObservedPoolCanisterV1],
}

fn local_inventory_identity(
    request: &LocalInventoryRequest,
    facts: InventoryIdentityFacts<'_>,
) -> DeploymentIdentityV1 {
    local_deployment_identity(
        request,
        InventoryIdentityInput {
            fleet_name: facts.fleet_name,
            root_principal: facts.root_principal,
            deployment_manifest_digest: facts.deployment_manifest_digest,
            canonical_runtime_config_digest: facts.canonical_runtime_config_digest,
            role_topology_hash: Some(stable_json_sha256_hex(&facts.observed_canisters)),
            artifact_set_digest: Some(stable_json_sha256_hex(&facts.observed_artifacts)),
            pool_identity_set_digest: Some(stable_json_sha256_hex(&facts.observed_pool)),
        },
    )
}

/// Collect a read-only manifest of locally materialized role artifacts.
pub fn collect_local_role_artifact_manifest(
    request: &LocalArtifactManifestRequest,
) -> RoleArtifactManifestV1 {
    let config = deployment_config_path(&request.workspace_root, request.config_path.as_deref());
    let mut unresolved_artifacts = Vec::new();
    let fleet_name = configured_fleet_name(&config).unwrap_or_else(|err| {
        unresolved_artifacts.push(observation_gap(
            "local_config.fleet_name",
            format!(
                "could not resolve fleet name from {}: {err}",
                config.display()
            ),
        ));
        "unknown".to_string()
    });
    let roles = configured_fleet_roles(&config).map_or_else(
        |err| {
            unresolved_artifacts.push(observation_gap(
                "local_config.roles",
                format!(
                    "could not resolve configured roles from {}: {err}",
                    config.display()
                ),
            ));
            Vec::new()
        },
        deployment_truth_roles_with_implicit_wasm_store,
    );
    let artifact_root = match resolve_artifact_root_for_observation(
        &request.icp_root,
        &request.network,
        &mut unresolved_artifacts,
    ) {
        Ok(root) => Some(root),
        Err(err) => {
            unresolved_artifacts.push(observation_gap(
                "local_artifacts.root",
                format!(
                    "could not resolve artifact root for network {}: {err}",
                    request.network
                ),
            ));
            None
        }
    };
    let release_entries = artifact_root
        .as_ref()
        .and_then(|root| load_release_entries(root, &mut unresolved_artifacts));
    let role_artifacts = artifact_root.as_ref().map_or_else(Vec::new, |root| {
        roles
            .iter()
            .map(|role| {
                role_artifact_from_local_files(
                    root,
                    role,
                    release_entries.as_ref(),
                    &mut unresolved_artifacts,
                )
            })
            .collect()
    });

    RoleArtifactManifestV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        manifest_id: format!("local:{}:{fleet_name}:artifacts", request.network),
        network: request.network.clone(),
        artifact_root: artifact_root.map(|root| root.display().to_string()),
        role_artifacts,
        unresolved_artifacts,
    }
}

fn collect_observed_artifacts(
    icp_root: &Path,
    network: &str,
    roles: &[String],
    unresolved_observations: &mut Vec<DeploymentObservationGapV1>,
) -> Vec<ObservedArtifactV1> {
    let artifact_root =
        match resolve_artifact_root_for_observation(icp_root, network, unresolved_observations) {
            Ok(root) => root,
            Err(err) => {
                unresolved_observations.push(observation_gap(
                    "local_artifacts.root",
                    format!("could not resolve artifact root for network {network}: {err}"),
                ));
                return Vec::new();
            }
        };

    roles
        .iter()
        .filter_map(|role| {
            let path = artifact_root.join(role).join(format!("{role}.wasm.gz"));
            if !path.is_file() {
                unresolved_observations.push(observation_gap(
                    format!("local_artifacts.{role}"),
                    format!("missing built artifact {}", path.display()),
                ));
                return None;
            }
            let size = fs::metadata(&path).ok().map(|metadata| metadata.len());
            let file_sha256 = observe_file_sha256(&path, role, unresolved_observations);
            let file_sha256_source = file_sha256
                .as_ref()
                .map(|_| ArtifactDigestSourceV1::ObservedFileDigest);
            Some(ObservedArtifactV1 {
                role: role.clone(),
                artifact_path: path.display().to_string(),
                file_sha256,
                file_sha256_source,
                payload_sha256: None,
                payload_size_bytes: size,
                source: deployment_truth_artifact_source(role),
            })
        })
        .collect()
}

fn load_release_entries(
    artifact_root: &Path,
    unresolved_artifacts: &mut Vec<DeploymentObservationGapV1>,
) -> Option<BTreeMap<String, crate::release_set::ReleaseSetEntry>> {
    let manifest_path = artifact_root
        .join("root")
        .join(ROOT_RELEASE_SET_MANIFEST_FILE);
    if !manifest_path.is_file() {
        unresolved_artifacts.push(observation_gap(
            "local_artifacts.release_set_manifest",
            format!("missing release-set manifest {}", manifest_path.display()),
        ));
        return None;
    }
    match load_root_release_set_manifest(&manifest_path) {
        Ok(manifest) => Some(
            manifest
                .entries
                .into_iter()
                .map(|entry| (entry.role.clone(), entry))
                .collect(),
        ),
        Err(err) => {
            unresolved_artifacts.push(observation_gap(
                "local_artifacts.release_set_manifest",
                format!(
                    "could not read release-set manifest {}: {err}",
                    manifest_path.display()
                ),
            ));
            None
        }
    }
}

fn resolve_artifact_root_for_observation(
    icp_root: &Path,
    network: &str,
    unresolved_artifacts: &mut Vec<DeploymentObservationGapV1>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let preferred = icp_root.join(".icp").join(network).join("canisters");
    if preferred.is_dir() {
        return Ok(preferred);
    }

    let local_fallback = icp_root.join(".icp/local/canisters");
    if network != "local" && local_fallback.is_dir() {
        unresolved_artifacts.push(observation_gap(
            "local_artifacts.network_fallback",
            format!(
                "artifact root {} was missing; observing fallback {}",
                preferred.display(),
                local_fallback.display()
            ),
        ));
        return Ok(local_fallback);
    }

    Err(format!("missing built ICP artifacts under {}", preferred.display()).into())
}

pub(super) fn release_set_manifest_digest(
    icp_root: &Path,
    network: &str,
    gaps: &mut Vec<DeploymentObservationGapV1>,
) -> Option<String> {
    let artifact_root = match resolve_artifact_root_for_observation(icp_root, network, gaps) {
        Ok(root) => root,
        Err(err) => {
            gaps.push(observation_gap(
                "deployment_manifest.path",
                format!("could not resolve release-set manifest root: {err}"),
            ));
            return None;
        }
    };
    let manifest_path = artifact_root
        .join("root")
        .join(ROOT_RELEASE_SET_MANIFEST_FILE);
    if !manifest_path.is_file() {
        gaps.push(observation_gap(
            "deployment_manifest.path",
            format!("missing release-set manifest {}", manifest_path.display()),
        ));
        return None;
    }

    match file_sha256_hex(&manifest_path) {
        Ok(hash) => Some(hash),
        Err(err) => {
            gaps.push(observation_gap(
                "deployment_manifest.sha256",
                format!(
                    "could not hash release-set manifest {}: {err}",
                    manifest_path.display()
                ),
            ));
            None
        }
    }
}

fn role_artifact_from_local_files(
    artifact_root: &Path,
    role: &str,
    release_entries: Option<&BTreeMap<String, crate::release_set::ReleaseSetEntry>>,
    unresolved_artifacts: &mut Vec<DeploymentObservationGapV1>,
) -> RoleArtifactV1 {
    let wasm_gz_path = artifact_root.join(role).join(format!("{role}.wasm.gz"));
    let (wasm_gz_size_bytes, observed_wasm_gz_file_sha256) = if wasm_gz_path.is_file() {
        (
            fs::metadata(&wasm_gz_path)
                .ok()
                .map(|metadata| metadata.len()),
            observe_file_sha256(&wasm_gz_path, role, unresolved_artifacts),
        )
    } else {
        unresolved_artifacts.push(observation_gap(
            format!("local_artifacts.{role}"),
            format!("missing built artifact {}", wasm_gz_path.display()),
        ));
        (None, None)
    };
    let observed_wasm_gz_file_sha256_source = observed_wasm_gz_file_sha256
        .as_ref()
        .map(|_| ArtifactDigestSourceV1::ObservedFileDigest);
    let release_entry = release_entries.and_then(|entries| entries.get(role));
    RoleArtifactV1 {
        role: role.to_string(),
        source: deployment_truth_artifact_source(role),
        build_profile: "unknown".to_string(),
        wasm_path: None,
        wasm_gz_path: Some(wasm_gz_path.display().to_string()),
        wasm_gz_size_bytes,
        wasm_sha256: None,
        wasm_gz_sha256: release_entry.map(|entry| entry.payload_sha256_hex.clone()),
        wasm_gz_sha256_source: release_entry.map(|_| ArtifactDigestSourceV1::ReleaseSetManifest),
        observed_wasm_gz_file_sha256,
        observed_wasm_gz_file_sha256_source,
        installed_module_hash: None,
        candid_path: None,
        candid_sha256: None,
        raw_config_sha256: None,
        canonical_embedded_config_sha256: None,
        embedded_topology_sha256: None,
        builder_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        rust_toolchain: None,
        package_version: None,
    }
}

fn observe_file_sha256(
    path: &Path,
    role: &str,
    gaps: &mut Vec<DeploymentObservationGapV1>,
) -> Option<String> {
    match file_sha256_hex(path) {
        Ok(hash) => Some(hash),
        Err(err) => {
            gaps.push(observation_gap(
                format!("local_artifacts.{role}.file_sha256"),
                format!("could not hash artifact {}: {err}", path.display()),
            ));
            None
        }
    }
}

fn observe_config_sha256(
    path: &Path,
    gaps: &mut Vec<DeploymentObservationGapV1>,
) -> Option<String> {
    match file_sha256_hex(path) {
        Ok(hash) => Some(hash),
        Err(err) => {
            gaps.push(observation_gap(
                "local_config.raw_sha256",
                format!("could not hash config {}: {err}", path.display()),
            ));
            None
        }
    }
}

fn observe_deployment_manifest_digest(
    icp_root: &Path,
    network: &str,
    gaps: &mut Vec<DeploymentObservationGapV1>,
) -> Option<String> {
    release_set_manifest_digest(icp_root, network, gaps)
}

fn observe_canonical_runtime_config_digest(
    path: &Path,
    gaps: &mut Vec<DeploymentObservationGapV1>,
) -> Option<String> {
    match canonical_runtime_config_sha256_hex(path) {
        Ok(hash) => Some(hash),
        Err(err) => {
            gaps.push(observation_gap(
                "local_config.canonical_runtime_config_sha256",
                format!(
                    "could not hash canonical runtime config {}: {err}",
                    path.display()
                ),
            ));
            None
        }
    }
}

struct InventoryIdentityInput<'a> {
    fleet_name: &'a str,
    root_principal: Option<String>,
    deployment_manifest_digest: Option<String>,
    canonical_runtime_config_digest: Option<String>,
    role_topology_hash: Option<String>,
    artifact_set_digest: Option<String>,
    pool_identity_set_digest: Option<String>,
}

fn local_deployment_identity(
    request: &LocalInventoryRequest,
    input: InventoryIdentityInput<'_>,
) -> DeploymentIdentityV1 {
    DeploymentIdentityV1 {
        deployment_name: input.fleet_name.to_string(),
        network: request.network.clone(),
        root_principal: input.root_principal,
        authority_profile_hash: None,
        role_topology_hash: input.role_topology_hash,
        deployment_manifest_digest: input.deployment_manifest_digest,
        canonical_runtime_config_digest: input.canonical_runtime_config_digest,
        role_embedded_config_set_digest: None,
        artifact_set_digest: input.artifact_set_digest,
        pool_identity_set_digest: input.pool_identity_set_digest,
        canic_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        ic_memory_version: None,
    }
}

fn install_state_observed_canisters(
    state: &crate::install_root::InstallState,
    icp_root: &Path,
    network: &str,
    gaps: &mut Vec<DeploymentObservationGapV1>,
) -> Vec<ObservedCanisterV1> {
    match read_live_canister_status(icp_root, network, &state.root_canister_id) {
        Ok(report) => vec![observed_root_from_status(state, &report)],
        Err(err) => {
            gaps.push(observation_gap(
                "live_canister_status.root",
                format!(
                    "could not observe live root canister status for {}: {err}",
                    state.root_canister_id
                ),
            ));
            vec![observed_root_from_install_state(state)]
        }
    }
}

fn install_state_registry_observations(
    state: &crate::install_root::InstallState,
    request: &LocalInventoryRequest,
    fleet_name: &str,
    pool_expectations: &[ConfiguredPoolExpectation],
    observed_canisters: &mut Vec<ObservedCanisterV1>,
    gaps: &mut Vec<DeploymentObservationGapV1>,
) -> Vec<ObservedPoolCanisterV1> {
    match resolve_installed_fleet_from_root(
        &InstalledFleetRequest {
            fleet: fleet_name.to_string(),
            network: request.network.clone(),
            icp: "icp".to_string(),
            detect_lost_local_root: false,
        },
        &request.icp_root,
    ) {
        Ok(resolution) => {
            let mut registry_canisters = registry_entries_to_observed_canisters(
                &state.root_canister_id,
                &resolution.registry.entries,
            );
            enrich_registry_observed_canisters(
                &mut registry_canisters,
                &request.icp_root,
                &request.network,
                gaps,
            );
            let mut observed_pool = registry_entries_to_observed_pool(
                &state.root_canister_id,
                &resolution.registry.entries,
                pool_expectations,
                gaps,
            );
            apply_canister_control_to_observed_pool(&mut observed_pool, &registry_canisters);
            observed_canisters.extend(registry_canisters);
            observed_pool
        }
        Err(err) => {
            gaps.push(observation_gap(
                "live_subnet_registry",
                format!(
                    "could not observe live subnet registry for root {}: {err}",
                    state.root_canister_id
                ),
            ));
            Vec::new()
        }
    }
}

pub(super) fn registry_entries_to_observed_canisters(
    root_canister_id: &str,
    entries: &[RegistryEntry],
) -> Vec<ObservedCanisterV1> {
    entries
        .iter()
        .filter(|entry| entry.pid != root_canister_id)
        .filter_map(registry_entry_to_observed_canister)
        .collect()
}

fn registry_entry_to_observed_canister(entry: &RegistryEntry) -> Option<ObservedCanisterV1> {
    let role = entry.role.clone()?;
    Some(ObservedCanisterV1 {
        canister_id: entry.pid.clone(),
        role: Some(role),
        control_class: registry_entry_control_class(entry),
        controllers: Vec::new(),
        module_hash: entry.module_hash.as_deref().map(normalize_module_hash),
        status: None,
        root_trust_anchor: entry.parent_pid.clone(),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry".to_string()),
    })
}

pub(super) fn apply_canister_control_to_observed_pool(
    observed_pool: &mut [ObservedPoolCanisterV1],
    observed_canisters: &[ObservedCanisterV1],
) {
    let control_by_canister = observed_canisters
        .iter()
        .map(|canister| (canister.canister_id.as_str(), canister.control_class))
        .collect::<BTreeMap<_, _>>();
    for pool in observed_pool {
        if let Some(control_class) = control_by_canister.get(pool.canister_id.as_str()) {
            pool.control_class = *control_class;
        }
    }
}

fn enrich_registry_observed_canisters(
    observed_canisters: &mut [ObservedCanisterV1],
    icp_root: &Path,
    network: &str,
    gaps: &mut Vec<DeploymentObservationGapV1>,
) {
    for observed in observed_canisters {
        match read_live_canister_status(icp_root, network, &observed.canister_id) {
            Ok(report) => apply_live_status_to_registry_observation(observed, &report),
            Err(err) => gaps.push(observation_gap(
                live_status_gap_key(observed),
                format!(
                    "could not observe live canister status for role {} at {}: {err}",
                    observed.role.as_deref().unwrap_or("unknown"),
                    observed.canister_id
                ),
            )),
        }
    }
}

pub(super) fn apply_live_status_to_registry_observation(
    observed: &mut ObservedCanisterV1,
    report: &IcpCanisterStatusReport,
) {
    let controllers = report
        .settings
        .as_ref()
        .map(|settings| settings.controllers.clone())
        .unwrap_or_default();
    observed.canister_id = if report.id.is_empty() {
        observed.canister_id.clone()
    } else {
        report.id.clone()
    };
    observed.control_class = classify_registry_observed_control(
        observed.control_class,
        &controllers,
        observed.root_trust_anchor.as_deref(),
    );
    observed.controllers = controllers;
    observed.module_hash = report.module_hash.as_deref().map(normalize_module_hash);
    observed.status = Some(report.status.clone());
    observed.role_assignment_source = Some("subnet_registry+icp_canister_status".to_string());
}

fn live_status_gap_key(observed: &ObservedCanisterV1) -> String {
    observed.role.as_ref().map_or_else(
        || format!("live_canister_status.{}", observed.canister_id),
        |role| format!("live_canister_status.{role}"),
    )
}

fn classify_registry_observed_control(
    fallback: CanisterControlClassV1,
    controllers: &[String],
    root_trust_anchor: Option<&str>,
) -> CanisterControlClassV1 {
    let Some(anchor) = root_trust_anchor else {
        return fallback;
    };
    if controllers.iter().any(|controller| controller == anchor) {
        fallback
    } else {
        CanisterControlClassV1::UnknownUnsafe
    }
}

const fn registry_entry_control_class(entry: &RegistryEntry) -> CanisterControlClassV1 {
    if entry.parent_pid.is_some() {
        CanisterControlClassV1::CanicManagedPool
    } else {
        CanisterControlClassV1::UnknownUnsafe
    }
}

pub(super) fn registry_entries_to_observed_pool(
    root_canister_id: &str,
    entries: &[RegistryEntry],
    pool_expectations: &[ConfiguredPoolExpectation],
    gaps: &mut Vec<DeploymentObservationGapV1>,
) -> Vec<ObservedPoolCanisterV1> {
    let expectations_by_role = pool_expectations_by_role(pool_expectations);
    let mut seen = BTreeSet::new();
    let mut observed = Vec::new();

    for entry in entries {
        if entry.pid == root_canister_id {
            continue;
        }
        let Some(role) = entry.role.as_ref() else {
            continue;
        };
        let Some(expectations) = expectations_by_role.get(role.as_str()) else {
            continue;
        };
        let [expectation] = expectations.as_slice() else {
            gaps.push(observation_gap(
                format!("live_subnet_registry.pool.{role}"),
                format!(
                    "could not assign observed role {role} to one configured pool without ambiguity"
                ),
            ));
            continue;
        };
        if !seen.insert(entry.pid.as_str()) {
            continue;
        }
        observed.push(ObservedPoolCanisterV1 {
            pool: expectation.pool.clone(),
            canister_id: entry.pid.clone(),
            role: Some(role.clone()),
            control_class: pool_control_class(entry),
        });
    }

    observed
}

fn pool_expectations_by_role(
    pool_expectations: &[ConfiguredPoolExpectation],
) -> BTreeMap<&str, Vec<&ConfiguredPoolExpectation>> {
    let mut by_role = BTreeMap::<&str, Vec<&ConfiguredPoolExpectation>>::new();
    for expectation in pool_expectations {
        by_role
            .entry(expectation.canister_role.as_str())
            .or_default()
            .push(expectation);
    }
    by_role
}

const fn pool_control_class(entry: &RegistryEntry) -> CanisterControlClassV1 {
    if entry.parent_pid.is_some() {
        CanisterControlClassV1::CanicManagedPool
    } else {
        CanisterControlClassV1::UnknownUnsafe
    }
}

fn read_live_canister_status(
    icp_root: &Path,
    network: &str,
    canister_id: &str,
) -> Result<IcpCanisterStatusReport, crate::icp::IcpCommandError> {
    IcpCli::new("icp", Some(network.to_string()), None)
        .with_cwd(icp_root)
        .canister_status_report(canister_id)
}

pub(super) fn observed_root_from_status(
    state: &crate::install_root::InstallState,
    report: &IcpCanisterStatusReport,
) -> ObservedCanisterV1 {
    let controllers = report
        .settings
        .as_ref()
        .map(|settings| settings.controllers.clone())
        .unwrap_or_default();
    ObservedCanisterV1 {
        canister_id: if report.id.is_empty() {
            state.root_canister_id.clone()
        } else {
            report.id.clone()
        },
        role: Some("root".to_string()),
        control_class: classify_root_control(&controllers, &state.root_canister_id),
        controllers,
        module_hash: report.module_hash.as_deref().map(normalize_module_hash),
        status: Some(report.status.clone()),
        root_trust_anchor: Some(state.root_canister_id.clone()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("icp_canister_status".to_string()),
    }
}

fn observed_root_from_install_state(
    state: &crate::install_root::InstallState,
) -> ObservedCanisterV1 {
    ObservedCanisterV1 {
        canister_id: state.root_canister_id.clone(),
        role: Some("root".to_string()),
        control_class: CanisterControlClassV1::UnknownUnsafe,
        controllers: Vec::new(),
        module_hash: None,
        status: None,
        root_trust_anchor: Some(state.root_canister_id.clone()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("local_install_state".to_string()),
    }
}

fn classify_root_control(controllers: &[String], root_canister_id: &str) -> CanisterControlClassV1 {
    if controllers
        .iter()
        .any(|controller| controller == root_canister_id)
    {
        CanisterControlClassV1::DeploymentControlled
    } else {
        CanisterControlClassV1::UnknownUnsafe
    }
}

fn normalize_module_hash(hash: &str) -> String {
    hash.strip_prefix("0x")
        .or_else(|| hash.strip_prefix("0X"))
        .unwrap_or(hash)
        .to_ascii_lowercase()
}

fn observation_gap(
    key: impl Into<String>,
    description: impl Into<String>,
) -> DeploymentObservationGapV1 {
    DeploymentObservationGapV1 {
        key: key.into(),
        description: description.into(),
    }
}
