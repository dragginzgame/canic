//! Module: domain::policy::pure::fleet_activation
//!
//! Responsibility: decide which exact managed endpoints may run while a Canister is Prepared.
//! Does not own: activation state reads, endpoint dispatch, caller authorization, or mutation.
//! Boundary: workflow supplies the current role and endpoint call after reading protected state.

use crate::{
    ids::{EndpointCall, EndpointCallKind},
    protocol::CANIC_FLEET_ACTIVATION_STATUS,
};
use thiserror::Error as ThisError;

///
/// FleetActivationEndpointPolicyError
///

#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum FleetActivationEndpointPolicyError {
    #[error("endpoint {endpoint} ({kind:?}) is fenced while the root Canister is Prepared")]
    Fenced {
        endpoint: &'static str,
        kind: EndpointCallKind,
    },
}

/// Require one exact recovery endpoint admitted for a Prepared root.
pub fn require_prepared_root_endpoint(
    call: EndpointCall,
) -> Result<(), FleetActivationEndpointPolicyError> {
    if call.kind == EndpointCallKind::Query && call.endpoint.name == CANIC_FLEET_ACTIVATION_STATUS {
        return Ok(());
    }

    Err(FleetActivationEndpointPolicyError::Fenced {
        endpoint: call.endpoint.name,
        kind: call.kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::EndpointId;

    fn call(name: &'static str, kind: EndpointCallKind) -> EndpointCall {
        EndpointCall {
            endpoint: EndpointId::new(name),
            kind,
        }
    }

    #[test]
    fn prepared_root_admits_only_implemented_recovery_inspection() {
        assert_eq!(
            require_prepared_root_endpoint(call(
                CANIC_FLEET_ACTIVATION_STATUS,
                EndpointCallKind::Query,
            )),
            Ok(())
        );
    }

    #[test]
    fn prepared_root_rejects_ordinary_and_wrong_kind_calls() {
        for (endpoint, kind) in [
            ("application_update", EndpointCallKind::Update),
            ("canic_sync_state", EndpointCallKind::Update),
            ("canic_upsert_root_issuer_policy", EndpointCallKind::Update),
            (CANIC_FLEET_ACTIVATION_STATUS, EndpointCallKind::Update),
            (
                CANIC_FLEET_ACTIVATION_STATUS,
                EndpointCallKind::QueryComposite,
            ),
        ] {
            assert_eq!(
                require_prepared_root_endpoint(call(endpoint, kind)),
                Err(FleetActivationEndpointPolicyError::Fenced { endpoint, kind })
            );
        }
    }
}
