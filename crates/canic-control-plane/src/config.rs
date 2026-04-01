use crate::{ids::CanisterRole, schema::WasmStoreConfig};
use canic_core::{
    __control_plane_core as cp_core,
    error::{InternalError, InternalErrorOrigin},
};
use cp_core::ops::runtime::env::EnvOps;

/// Return the implicit store policy used by the current subnet.
#[must_use]
pub fn current_subnet_default_wasm_store() -> WasmStoreConfig {
    WasmStoreConfig::implicit()
}

/// Resolve the local store policy for the current canister.
pub fn current_wasm_store() -> Result<WasmStoreConfig, InternalError> {
    let canister_role = EnvOps::canister_role()?;

    if canister_role == CanisterRole::WASM_STORE {
        Ok(WasmStoreConfig::implicit())
    } else {
        Err(InternalError::ops(
            InternalErrorOrigin::Ops,
            format!("current canister '{canister_role}' is not configured as a wasm store"),
        ))
    }
}
