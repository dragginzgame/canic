use super::*;
use crate::{
    fleet_catalog::{FleetCatalogEntryV1, read_fleet_catalog_entry_from_root},
    network::resolve_canonical_network_id_from_root,
    release_set::{AppConfigSnapshot, ConfiguredPoolExpectation, artifact_root_path},
};
use canic_core::ids::CanonicalNetworkId;
use std::path::{Path, PathBuf};

///
/// LocalDeploymentPlanRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalDeploymentPlanRequest {
    pub fleet_name: String,
    pub app: String,
    pub environment: String,
    pub artifact_environment: String,
    pub workspace_root: PathBuf,
    pub icp_root: PathBuf,
    pub config_path: Option<PathBuf>,
    pub runtime_variant: String,
    pub build_profile: String,
}

/// Build a local deployment plan from resolved host config and local artifact
/// observations without querying or mutating IC state.
#[must_use]
pub fn build_local_deployment_plan(request: &LocalDeploymentPlanRequest) -> DeploymentPlanV1 {
    let artifact_root = artifact_root_path(&request.icp_root, &request.artifact_environment);
    build_local_deployment_plan_at_root(request, &artifact_root)
}

pub fn build_local_deployment_plan_at_root(
    request: &LocalDeploymentPlanRequest,
    artifact_root: &Path,
) -> DeploymentPlanV1 {
    let config = deployment_config_path(&request.workspace_root, request.config_path.as_deref());
    let mut unresolved_assumptions = Vec::new();
    let (roles, expected_pool) = match AppConfigSnapshot::load(&config) {
        Ok(snapshot) => {
            if snapshot.app_id() != request.app {
                unresolved_assumptions.push(assumption(
                    "local_config.app",
                    format!(
                        "{} declares App {}, not requested App {}",
                        config.display(),
                        snapshot.app_id(),
                        request.app
                    ),
                ));
            }
            (
                deployment_truth_roles_with_built_in_infrastructure(snapshot.bootstrap_roles()),
                local_expected_pool(snapshot.pool_expectations()),
            )
        }
        Err(err) => {
            for (code, subject) in [
                ("local_config.app", "App identity"),
                ("local_config.roles", "configured roles"),
                ("local_config.pools", "configured pool expectations"),
            ] {
                unresolved_assumptions.push(assumption(
                    code,
                    format!(
                        "could not resolve {subject} from {}: {err}",
                        config.display()
                    ),
                ));
            }
            (Vec::new(), Vec::new())
        }
    };
    let resolved_fleet = local_fleet_identity(request, &mut unresolved_assumptions);
    // The Coordinator-anchored Fleet catalog no longer identifies one
    // deployment root. Root-specific deployment truth must come from the
    // multi-root Registry rather than discovery metadata.
    let root_canister_id = None;
    let raw_config_sha256 = config_sha256_assumption(&config, &mut unresolved_assumptions);
    let canonical_runtime_config_digest =
        canonical_runtime_config_assumption(&config, &mut unresolved_assumptions);
    let deployment_manifest_digest =
        deployment_manifest_digest_assumption(request, artifact_root, &mut unresolved_assumptions);
    let artifact_manifest = local_artifact_manifest(request, artifact_root, config);
    extend_artifact_assumptions(
        &mut unresolved_assumptions,
        artifact_manifest.unresolved_artifacts,
    );
    let authority_profile = local_authority_profile(request);
    let role_artifacts = local_plan_role_artifacts(
        artifact_manifest.role_artifacts,
        &request.build_profile,
        raw_config_sha256.as_ref(),
    );
    let expected_canisters = local_expected_canisters(roles, root_canister_id.as_deref());
    let identity = local_plan_identity(
        request,
        PlanIdentityFacts {
            root_canister_id: root_canister_id.clone(),
            resolved_fleet: &resolved_fleet,
            deployment_manifest_digest,
            canonical_runtime_config_digest,
            authority_profile: &authority_profile,
            expected_canisters: &expected_canisters,
            role_artifacts: &role_artifacts,
            expected_pool: &expected_pool,
        },
    );
    DeploymentPlanV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_id: local_plan_id(request),
        deployment_identity: identity,
        trust_domain: TrustDomainV1 {
            root_trust_anchor: root_canister_id,
        },
        runtime_variant: request.runtime_variant.clone(),
        authority_profile,
        role_artifacts,
        expected_canisters,
        expected_pool,
        expected_verifier_readiness: VerifierReadinessExpectationV1 {
            required: false,
            expected_role_epochs: Vec::new(),
        },
        unresolved_assumptions,
    }
}

fn local_plan_id(request: &LocalDeploymentPlanRequest) -> String {
    format!("local:{}:{}:plan", request.environment, request.fleet_name)
}

struct PlanIdentityFacts<'a> {
    root_canister_id: Option<String>,
    resolved_fleet: &'a ResolvedPlanFleet,
    deployment_manifest_digest: Option<String>,
    canonical_runtime_config_digest: Option<String>,
    authority_profile: &'a AuthorityProfileV1,
    expected_canisters: &'a [ExpectedCanisterV1],
    role_artifacts: &'a [RoleArtifactV1],
    expected_pool: &'a [ExpectedPoolCanisterV1],
}

fn local_artifact_manifest(
    request: &LocalDeploymentPlanRequest,
    artifact_root: &Path,
    config: PathBuf,
) -> RoleArtifactManifestV1 {
    collect_local_role_artifact_manifest_at_root(
        &LocalArtifactManifestRequest {
            environment: request.environment.clone(),
            artifact_environment: request.artifact_environment.clone(),
            workspace_root: request.workspace_root.clone(),
            icp_root: request.icp_root.clone(),
            config_path: Some(config),
        },
        artifact_root,
    )
}

fn local_plan_identity(
    request: &LocalDeploymentPlanRequest,
    facts: PlanIdentityFacts<'_>,
) -> DeploymentIdentityV1 {
    local_deployment_identity(
        request,
        PlanIdentityInput {
            root_canister_id: facts.root_canister_id,
            canonical_network_id: facts.resolved_fleet.canonical_network_id,
            fleet_id: facts
                .resolved_fleet
                .catalog
                .as_ref()
                .map(|fleet| fleet.fleet_id),
            deployment_manifest_digest: facts.deployment_manifest_digest,
            canonical_runtime_config_digest: facts.canonical_runtime_config_digest,
            authority_profile_hash: Some(stable_json_sha256_hex(facts.authority_profile)),
            role_topology_hash: Some(stable_json_sha256_hex(&facts.expected_canisters)),
            artifact_set_digest: Some(stable_json_sha256_hex(&facts.role_artifacts)),
            pool_identity_set_digest: Some(stable_json_sha256_hex(&facts.expected_pool)),
        },
    )
}

struct ResolvedPlanFleet {
    canonical_network_id: Option<CanonicalNetworkId>,
    catalog: Option<FleetCatalogEntryV1>,
}

fn local_fleet_identity(
    request: &LocalDeploymentPlanRequest,
    assumptions: &mut Vec<DeploymentAssumptionV1>,
) -> ResolvedPlanFleet {
    let canonical_network_id =
        match resolve_canonical_network_id_from_root(&request.icp_root, &request.environment) {
            Ok(network) => Some(network),
            Err(error) => {
                assumptions.push(assumption(
                    DeploymentAssumptionKindV1::FleetCatalogReadFailed.key(),
                    format!(
                        "could not resolve canonical network identity for {}: {error}",
                        request.environment
                    ),
                ));
                None
            }
        };
    let catalog = if canonical_network_id.is_none() {
        None
    } else {
        match read_fleet_catalog_entry_from_root(
            &request.icp_root,
            &request.environment,
            &request.fleet_name,
        ) {
            Ok(Some(fleet)) => Some(fleet),
            Ok(None) => {
                assumptions.push(assumption(
                DeploymentAssumptionKindV1::FleetCatalogMissing.key(),
                format!(
                    "no installed Fleet catalog entry exists for {}; Coordinator identity is unknown until installation completes",
                    request.fleet_name
                ),
            ));
                None
            }
            Err(error) => {
                assumptions.push(assumption(
                    DeploymentAssumptionKindV1::FleetCatalogReadFailed.key(),
                    format!(
                        "could not read Fleet catalog for {}: {error}",
                        request.fleet_name,
                    ),
                ));
                None
            }
        }
    };
    ResolvedPlanFleet {
        canonical_network_id,
        catalog,
    }
}

struct PlanIdentityInput {
    canonical_network_id: Option<CanonicalNetworkId>,
    fleet_id: Option<canic_core::ids::FleetId>,
    root_canister_id: Option<String>,
    deployment_manifest_digest: Option<String>,
    canonical_runtime_config_digest: Option<String>,
    authority_profile_hash: Option<String>,
    role_topology_hash: Option<String>,
    artifact_set_digest: Option<String>,
    pool_identity_set_digest: Option<String>,
}

fn local_deployment_identity(
    request: &LocalDeploymentPlanRequest,
    input: PlanIdentityInput,
) -> DeploymentIdentityV1 {
    DeploymentIdentityV1 {
        canonical_network_id: input.canonical_network_id,
        fleet_id: input.fleet_id,
        fleet_name: request.fleet_name.clone(),
        app: request.app.clone(),
        environment: request.environment.clone(),
        root_principal: input.root_canister_id,
        authority_profile_hash: input.authority_profile_hash,
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

fn local_authority_profile(request: &LocalDeploymentPlanRequest) -> AuthorityProfileV1 {
    // One App-level controller set cannot represent the role-specific authority
    // retained by Coordinator/root/Store install journals and verified live.
    AuthorityProfileV1 {
        profile_id: format!(
            "local:{}:{}:authority",
            request.environment, request.fleet_name
        ),
        expected_controllers: Vec::new(),
        staging_controllers: Vec::new(),
        emergency_controllers: Vec::new(),
    }
}

fn local_expected_canisters(
    roles: Vec<String>,
    root_canister_id: Option<&str>,
) -> Vec<ExpectedCanisterV1> {
    roles
        .into_iter()
        .map(|role| ExpectedCanisterV1 {
            canister_id: if role == "root" {
                root_canister_id.map(str::to_string)
            } else {
                None
            },
            role,
            control_class: CanisterControlClassV1::DeploymentControlled,
        })
        .collect()
}

fn local_expected_pool(pools: Vec<ConfiguredPoolExpectation>) -> Vec<ExpectedPoolCanisterV1> {
    pools
        .into_iter()
        .map(|pool| ExpectedPoolCanisterV1 {
            pool: pool.pool,
            canister_id: None,
            role: Some(pool.canister_role),
        })
        .collect()
}

fn local_plan_role_artifacts(
    artifacts: Vec<RoleArtifactV1>,
    build_profile: &str,
    raw_config_sha256: Option<&String>,
) -> Vec<RoleArtifactV1> {
    artifacts
        .into_iter()
        .map(|mut artifact| {
            artifact.build_profile = build_profile.to_string();
            artifact.raw_config_sha256 = raw_config_sha256.cloned();
            artifact
        })
        .collect()
}

fn extend_artifact_assumptions(
    assumptions: &mut Vec<DeploymentAssumptionV1>,
    gaps: Vec<DeploymentObservationGapV1>,
) {
    assumptions.extend(
        gaps.into_iter()
            .map(|gap| assumption(gap.key, gap.description)),
    );
}

fn assumption(key: impl Into<String>, description: impl Into<String>) -> DeploymentAssumptionV1 {
    DeploymentAssumptionV1 {
        key: key.into(),
        description: description.into(),
    }
}

fn config_sha256_assumption(
    path: &std::path::Path,
    assumptions: &mut Vec<DeploymentAssumptionV1>,
) -> Option<String> {
    match file_sha256_hex(path) {
        Ok(hash) => Some(hash),
        Err(err) => {
            assumptions.push(assumption(
                "local_config.raw_sha256",
                format!("could not hash config {}: {err}", path.display()),
            ));
            None
        }
    }
}

fn canonical_runtime_config_assumption(
    path: &std::path::Path,
    assumptions: &mut Vec<DeploymentAssumptionV1>,
) -> Option<String> {
    match canonical_runtime_config_sha256_hex(path) {
        Ok(hash) => Some(hash),
        Err(err) => {
            assumptions.push(assumption(
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

fn deployment_manifest_digest_assumption(
    request: &LocalDeploymentPlanRequest,
    artifact_root: &Path,
    assumptions: &mut Vec<DeploymentAssumptionV1>,
) -> Option<String> {
    let mut gaps = Vec::new();
    let digest =
        super::observe::release_set_manifest_digest(&request.icp_root, artifact_root, &mut gaps);
    assumptions.extend(
        gaps.into_iter()
            .map(|gap| assumption(gap.key, gap.description)),
    );
    digest
}
