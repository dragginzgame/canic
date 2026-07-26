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

/// Return the implicit store policy used by the Fleet Subnet Root.
#[cfg(feature = "root-control-plane")]
#[must_use]
pub fn fleet_subnet_root_default_wasm_store() -> WasmStoreConfig {
    WasmStoreConfig::implicit()
}

/// Return every configured Component and Component Child release role.
#[cfg(feature = "root-control-plane")]
pub fn fleet_subnet_root_managed_release_roles() -> Result<BTreeSet<CanisterRole>, InternalError> {
    let mut roles = BTreeSet::new();

    for component_spec in ConfigOps::component_topology()?.component_specs {
        roles.insert(component_spec.component_role);
        roles.extend(component_spec.children.into_iter().map(|child| child.role));
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
