//! Module: ops::runtime::init_payload
//!
//! Responsibility: construct typed initialization payloads for Canic infrastructure.
//! Does not own: installation orchestration, artifact selection, or Component lifecycle state.
//! Boundary: converts current runtime authority into passive boundary data.

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::{
        abi::v1::{CanisterInitAuthority, CanisterInitPayload},
        env::EnvBootstrapArgs,
    },
    ids::CanisterRole,
    ops::{
        runtime::env::EnvOps,
        storage::{StorageOpsError, fleet_activation::FleetActivationOps},
    },
};

/// Build the infrastructure authority payload for one Store owned by the current root.
pub fn wasm_store_init_payload(
    target_pid: Principal,
) -> Result<CanisterInitPayload, InternalError> {
    if target_pid == Principal::anonymous() {
        return Err(InternalError::invalid_input(
            "managed infrastructure target Canister is anonymous",
        ));
    }

    EnvOps::require_root()?;
    let root_pid = EnvOps::root_pid()?;
    let env = EnvBootstrapArgs {
        fleet_subnet_root_pid: Some(EnvOps::fleet_subnet_root_pid()?),
        component_spec: None,
        subnet_pid: Some(EnvOps::subnet_pid()?),
        root_pid: Some(root_pid),
        canister_role: Some(CanisterRole::WASM_STORE),
        parent_pid: Some(root_pid),
    };
    let identity = FleetActivationOps::status(EnvOps::is_root())
        .map_err(StorageOpsError::from)?
        .identity;

    Ok(CanisterInitPayload {
        install_id: identity.operation_id,
        release_build_id: identity.release_build_id,
        authority: CanisterInitAuthority::Infrastructure {
            fleet: identity.fleet,
            env,
        },
    })
}
