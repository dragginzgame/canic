use super::*;
use crate::{
    install_root::{RootVerificationStatus, read_named_deployment_install_state_from_root},
    release_set::{
        ConfiguredPoolExpectation, configured_bootstrap_roles, configured_controllers,
        configured_fleet_name, configured_pool_expectations,
    },
};
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
    let roles = configured_bootstrap_roles(&config).map_or_else(
        |err| {
            unresolved_assumptions.push(assumption(
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
    let expected_controllers = configured_controllers(&config).unwrap_or_else(|err| {
        unresolved_assumptions.push(assumption(
            "local_config.controllers",
            format!(
                "could not resolve configured controllers from {}: {err}",
                config.display()
            ),
        ));
        Vec::new()
    });
    let expected_pool = configured_pool_expectations(&config).map_or_else(
        |err| {
            unresolved_assumptions.push(assumption(
                "local_config.pools",
                format!(
                    "could not resolve configured pool expectations from {}: {err}",
                    config.display()
                ),
            ));
            Vec::new()
        },
        local_expected_pool,
    );
    let root_canister_id = local_root_canister_id(request, &mut unresolved_assumptions);
    let raw_config_sha256 = config_sha256_assumption(&config, &mut unresolved_assumptions);
    let canonical_runtime_config_digest =
        canonical_runtime_config_assumption(&config, &mut unresolved_assumptions);
    let deployment_manifest_digest =
        deployment_manifest_digest_assumption(request, &mut unresolved_assumptions);
    let artifact_manifest = local_artifact_manifest(request, config);
    extend_artifact_assumptions(
        &mut unresolved_assumptions,
        artifact_manifest.unresolved_artifacts,
    );
    let authority_profile = local_authority_profile(request, expected_controllers);
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
        plan_id: format!("local:{}:{}:plan", request.network, request.deployment_name),
        deployment_identity: identity,
        trust_domain: TrustDomainV1 {
            root_trust_anchor: root_canister_id,
            migration_from: None,
        },
        fleet_template,
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

struct PlanIdentityFacts<'a> {
    root_canister_id: Option<String>,
    deployment_manifest_digest: Option<String>,
    canonical_runtime_config_digest: Option<String>,
    authority_profile: &'a AuthorityProfileV1,
    expected_canisters: &'a [ExpectedCanisterV1],
    role_artifacts: &'a [RoleArtifactV1],
    expected_pool: &'a [ExpectedPoolCanisterV1],
}

fn local_artifact_manifest(
    request: &LocalDeploymentPlanRequest,
    config: PathBuf,
) -> RoleArtifactManifestV1 {
    collect_local_role_artifact_manifest(&LocalArtifactManifestRequest {
        network: request.network.clone(),
        workspace_root: request.workspace_root.clone(),
        icp_root: request.icp_root.clone(),
        config_path: Some(config),
    })
}

fn local_plan_identity(
    request: &LocalDeploymentPlanRequest,
    facts: PlanIdentityFacts<'_>,
) -> DeploymentIdentityV1 {
    local_deployment_identity(
        request,
        PlanIdentityInput {
            root_canister_id: facts.root_canister_id,
            deployment_manifest_digest: facts.deployment_manifest_digest,
            canonical_runtime_config_digest: facts.canonical_runtime_config_digest,
            authority_profile_hash: Some(stable_json_sha256_hex(facts.authority_profile)),
            role_topology_hash: Some(stable_json_sha256_hex(&facts.expected_canisters)),
            artifact_set_digest: Some(stable_json_sha256_hex(&facts.role_artifacts)),
            pool_identity_set_digest: Some(stable_json_sha256_hex(&facts.expected_pool)),
        },
    )
}

fn local_root_canister_id(
    request: &LocalDeploymentPlanRequest,
    assumptions: &mut Vec<DeploymentAssumptionV1>,
) -> Option<String> {
    match read_named_deployment_install_state_from_root(
        &request.icp_root,
        &request.network,
        &request.deployment_name,
    ) {
        Ok(Some(state))
            if state.network == request.network
                && state.root_verification == RootVerificationStatus::Verified =>
        {
            Some(state.root_canister_id)
        }
        Ok(Some(state)) if state.network == request.network => {
            assumptions.push(assumption(
                "local_state.unverified_root_canister_id",
                format!(
                    "deployment state for {} records root {}, but root verification is {:?}; run deploy check/verification before mutation authority is trusted",
                    request.deployment_name, state.root_canister_id, state.root_verification
                ),
            ));
            None
        }
        Ok(Some(state)) => {
            assumptions.push(assumption(
                DeploymentAssumptionKindV1::LocalStateNetworkMismatch.key(),
                format!(
                    "deployment state for {} has network {}, expected {}",
                    request.deployment_name, state.network, request.network
                ),
            ));
            None
        }
        Ok(None) => {
            assumptions.push(assumption(
                DeploymentAssumptionKindV1::LocalStateMissing.key(),
                format!(
                    "no local deployment state exists for {}; root identity is unknown until install or explicit deploy register with --allow-unverified",
                    request.deployment_name
                ),
            ));
            None
        }
        Err(err) => {
            assumptions.push(assumption(
                DeploymentAssumptionKindV1::LocalStateReadFailed.key(),
                format!(
                    "could not read deployment state for {}: {err}",
                    request.deployment_name
                ),
            ));
            None
        }
    }
}

struct PlanIdentityInput {
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
        deployment_name: request.deployment_name.clone(),
        network: request.network.clone(),
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

fn local_authority_profile(
    request: &LocalDeploymentPlanRequest,
    expected_controllers: Vec<String>,
) -> AuthorityProfileV1 {
    AuthorityProfileV1 {
        profile_id: format!(
            "local:{}:{}:authority",
            request.network, request.deployment_name
        ),
        expected_controllers,
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
    assumptions: &mut Vec<DeploymentAssumptionV1>,
) -> Option<String> {
    let mut gaps = Vec::new();
    let digest =
        super::observe::release_set_manifest_digest(&request.icp_root, &request.network, &mut gaps);
    assumptions.extend(
        gaps.into_iter()
            .map(|gap| assumption(gap.key, gap.description)),
    );
    digest
}
