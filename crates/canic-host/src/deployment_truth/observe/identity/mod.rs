use super::super::*;
use super::inventory::LocalInventoryRequest;

pub(super) struct InventoryIdentityFacts<'a> {
    pub(super) canonical_network_id: canic_core::ids::CanonicalNetworkId,
    pub(super) fleet_id: Option<canic_core::ids::FleetId>,
    pub(super) app: String,
    pub(super) root_principal: Option<String>,
    pub(super) deployment_manifest_digest: Option<String>,
    pub(super) canonical_runtime_config_digest: Option<String>,
    pub(super) observed_canisters: &'a [ObservedCanisterV1],
    pub(super) observed_artifacts: &'a [ObservedArtifactV1],
    pub(super) observed_pool: &'a [ObservedPoolCanisterV1],
}

pub(super) fn local_inventory_identity(
    request: &LocalInventoryRequest,
    facts: InventoryIdentityFacts<'_>,
) -> DeploymentIdentityV1 {
    local_deployment_identity(
        request,
        InventoryIdentityInput {
            canonical_network_id: facts.canonical_network_id,
            fleet_id: facts.fleet_id,
            app: facts.app,
            root_principal: facts.root_principal,
            deployment_manifest_digest: facts.deployment_manifest_digest,
            canonical_runtime_config_digest: facts.canonical_runtime_config_digest,
            role_topology_hash: Some(stable_json_sha256_hex(&facts.observed_canisters)),
            artifact_set_digest: Some(stable_json_sha256_hex(&facts.observed_artifacts)),
            pool_identity_set_digest: Some(stable_json_sha256_hex(&facts.observed_pool)),
        },
    )
}
struct InventoryIdentityInput {
    canonical_network_id: canic_core::ids::CanonicalNetworkId,
    fleet_id: Option<canic_core::ids::FleetId>,
    app: String,
    root_principal: Option<String>,
    deployment_manifest_digest: Option<String>,
    canonical_runtime_config_digest: Option<String>,
    role_topology_hash: Option<String>,
    artifact_set_digest: Option<String>,
    pool_identity_set_digest: Option<String>,
}

fn local_deployment_identity(
    request: &LocalInventoryRequest,
    input: InventoryIdentityInput,
) -> DeploymentIdentityV1 {
    DeploymentIdentityV1 {
        canonical_network_id: Some(input.canonical_network_id),
        fleet_id: input.fleet_id,
        fleet_name: request.fleet_name.clone(),
        app: input.app,
        environment: request.environment.clone(),
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
