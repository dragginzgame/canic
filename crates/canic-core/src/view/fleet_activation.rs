//! Module: view::fleet_activation
//!
//! Responsibility: carry one internal Fleet-activation transition result across layers.
//! Does not own: activation mutation, runtime startup, or endpoint serialization.
//! Boundary: storage ops report whether one exact transition committed; workflows consume it once.

use crate::cdk::types::Principal;
use crate::dto::{
    component_registry::ComponentRuntimeStatusResponse,
    fleet_activation::FleetActivationStatusResponse,
};

/// The exact root-owned Wasm Store included in fresh Fleet activation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetActivationWasmStoreView {
    pub pid: Principal,
}

///
/// FleetActivationTransition
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetActivationTransition {
    pub status: FleetActivationStatusResponse,
    pub transitioned: bool,
    pub application_init_args: Option<Vec<u8>>,
}

///
/// ComponentRuntimeActivationTransition
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentRuntimeActivationTransition {
    pub status: ComponentRuntimeStatusResponse,
    pub transitioned: bool,
    pub application_init_args: Option<Vec<u8>>,
}
