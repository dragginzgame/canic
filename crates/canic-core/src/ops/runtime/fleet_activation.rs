//! Module: ops::runtime::fleet_activation
//!
//! Responsibility: expose the explicit managed-versus-standalone endpoint mode to lifecycle workflows.
//! Does not own: stable Fleet activation state, endpoint policy, or lifecycle orchestration.
//! Boundary: managed is the fail-closed default; only `start_local!` selects standalone-local mode.

use crate::model::fleet_activation::endpoint_mode::{FleetEndpointMode, FleetEndpointModeState};

///
/// FleetActivationRuntimeOps
///

pub struct FleetActivationRuntimeOps;

impl FleetActivationRuntimeOps {
    pub(crate) fn set_managed() {
        FleetEndpointModeState::set(FleetEndpointMode::Managed);
    }

    pub(crate) fn set_standalone_local() {
        FleetEndpointModeState::set(FleetEndpointMode::StandaloneLocal);
    }

    #[must_use]
    pub(crate) fn is_standalone_local() -> bool {
        FleetEndpointModeState::get() == FleetEndpointMode::StandaloneLocal
    }
}
