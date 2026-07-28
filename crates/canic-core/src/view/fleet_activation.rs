//! Module: view::fleet_activation
//!
//! Responsibility: carry one internal Fleet-activation transition result across layers.
//! Does not own: activation mutation, runtime startup, or endpoint serialization.
//! Boundary: storage ops report whether one exact transition committed; workflows consume it once.

use crate::dto::{
    component_registry::ComponentRuntimeStatusResponse,
    fleet_activation::FleetActivationStatusResponse,
};

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
