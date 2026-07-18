use super::super::*;
use super::artifacts::{
    collect_observed_artifacts, observe_canonical_runtime_config_digest, observe_config_sha256,
    observe_deployment_manifest_digest,
};
use super::config::observe_local_config_facts;
use super::identity::{InventoryIdentityFacts, local_inventory_identity};
use super::root::{install_state_observations, observed_root_observation};
use crate::install_root::{InstallStateError, read_named_deployment_install_state_from_root};
use std::path::PathBuf;
use thiserror::Error as ThisError;

///
/// LocalInventoryRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalInventoryRequest {
    pub deployment_name: String,
    pub environment: String,
    pub artifact_environment: String,
    pub workspace_root: PathBuf,
    pub icp_root: PathBuf,
    pub config_path: Option<PathBuf>,
    pub observed_at: String,
}

///
/// DeploymentTruthError
///
#[derive(Debug, ThisError)]
pub enum DeploymentTruthError {
    #[error("failed to read local deployment state: {0}")]
    LocalState(#[source] InstallStateError),
}

/// Collect read-only local deployment facts without querying or mutating IC state.
pub fn collect_local_deployment_inventory(
    request: &LocalInventoryRequest,
) -> Result<DeploymentInventoryV1, DeploymentTruthError> {
    let config = deployment_config_path(&request.workspace_root, request.config_path.as_deref());
    let mut unresolved_observations = Vec::new();
    let local_config_facts = observe_local_config_facts(&config, &mut unresolved_observations);

    let install_state = read_named_deployment_install_state_from_root(
        &request.icp_root,
        &request.environment,
        &request.deployment_name,
    )
    .map_err(DeploymentTruthError::LocalState)?;
    let raw_config_sha256 = observe_config_sha256(&config, &mut unresolved_observations);
    let canonical_runtime_config_digest =
        observe_canonical_runtime_config_digest(&config, &mut unresolved_observations);
    let deployment_manifest_digest = observe_deployment_manifest_digest(
        &request.icp_root,
        &request.artifact_environment,
        &mut unresolved_observations,
    );
    let observed_artifacts = collect_observed_artifacts(
        &request.icp_root,
        &request.artifact_environment,
        &local_config_facts.roles,
        &mut unresolved_observations,
    );
    let (observed_canisters, observed_pool) = install_state_observations(
        install_state.as_ref(),
        request,
        &local_config_facts.pool_expectations,
        &mut unresolved_observations,
    );
    let observed_root = observed_root_observation(
        install_state.as_ref(),
        request,
        &local_config_facts.fleet_name,
        &observed_canisters,
    );
    let observed_identity = Some(local_inventory_identity(
        request,
        InventoryIdentityFacts {
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
        inventory_id: format!("local:{}:{}", request.environment, request.deployment_name),
        observed_at: request.observed_at.clone(),
        observed_identity,
        observed_root,
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
