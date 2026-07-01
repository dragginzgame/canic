use crate::{ids::CanisterRole, schema::WasmStoreConfig};
use canic_core::control_plane_support::error::InternalError;
#[cfg(feature = "wasm-store-canister")]
use canic_core::control_plane_support::error::InternalErrorOrigin;
#[cfg(feature = "root-control-plane")]
use canic_core::control_plane_support::ops::config::ConfigOps;
#[cfg(feature = "wasm-store-canister")]
use canic_core::control_plane_support::ops::runtime::env::EnvOps;
#[cfg(feature = "root-control-plane")]
use std::collections::BTreeSet;

/// Return the implicit store policy used by the current subnet.
#[cfg(feature = "root-control-plane")]
#[must_use]
pub fn current_subnet_default_wasm_store() -> WasmStoreConfig {
    WasmStoreConfig::implicit()
}

/// Return the configured managed release roles for the current subnet.
#[cfg(feature = "root-control-plane")]
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
#[cfg(feature = "wasm-store-canister")]
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
