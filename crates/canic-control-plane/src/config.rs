use crate::{
    ids::{CanisterRole, WasmStoreBinding},
    ops::storage::state::subnet::SubnetStateOps,
    schema::WasmStoreConfig,
};
use canic_core::{
    __control_plane_core as cp_core,
    error::{InternalError, InternalErrorOrigin},
};
use cp_core::ops::runtime::env::EnvOps;

///
/// Resolve the default publication binding for the current subnet.
///

#[must_use]
pub fn current_subnet_default_wasm_store_binding() -> WasmStoreBinding {
    SubnetStateOps::publication_store_binding()
        .filter(|binding| SubnetStateOps::wasm_store_pid(binding).is_some())
        .or_else(|| {
            SubnetStateOps::wasm_stores()
                .into_iter()
                .min_by(|left, right| left.created_at.cmp(&right.created_at))
                .map(|record| record.binding)
        })
        .unwrap_or_else(|| WasmStoreBinding::new("primary"))
}

/// Return the implicit store policy used by the current subnet.
#[must_use]
pub const fn current_subnet_default_wasm_store() -> WasmStoreConfig {
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
