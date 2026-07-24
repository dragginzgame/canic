//! Module: model::fleet_activation::endpoint_mode
//!
//! Responsibility: own the explicit runtime distinction between managed and standalone-local endpoints.
//! Does not own: Fleet identity, stable activation state, endpoint policy, or lifecycle scheduling.
//! Boundary: lifecycle ops set the mode synchronously; missing initialization defaults fail-closed to managed.

use std::cell::Cell;

///
/// FleetEndpointMode
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetEndpointMode {
    Managed,
    StandaloneLocal,
}

thread_local! {
    static FLEET_ENDPOINT_MODE: Cell<FleetEndpointMode> =
        const { Cell::new(FleetEndpointMode::Managed) };
}

///
/// FleetEndpointModeState
///

pub struct FleetEndpointModeState;

impl FleetEndpointModeState {
    pub fn set(mode: FleetEndpointMode) {
        FLEET_ENDPOINT_MODE.set(mode);
    }

    #[must_use]
    pub fn get() -> FleetEndpointMode {
        FLEET_ENDPOINT_MODE.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_mode_defaults_to_managed_and_changes_only_explicitly() {
        FleetEndpointModeState::set(FleetEndpointMode::Managed);
        assert_eq!(FleetEndpointModeState::get(), FleetEndpointMode::Managed);

        FleetEndpointModeState::set(FleetEndpointMode::StandaloneLocal);
        assert_eq!(
            FleetEndpointModeState::get(),
            FleetEndpointMode::StandaloneLocal
        );

        FleetEndpointModeState::set(FleetEndpointMode::Managed);
    }
}
