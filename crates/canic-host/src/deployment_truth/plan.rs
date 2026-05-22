use super::*;
use crate::release_set::{configured_fleet_name, configured_fleet_roles};
use std::path::PathBuf;

///
/// LocalDeploymentPlanRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalDeploymentPlanRequest {
    pub deployment_name: String,
    pub network: String,
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
    let config = deployment_config_path(&request.workspace_root, request.config_path.as_deref());
    let mut unresolved_assumptions = Vec::new();
    let fleet_template = configured_fleet_name(&config).unwrap_or_else(|err| {
        unresolved_assumptions.push(assumption(
            "local_config.fleet_name",
            format!(
                "could not resolve fleet template name from {}: {err}",
                config.display()
            ),
        ));
        request.deployment_name.clone()
    });
    let roles = configured_fleet_roles(&config).unwrap_or_else(|err| {
        unresolved_assumptions.push(assumption(
            "local_config.roles",
            format!(
                "could not resolve configured roles from {}: {err}",
                config.display()
            ),
        ));
        Vec::new()
    });
    let raw_config_sha256 = config_sha256_assumption(&config, &mut unresolved_assumptions);
    let artifact_manifest = collect_local_role_artifact_manifest(&LocalArtifactManifestRequest {
        network: request.network.clone(),
        workspace_root: request.workspace_root.clone(),
        icp_root: request.icp_root.clone(),
        config_path: Some(config),
    });
    unresolved_assumptions.extend(
        artifact_manifest
            .unresolved_artifacts
            .into_iter()
            .map(|gap| assumption(gap.key, gap.description)),
    );

    DeploymentPlanV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_id: format!("local:{}:{}:plan", request.network, request.deployment_name),
        deployment_identity: DeploymentIdentityV1 {
            deployment_name: request.deployment_name.clone(),
            network: request.network.clone(),
            root_principal: None,
            authority_profile_hash: None,
            role_topology_hash: None,
            deployment_manifest_digest: None,
            canonical_runtime_config_digest: None,
            role_embedded_config_set_digest: None,
            artifact_set_digest: None,
            pool_identity_set_digest: None,
            canic_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            ic_memory_version: None,
        },
        trust_domain: TrustDomainV1 {
            root_trust_anchor: None,
            migration_from: None,
        },
        fleet_template,
        runtime_variant: request.runtime_variant.clone(),
        authority_profile: AuthorityProfileV1 {
            profile_id: format!(
                "local:{}:{}:authority",
                request.network, request.deployment_name
            ),
            expected_controllers: Vec::new(),
            staging_controllers: Vec::new(),
            emergency_controllers: Vec::new(),
        },
        role_artifacts: artifact_manifest
            .role_artifacts
            .into_iter()
            .map(|mut artifact| {
                artifact.build_profile.clone_from(&request.build_profile);
                artifact.raw_config_sha256.clone_from(&raw_config_sha256);
                artifact
            })
            .collect(),
        expected_canisters: roles
            .into_iter()
            .map(|role| ExpectedCanisterV1 {
                role,
                canister_id: None,
                control_class: CanisterControlClassV1::DeploymentControlled,
            })
            .collect(),
        expected_pool: Vec::new(),
        expected_verifier_readiness: VerifierReadinessExpectationV1 {
            required: false,
            expected_role_epochs: Vec::new(),
        },
        unresolved_assumptions,
    }
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
