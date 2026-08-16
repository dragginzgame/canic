//! Module: access::deployment
//!
//! Responsibility: gate application endpoints by protected Component deployment purpose.
//! Does not own: caller authentication, service topology, or application write semantics.
//! Boundary: only an active Directory-validated Authority purpose satisfies the write guard.

use crate::{InternalError, access::AccessError, ids::FleetServiceId};

/// Require this Component tree to hold one exact Fleet service's write Authority purpose.
pub fn require_service_authority(service: &str) -> Result<(), AccessError> {
    let service = FleetServiceId::try_from(service.to_owned())
        .map_err(|_| AccessError::ServiceGuardInvalid)?;
    service_authority_access_result(
        crate::workflow::component_runtime::service_authority_matches(&service),
    )
}

const fn service_authority_access_result(
    result: Result<bool, InternalError>,
) -> Result<(), AccessError> {
    match result {
        Ok(true) => Ok(()),
        Ok(false) => Err(AccessError::ServiceAuthorityRequired),
        Err(error) => Err(AccessError::Internal(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_authority_access_distinguishes_denial_from_runtime_failure() {
        assert!(service_authority_access_result(Ok(true)).is_ok());
        assert!(matches!(
            service_authority_access_result(Ok(false)),
            Err(AccessError::ServiceAuthorityRequired)
        ));

        let error = InternalError::state_failure();
        let Err(AccessError::Internal(error)) = service_authority_access_result(Err(error)) else {
            panic!("runtime failure must remain typed");
        };
        assert_eq!(error.code(), crate::diagnostics::codes::STATE_FAILED);
    }
}
