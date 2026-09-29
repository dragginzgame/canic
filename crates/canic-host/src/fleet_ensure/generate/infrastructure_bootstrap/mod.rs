//! Compile explicit supplied identities without pretending an empty Root is a running estate.
//!
//! The bootstrap survey and reviewed effect driver own all live custody and initialization.

#[cfg(test)]
pub(super) mod tests;

use crate::{
    fleet_ensure::{
        generate::{
            EstateSeed, FleetGenerateError, FleetGenerateRequest, FleetSource, GenerationOutput,
            generate, validate_identity_seed_selection,
        },
        model::{DesiredFleet, infrastructure_bootstrap::BootstrapCoordinatorSelection},
    },
    icp::LocalReplicaTarget,
};

/// Compile current artifact authority for a separately reviewed bootstrap survey.
/// Every Root, Store and pool ID is explicit; only the selected Coordinator may be created.
pub fn generate_infrastructure_bootstrap(
    request: &FleetGenerateRequest<'_>,
    coordinator: BootstrapCoordinatorSelection,
    local_replica: Option<&LocalReplicaTarget>,
) -> Result<DesiredFleet, FleetGenerateError> {
    if local_replica.is_some()
        && crate::icp_config::resolve_icp_build_network_from_root(request.root, request.environment)
            .map_err(|error| FleetGenerateError::Authority(error.to_string()))?
            == canic_core::ids::BuildNetwork::Ic
    {
        return Err(FleetGenerateError::Authority(
            "local bootstrap generation requires a local network profile".into(),
        ));
    }
    match generate(request, local_replica, Some(coordinator))? {
        GenerationOutput::Bootstrap(desired) => Ok(*desired),
        GenerationOutput::Ordinary(_) => Err(FleetGenerateError::Authority(
            "expected bootstrap generation".into(),
        )),
    }
}

pub(super) fn validate_seed(
    source: &FleetSource,
    seed: &EstateSeed,
    coordinator: BootstrapCoordinatorSelection,
) -> Result<(), FleetGenerateError> {
    let create = coordinator == BootstrapCoordinatorSelection::Create;
    if seed.fresh_estate
        || seed.treasury.is_some()
        || create != (seed.coordinator == "create")
        || seed.roots.iter().any(|root| root.pool_imports.is_empty())
    {
        return Err(FleetGenerateError::SeedTopology(
            "bootstrap requires explicit Root/Store IDs and held pool imports, no separate treasury, and coordinator = \"create\" only with explicit Create selection".into(),
        ));
    }
    validate_identity_seed_selection(source, seed, create)
}

/// Verify all supplied seed slots and resolve only the explicitly created Coordinator.
pub(in crate::fleet_ensure) fn seed_projection(
    desired: &DesiredFleet,
    original: &str,
    created_coordinator: Option<candid::Principal>,
) -> Result<String, FleetGenerateError> {
    use canic_core::cdk::types::Cycles;
    let invalid = || {
        FleetGenerateError::SeedTopology(
            "bootstrap seed differs from reviewed physical authority".into(),
        )
    };
    let seed: EstateSeed = toml::from_str(original).map_err(|_| invalid())?;
    let mut document: toml::Value = toml::from_str(original).map_err(|_| invalid())?;
    let original_document = document.clone();
    let bootstrap = desired.bootstrap.as_ref().ok_or_else(invalid)?;
    let configured = |name: &str| desired.canisters.iter().find(|entry| entry.name == name);
    let principal = |name: &str| configured(name).and_then(|entry| entry.principal.as_deref());
    let coordinator = principal(&bootstrap.coordinator);
    if seed.schema_version != 1
        || seed.fresh_estate
        || seed.treasury.is_some()
        || seed.fleet_id != bootstrap.fleet_id
        || seed.cycles_ledger != desired.cycles_ledger
        || seed.management_creation_fee_cycles.parse::<Cycles>().ok()
            != desired
                .management_creation_fee_cycles
                .parse::<Cycles>()
                .ok()
        || seed.roots.len() != bootstrap.roots.len()
        || coordinator.map_or(seed.coordinator != "create", |id| seed.coordinator != id)
    {
        return Err(invalid());
    }
    for root in &bootstrap.roots {
        let mut matches = seed
            .roots
            .iter()
            .filter(|entry| entry.placement_subnet == root.placement_subnet.to_string());
        let selected = matches.next().ok_or_else(invalid)?;
        let pools = root
            .canister_pool_imports
            .iter()
            .map(|name| principal(name).ok_or_else(invalid))
            .collect::<Result<Vec<_>, _>>()?;
        if matches.next().is_some()
            || principal(&root.root) != Some(selected.root.as_str())
            || principal(&root.store) != Some(selected.store.as_str())
            || selected
                .pool_imports
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != pools
        {
            return Err(invalid());
        }
    }
    if let Some(created) = created_coordinator {
        if created == candid::Principal::anonymous()
            || created == candid::Principal::management_canister()
            || coordinator.is_some_and(|selected| selected != created.to_text())
        {
            return Err(invalid());
        }
        document["coordinator"] = toml::Value::String(created.to_text());
    }
    if document == original_document {
        return Ok(original.to_owned());
    }
    toml::to_string_pretty(&document).map_err(|_| invalid())
}
