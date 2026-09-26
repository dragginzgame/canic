//! Retain a fresh Root's exact supplied-capacity hold through infrastructure initialization.

#[cfg(test)]
mod tests;

use crate::{
    ops::canister_pool::capacity_import::CanisterPoolImportOps,
    storage::stable::canister_pool::{
        CanisterPoolStateRecord, CanisterPoolStore, capacity_import::PoolImportBootstrapRecord,
    },
};
use canic_core::{
    cdk::types::Principal,
    control_plane_support::error::InternalError,
    dto::{
        fleet_subnet_root::FleetSubnetRootInitArgs,
        pool_import::{PoolImportBootstrap, PoolImportReservation},
    },
    ids::MAX_FLEET_CAPACITY_IMPORT_SOURCES,
};
use std::collections::BTreeSet;

/// Validate the exact initialization declaration without mutation or platform effects.
pub fn validate(args: &FleetSubnetRootInitArgs) -> Result<(), InternalError> {
    let Some(hold) = &args.capacity_import_bootstrap else {
        return Ok(());
    };
    let binding = &args.authority.binding;
    let store = args.authority.wasm_store_authority.wasm_store;
    let excluded = [
        binding.fleet_subnet_root,
        binding.authority.binding.coordinator,
        store,
        hold.operator,
    ];
    let valid_principal =
        |id: Principal| id != Principal::anonymous() && id != Principal::management_canister();
    let unique = hold.sources.iter().copied().collect::<BTreeSet<_>>();
    let canonical = unique.iter().copied().eq(hold.sources.iter().copied());
    let bounded = !hold.sources.is_empty()
        && hold.sources.len() <= MAX_FLEET_CAPACITY_IMPORT_SOURCES
        && hold.sources.len() <= binding.limits.canister_pool.maximum_size as usize;
    let valid_ids = valid_principal(hold.operator)
        && !excluded[..3].contains(&hold.operator)
        && hold.sources.iter().all(|id| {
            valid_principal(*id)
                && !excluded.contains(id)
                && !binding.authority.binding.recovery_controllers.contains(id)
        });
    if !args.canister_pool_imports.is_empty()
        || !canonical
        || !bounded
        || !valid_ids
        || hold.review_sha256 == [0; 32]
        || args.install_id == [0; 32]
    {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}

/// Seed only a fresh pool owner; this is called synchronously during installation.
pub fn initialize(args: &FleetSubnetRootInitArgs) -> Result<(), InternalError> {
    validate(args)?;
    let Some(hold) = &args.capacity_import_bootstrap else {
        return Ok(());
    };
    let store = args.authority.wasm_store_authority.wasm_store;
    let mut state = CanisterPoolStore::state();
    if state != CanisterPoolStateRecord::default()
        || !CanisterPoolStore::export().entries.is_empty()
    {
        return Err(InternalError::conflict());
    }
    state.bootstrap_import = Some(PoolImportBootstrapRecord {
        review_sha256: hold.review_sha256,
        install_id: args.install_id,
        operator: hold.operator,
        store,
        sources: hold.sources.clone(),
    });
    CanisterPoolStore::set_state(state);
    Ok(())
}

/// Return protected initialization authority without observing or assigning source canisters.
pub fn context() -> Option<PoolImportBootstrap> {
    CanisterPoolStore::state()
        .bootstrap_import
        .map(|hold| PoolImportBootstrap {
            review_sha256: hold.review_sha256,
            operator: hold.operator,
            sources: hold.sources,
        })
}

pub fn pending() -> bool {
    CanisterPoolStore::state().bootstrap_import.is_some()
}

/// The supplied Store can initialize while capacity stays fenced; an active import still excludes it.
pub fn require_store_initialization(store: Principal) -> Result<(), InternalError> {
    let wrong_store = CanisterPoolStore::state()
        .bootstrap_import
        .is_some_and(|hold| hold.store != store);
    if wrong_store || CanisterPoolImportOps::active_identity().is_some() {
        return Err(InternalError::conflict());
    }
    Ok(())
}

pub(super) fn require_reservation(
    state: &CanisterPoolStateRecord,
    request: &PoolImportReservation,
) -> Result<(), InternalError> {
    let Some(hold) = &state.bootstrap_import else {
        return Ok(());
    };
    let sources = request
        .sources
        .iter()
        .map(|source| source.canister_id)
        .collect::<Vec<_>>();
    if request.sequence != 0 || request.operator != hold.operator || sources != hold.sources {
        return Err(InternalError::conflict());
    }
    Ok(())
}
