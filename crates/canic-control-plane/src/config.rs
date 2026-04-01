use crate::{ids::CanisterRole, schema::WasmStoreConfig};
use canic_core::{
    __control_plane_core as cp_core,
    error::{InternalError, InternalErrorOrigin},
};
use cp_core::ops::{config::ConfigOps, runtime::env::EnvOps};
use std::collections::BTreeSet;

/// Return the implicit store policy used by the current subnet.
#[must_use]
pub fn current_subnet_default_wasm_store() -> WasmStoreConfig {
    WasmStoreConfig::implicit()
}

/// Return the configured managed release roles for the current subnet.
pub fn current_subnet_managed_release_roles() -> Result<BTreeSet<CanisterRole>, InternalError> {
    let subnet = ConfigOps::current_subnet()?;
    let mut roles = BTreeSet::new();

    for role in subnet.canisters.keys() {
        if role == &CanisterRole::ROOT || role == &CanisterRole::WASM_STORE {
            continue;
        }

        roles.insert(role.clone());
    }

    Ok(roles)
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
