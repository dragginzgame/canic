//! Module: access::deployment
//!
//! Responsibility: gate application endpoints by protected Component deployment purpose.
//! Does not own: caller authentication, service topology, or application write semantics.
//! Boundary: only an active Directory-validated Authority purpose satisfies the write guard.

use crate::{InternalError, access::AccessError};
use canic_contracts::ids::FleetServiceId;

/// Require this Component tree to hold one exact Fleet service's write Authority purpose.

pub fn require_service_authority(service: &str) -> Result<(), AccessError> {
    let service = FleetServiceId::try_from(service.to_owned())
        .map_err(|_| AccessError::ServiceGuardInvalid)?;
    service_authority_access_result(
        crate::workflow::component_runtime::service_authority_matches(&service),
        service,
    )
}

fn service_authority_access_result(
    result: Result<bool, InternalError>,
    service: FleetServiceId,
) -> Result<(), AccessError> {
    match result {
        Ok(true) => Ok(()),
        Ok(false) => Err(AccessError::ServiceAuthorityRequired { service }),
        Err(error) => Err(AccessError::Internal(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_authority_access_distinguishes_denial_from_runtime_failure() {
        let service: FleetServiceId = "database".parse().unwrap();
        assert!(service_authority_access_result(Ok(true), service.clone()).is_ok());
        let Err(AccessError::ServiceAuthorityRequired {
            service: denied_service,
        }) = service_authority_access_result(Ok(false), service.clone())
        else {
            panic!("service authority denial must retain the required service");
        };
        assert_eq!(denied_service, service);

        let error = InternalError::state_failure();
        let Err(AccessError::Internal(error)) =
            service_authority_access_result(Err(error), service)
        else {
            panic!("runtime failure must remain typed");
        };
        assert_eq!(error.code(), crate::diagnostics::codes::STATE_FAILED);
    }
}
