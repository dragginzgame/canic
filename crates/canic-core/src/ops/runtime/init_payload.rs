//! Module: ops::runtime::init_payload
//!
//! Responsibility: construct typed initialization payloads for Canic infrastructure.
//! Does not own: installation orchestration, artifact selection, or Component lifecycle state.
//! Boundary: converts current runtime authority into passive boundary data.

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::fleet_subnet_root::FleetSubnetWasmStoreInitArgs,
    ops::{
        runtime::env::EnvOps,
        storage::{StorageOpsError, fleet_activation::FleetActivationOps},
    },
};

/// Build the infrastructure authority payload for one Store owned by the current root.
pub fn wasm_store_init_args(
    target_pid: Principal,
) -> Result<FleetSubnetWasmStoreInitArgs, InternalError> {
    if target_pid == Principal::anonymous() {
        return Err(InternalError::invalid_input());
    }

    EnvOps::require_root()?;
    let authority = FleetActivationOps::wasm_store_authority().map_err(StorageOpsError::from)?;
    if authority.wasm_store != target_pid {
        return Err(InternalError::invalid_input());
    }
    let identity = FleetActivationOps::status(EnvOps::is_root())
        .map_err(StorageOpsError::from)?
        .identity;

    Ok(FleetSubnetWasmStoreInitArgs {
        authority,
        install_id: identity.operation_id,
    })
}
