use super::super::*;
use super::artifacts::{
    collect_observed_artifacts, observe_canonical_runtime_config_digest, observe_config_sha256,
    observe_deployment_manifest_digest,
};
use super::config::observe_local_config_facts;
use super::identity::{InventoryIdentityFacts, local_inventory_identity};
use super::root::{fleet_catalog_observations, observed_root_observation};
use crate::fleet_catalog::{FleetCatalogError, read_fleet_catalog_entry_from_root};
use crate::network::resolve_canonical_network_id_from_root;
use std::path::PathBuf;
use thiserror::Error as ThisError;

///
/// LocalInventoryRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalInventoryRequest {
    pub fleet_name: String,
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
    #[error("failed to read the canonical-network Fleet catalog: {0}")]
    FleetCatalog(#[source] FleetCatalogError),
}

/// Collect read-only local deployment facts without querying or mutating IC state.
pub fn collect_local_deployment_inventory(
    request: &LocalInventoryRequest,
) -> Result<DeploymentInventoryV1, DeploymentTruthError> {
    let config = deployment_config_path(&request.workspace_root, request.config_path.as_deref());
    let mut unresolved_observations = Vec::new();
    let local_config_facts = observe_local_config_facts(&config, &mut unresolved_observations);

    let installed_fleet = read_fleet_catalog_entry_from_root(
        &request.icp_root,
        &request.environment,
        &request.fleet_name,
    )
    .map_err(DeploymentTruthError::FleetCatalog)?;
    let canonical_network_id =
        resolve_canonical_network_id_from_root(&request.icp_root, &request.environment)
            .map_err(FleetCatalogError::from)
            .map_err(DeploymentTruthError::FleetCatalog)?;
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
    let (observed_canisters, observed_pool) = fleet_catalog_observations(
        installed_fleet.as_ref(),
        request,
        &local_config_facts.pool_expectations,
        &mut unresolved_observations,
    );
    let observed_root =
        observed_root_observation(installed_fleet.as_ref(), request, &observed_canisters);
    let observed_identity = Some(local_inventory_identity(
        request,
        InventoryIdentityFacts {
            canonical_network_id,
            fleet_id: installed_fleet.as_ref().map(|fleet| fleet.fleet_id),
            app: installed_fleet.as_ref().map_or_else(
                || local_config_facts.app.clone(),
                |fleet| fleet.app.to_string(),
            ),
            root_principal: installed_fleet
                .as_ref()
                .map(|fleet| fleet.root_principal.clone()),
            deployment_manifest_digest,
            canonical_runtime_config_digest: canonical_runtime_config_digest.clone(),
            observed_canisters: &observed_canisters,
            observed_artifacts: &observed_artifacts,
            observed_pool: &observed_pool,
        },
    ));

    Ok(DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: format!("local:{}:{}", request.environment, request.fleet_name),
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
