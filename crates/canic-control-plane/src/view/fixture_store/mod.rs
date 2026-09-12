//! Read-only fixture grant authority projected from the Registry identity.

use candid::Principal;
#[cfg(feature = "root-control-plane")]
use canic_core::dto::fixture_provisioning::FixtureTargetBinding;
#[cfg(feature = "wasm-store-canister")]
use canic_core::ids::ComponentBinding;

/// Existing Component authority and the exact Component or child caller.
#[cfg(feature = "wasm-store-canister")]
pub struct FixtureTargetAuthority<'a> {
    pub component: &'a ComponentBinding,
    pub target: Principal,
    pub parent: Principal,
}

/// Immutable source access selected for one installed target; owns no progress cursor.
#[cfg(feature = "root-control-plane")]
#[derive(Clone, Debug)]
pub struct FixtureInstallation {
    pub store: Principal,
    pub binding: FixtureTargetBinding,
    pub revision: u64,
}

/// Whether installation is selecting source access or replaying its retained intent.
#[cfg(feature = "root-control-plane")]
pub enum FixtureGrantSelection {
    Fresh,
    Retained(Option<u64>),
}
