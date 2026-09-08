//! Module: domain::policy::pure::fleet_activation
//!
//! Responsibility: decide which exact managed endpoints may run while a Canister is Prepared.
//! Does not own: activation state reads, endpoint dispatch, caller authorization, or mutation.
//! Boundary: workflow supplies the current role and endpoint call after reading protected state.

use crate::{
    ids::{EndpointCall, EndpointCallKind},
    protocol::{
        CANIC_ADMISSION_STATUS, CANIC_AUTH_STATUS, CANIC_COMMAND, CANIC_CONTROL_STATUS,
        CANIC_OBSERVABILITY, CANIC_PUBLIC_STATUS, CANIC_ROOT_COMMAND, CANIC_ROOT_OPERATION_STATUS,
        CANIC_ROOT_STATUS, CANIC_WASM_STORE_CATALOG, CANIC_WASM_STORE_COMMAND,
        CANIC_WASM_STORE_STATUS,
    },
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
    is_wasm_store: bool,
) -> Result<(), FleetActivationEndpointPolicyError> {
    let reads: &[&str] = if is_wasm_store {
        &[
            CANIC_PUBLIC_STATUS,
            CANIC_OBSERVABILITY,
            CANIC_WASM_STORE_STATUS,
            CANIC_WASM_STORE_CATALOG,
        ]
    } else {
        &[
            CANIC_PUBLIC_STATUS,
            CANIC_OBSERVABILITY,
            CANIC_AUTH_STATUS,
            CANIC_ADMISSION_STATUS,
            CANIC_CONTROL_STATUS,
        ]
    };
    let command = if is_wasm_store {
        CANIC_WASM_STORE_COMMAND
    } else {
        CANIC_COMMAND
    };
    if is_query(call, reads) || is_update(call, &[command]) {
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
    if is_query(
        call,
        &[
            CANIC_PUBLIC_STATUS,
            CANIC_OBSERVABILITY,
            CANIC_ROOT_OPERATION_STATUS,
            CANIC_ROOT_STATUS,
        ],
    ) || is_update(call, &[CANIC_ROOT_COMMAND])
    {
        return Ok(());
    }
    fenced(call)
}

fn is_query(call: EndpointCall, endpoints: &[&str]) -> bool {
    call.kind == EndpointCallKind::Query && endpoints.contains(&call.endpoint.name)
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
            (CANIC_ROOT_COMMAND, EndpointCallKind::Update),
            (CANIC_ROOT_STATUS, EndpointCallKind::Query),
            (CANIC_ROOT_OPERATION_STATUS, EndpointCallKind::Query),
            (CANIC_PUBLIC_STATUS, EndpointCallKind::Query),
            (CANIC_OBSERVABILITY, EndpointCallKind::Query),
        ] {
            assert_eq!(require_prepared_root_endpoint(call(endpoint, kind)), Ok(()));
        }
    }

    #[test]
    fn prepared_root_rejects_ordinary_and_wrong_kind_calls() {
        for (endpoint, kind) in [
            ("application_update", EndpointCallKind::Update),
            (CANIC_ROOT_COMMAND, EndpointCallKind::Query),
            (CANIC_ROOT_STATUS, EndpointCallKind::Update),
            (CANIC_ROOT_STATUS, EndpointCallKind::QueryComposite),
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
            (CANIC_CONTROL_STATUS, EndpointCallKind::Query),
            (CANIC_AUTH_STATUS, EndpointCallKind::Query),
            (CANIC_ADMISSION_STATUS, EndpointCallKind::Query),
            (CANIC_PUBLIC_STATUS, EndpointCallKind::Query),
            (CANIC_OBSERVABILITY, EndpointCallKind::Query),
            (CANIC_COMMAND, EndpointCallKind::Update),
        ] {
            assert_eq!(
                require_prepared_nonroot_endpoint(call(endpoint, kind), false),
                Ok(())
            );
        }

        for (endpoint, kind) in [
            ("application_update", EndpointCallKind::Update),
            ("application_query", EndpointCallKind::Query),
            (CANIC_WASM_STORE_CATALOG, EndpointCallKind::Query),
            (CANIC_CONTROL_STATUS, EndpointCallKind::Update),
            (CANIC_COMMAND, EndpointCallKind::Query),
        ] {
            assert_eq!(
                require_prepared_nonroot_endpoint(call(endpoint, kind), false),
                Err(FleetActivationEndpointPolicyError::Fenced { endpoint, kind })
            );
        }
    }

    #[test]
    fn prepared_store_uses_only_its_role_owned_commands_and_reads() {
        for (endpoint, kind) in [
            (CANIC_WASM_STORE_STATUS, EndpointCallKind::Query),
            (CANIC_WASM_STORE_CATALOG, EndpointCallKind::Query),
            (CANIC_PUBLIC_STATUS, EndpointCallKind::Query),
            (CANIC_OBSERVABILITY, EndpointCallKind::Query),
            (CANIC_WASM_STORE_COMMAND, EndpointCallKind::Update),
        ] {
            assert_eq!(
                require_prepared_nonroot_endpoint(call(endpoint, kind), true),
                Ok(())
            );
        }
        for (endpoint, kind) in [
            (CANIC_CONTROL_STATUS, EndpointCallKind::Query),
            (CANIC_COMMAND, EndpointCallKind::Update),
            (CANIC_WASM_STORE_STATUS, EndpointCallKind::Update),
            (CANIC_WASM_STORE_CATALOG, EndpointCallKind::Update),
            (CANIC_AUTH_STATUS, EndpointCallKind::Query),
            (CANIC_ADMISSION_STATUS, EndpointCallKind::Query),
            (CANIC_WASM_STORE_COMMAND, EndpointCallKind::Query),
        ] {
            assert_eq!(
                require_prepared_nonroot_endpoint(call(endpoint, kind), true),
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
