//! Module: domain::policy::pure::fleet_activation
//!
//! Responsibility: decide which exact managed endpoints may run while a Canister is Prepared.
//! Does not own: activation state reads, endpoint dispatch, caller authorization, or mutation.
//! Boundary: workflow supplies the current role and endpoint call after reading protected state.

use crate::{
    ids::{EndpointCall, EndpointCallKind},
    protocol::{CANIC_COMMAND, CANIC_STATUS},
};
use thiserror::Error as ThisError;

///
/// FleetActivationEndpointPolicyError
///

#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum FleetActivationEndpointPolicyError {
    #[error("endpoint {endpoint} ({kind:?}) is fenced while the managed Canister is Prepared")]
    Fenced {
        endpoint: &'static str,
        kind: EndpointCallKind,
    },
}

/// Require one exact recovery endpoint admitted for a Prepared non-root.
pub fn require_prepared_nonroot_endpoint(
    call: EndpointCall,
) -> Result<(), FleetActivationEndpointPolicyError> {
    if is_query(call, CANIC_STATUS) || is_update(call, &[CANIC_COMMAND]) {
        return Ok(());
    }
    fenced(call)
}

/// Require one compile-selected Store data lane while that Store is Prepared.
pub fn require_prepared_store_data_endpoint(
    call: EndpointCall,
) -> Result<(), FleetActivationEndpointPolicyError> {
    if call.kind == EndpointCallKind::Update {
        return Ok(());
    }
    fenced(call)
}

/// Require one exact recovery endpoint admitted for a Prepared root.
pub fn require_prepared_root_endpoint(
    call: EndpointCall,
) -> Result<(), FleetActivationEndpointPolicyError> {
    if is_query(call, CANIC_STATUS) || is_update(call, &[CANIC_COMMAND]) {
        return Ok(());
    }
    fenced(call)
}

fn is_query(call: EndpointCall, endpoint: &str) -> bool {
    call.kind == EndpointCallKind::Query && call.endpoint.name == endpoint
}

fn is_update(call: EndpointCall, endpoints: &[&str]) -> bool {
    call.kind == EndpointCallKind::Update && endpoints.contains(&call.endpoint.name)
}

const fn fenced(call: EndpointCall) -> Result<(), FleetActivationEndpointPolicyError> {
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
    fn prepared_root_admits_only_the_role_owned_entrypoints() {
        for (endpoint, kind) in [
            (CANIC_COMMAND, EndpointCallKind::Update),
            (CANIC_STATUS, EndpointCallKind::Query),
        ] {
            assert_eq!(require_prepared_root_endpoint(call(endpoint, kind)), Ok(()));
        }
    }

    #[test]
    fn prepared_root_rejects_ordinary_and_wrong_kind_calls() {
        for (endpoint, kind) in [
            ("application_update", EndpointCallKind::Update),
            (CANIC_COMMAND, EndpointCallKind::Query),
            (CANIC_STATUS, EndpointCallKind::Update),
            (CANIC_STATUS, EndpointCallKind::QueryComposite),
        ] {
            assert_eq!(
                require_prepared_root_endpoint(call(endpoint, kind)),
                Err(FleetActivationEndpointPolicyError::Fenced { endpoint, kind })
            );
        }
    }

    #[test]
    fn prepared_nonroot_uses_the_ordinary_role_recovery_allowlist() {
        for (endpoint, kind) in [
            (CANIC_STATUS, EndpointCallKind::Query),
            (CANIC_COMMAND, EndpointCallKind::Update),
        ] {
            assert_eq!(
                require_prepared_nonroot_endpoint(call(endpoint, kind)),
                Ok(())
            );
        }

        for (endpoint, kind) in [
            ("application_update", EndpointCallKind::Update),
            ("application_query", EndpointCallKind::Query),
            (CANIC_STATUS, EndpointCallKind::Update),
            (CANIC_COMMAND, EndpointCallKind::Query),
        ] {
            assert_eq!(
                require_prepared_nonroot_endpoint(call(endpoint, kind)),
                Err(FleetActivationEndpointPolicyError::Fenced { endpoint, kind })
            );
        }
    }

    #[test]
    fn prepared_store_data_policy_is_reachable_only_through_compile_selected_updates() {
        assert_eq!(
            require_prepared_store_data_endpoint(call("store_data_lane", EndpointCallKind::Update)),
            Ok(())
        );
        assert!(
            require_prepared_store_data_endpoint(call("store_data_lane", EndpointCallKind::Query))
                .is_err()
        );
    }
}
