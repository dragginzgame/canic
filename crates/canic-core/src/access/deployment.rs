//! Module: access::deployment
//!
//! Responsibility: gate application endpoints by protected Component deployment purpose.
//! Does not own: caller authentication, service topology, or application write semantics.
//! Boundary: only an active Directory-validated Authority purpose satisfies the write guard.

use crate::{access::AccessError, ids::FleetServiceId};

/// Require this Component tree to hold one exact Fleet service's write Authority purpose.
pub fn require_service_authority(service: &str) -> Result<(), AccessError> {
    let service = FleetServiceId::try_from(service.to_owned()).map_err(|error| {
        AccessError::Denied(format!("invalid Fleet service Authority guard: {error}"))
    })?;
    crate::workflow::component_runtime::require_service_authority(&service)
        .map_err(|error| AccessError::Denied(error.to_string()))
}
