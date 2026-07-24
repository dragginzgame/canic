//! Module: view::fleet_activation
//!
//! Responsibility: carry one internal Fleet-activation transition result across layers.
//! Does not own: activation mutation, runtime startup, or endpoint serialization.
//! Boundary: storage ops report whether one exact transition committed; workflows consume it once.

use crate::dto::fleet_activation::FleetActivationStatusResponse;

///
/// FleetActivationTransition
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetActivationTransition {
    pub status: FleetActivationStatusResponse,
    pub transitioned: bool,
}
