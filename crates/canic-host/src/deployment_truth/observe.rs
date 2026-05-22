use super::*;
use crate::{
    install_root::read_named_fleet_install_state_from_root,
    release_set::{
        ROOT_RELEASE_SET_MANIFEST_FILE, configured_fleet_name, configured_fleet_roles,
        load_root_release_set_manifest,
    },
};
use std::{
    collections::BTreeMap,
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
        Ok(configured_roles) => roles = configured_roles,
        Err(err) => unresolved_observations.push(observation_gap(
            "local_config.roles",
            format!(
                "could not resolve configured roles from {}: {err}",
                config.display()
            ),
        )),
    }

    let install_state =
        read_named_fleet_install_state_from_root(&request.icp_root, &request.network, &fleet_name)
            .map_err(|err| DeploymentTruthError::LocalState(err.to_string()))?;
    let raw_config_sha256 = observe_config_sha256(&config, &mut unresolved_observations);
    let observed_identity = Some(local_deployment_identity(
        request,
        &fleet_name,
        raw_config_sha256.clone(),
        install_state
            .as_ref()
            .map(|state| state.root_canister_id.clone()),
    ));
    let observed_artifacts = collect_observed_artifacts(
        &request.icp_root,
        &request.network,
        &roles,
        &mut unresolved_observations,
    );

    Ok(DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: format!("local:{}:{fleet_name}", request.network),
        observed_at: request.observed_at.clone(),
        observed_identity,
        local_config: LocalDeploymentConfigV1 {
            config_path: Some(config.display().to_string()),
            raw_config_sha256,
            canonical_embedded_config_sha256: None,
        },
        observed_canisters: install_state
            .as_ref()
            .map_or_else(Vec::new, install_state_observed_canisters),
        observed_pool: Vec::new(),
        observed_artifacts,
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations,
    })
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
    let roles = configured_fleet_roles(&config).unwrap_or_else(|err| {
        unresolved_artifacts.push(observation_gap(
            "local_config.roles",
            format!(
                "could not resolve configured roles from {}: {err}",
                config.display()
            ),
        ));
        Vec::new()
    });
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
                source: ArtifactSourceV1::LocalBuild,
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
        source: ArtifactSourceV1::LocalBuild,
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

fn local_deployment_identity(
    request: &LocalInventoryRequest,
    fleet_name: &str,
    deployment_manifest_digest: Option<String>,
    root_principal: Option<String>,
) -> DeploymentIdentityV1 {
    DeploymentIdentityV1 {
        deployment_name: fleet_name.to_string(),
        network: request.network.clone(),
        root_principal,
        authority_profile_hash: None,
        role_topology_hash: None,
        deployment_manifest_digest,
        canonical_runtime_config_digest: None,
        role_embedded_config_set_digest: None,
        artifact_set_digest: None,
        pool_identity_set_digest: None,
        canic_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        ic_memory_version: None,
    }
}

fn install_state_observed_canisters(
    state: &crate::install_root::InstallState,
) -> Vec<ObservedCanisterV1> {
    vec![ObservedCanisterV1 {
        canister_id: state.root_canister_id.clone(),
        role: Some("root".to_string()),
        control_class: CanisterControlClassV1::UnknownUnsafe,
        controllers: Vec::new(),
        module_hash: None,
        status: None,
        root_trust_anchor: Some(state.root_canister_id.clone()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("local_install_state".to_string()),
    }]
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
