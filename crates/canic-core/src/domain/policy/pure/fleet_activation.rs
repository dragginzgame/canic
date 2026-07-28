//! Module: domain::policy::pure::fleet_activation
//!
//! Responsibility: decide which exact managed endpoints may run while a Canister is Prepared.
//! Does not own: activation state reads, endpoint dispatch, caller authorization, or mutation.
//! Boundary: workflow supplies the current role and endpoint call after reading protected state.

use crate::{
    ids::{EndpointCall, EndpointCallKind},
    protocol::{
        CANIC_ACTIVATE_FLEET, CANIC_FLEET_ACTIVATION_STATUS, CANIC_FLEET_REGISTRY_ACTIVATE_MIRROR,
        CANIC_FLEET_REGISTRY_MIRROR_STATUS, CANIC_FLEET_REGISTRY_SYNC_STATUS,
        CANIC_FLEET_REGISTRY_SYNCHRONIZE, CANIC_FLEET_SUBNET_ROOT_AUTHORITY,
        CANIC_MANAGED_CANISTER_BINDING, CANIC_PREPARE_FLEET_ACTIVATION,
        CANIC_PREPARE_FLEET_CREDENTIAL_GENERATION, CANIC_RESUME_FLEET_ACTIVATION,
        CANIC_ROOT_COMPONENT_ALLOCATE, CANIC_ROOT_COMPONENT_ALLOCATION_STATUS,
        CANIC_ROOT_COMPONENT_CREATE, CANIC_ROOT_COMPONENT_INSTALL,
        CANIC_ROOT_COMPONENT_REGISTRY_PREPARE, CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
        CANIC_ROOT_STORE_BOOTSTRAP, CANIC_ROOT_STORE_BOOTSTRAP_STATUS, CANIC_SYNC_STATE,
        CANIC_SYNC_TOPOLOGY, CANIC_TEMPLATE_PREPARE_ADMIN, CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN,
        CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN, CANIC_WASM_STORE_CATALOG, CANIC_WASM_STORE_INFO,
        CANIC_WASM_STORE_PREPARE, CANIC_WASM_STORE_PUBLISH_CHUNK, CANIC_WASM_STORE_STAGE_MANIFEST,
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
) -> Result<(), FleetActivationEndpointPolicyError> {
    if is_status_query(call)
        || is_query(call, CANIC_MANAGED_CANISTER_BINDING)
        || is_query(call, CANIC_WASM_STORE_CATALOG)
        || is_query(call, CANIC_WASM_STORE_STATUS)
        || is_update(
            call,
            &[
                CANIC_SYNC_STATE,
                CANIC_SYNC_TOPOLOGY,
                CANIC_PREPARE_FLEET_CREDENTIAL_GENERATION,
                CANIC_ACTIVATE_FLEET,
                CANIC_WASM_STORE_INFO,
                CANIC_WASM_STORE_PREPARE,
                CANIC_WASM_STORE_PUBLISH_CHUNK,
                CANIC_WASM_STORE_STAGE_MANIFEST,
            ],
        )
    {
        return Ok(());
    }
    fenced(call)
}

/// Require one exact recovery endpoint admitted for a Prepared root.
pub fn require_prepared_root_endpoint(
    call: EndpointCall,
) -> Result<(), FleetActivationEndpointPolicyError> {
    if is_status_query(call)
        || is_query(call, CANIC_FLEET_SUBNET_ROOT_AUTHORITY)
        || is_query(call, CANIC_ROOT_COMPONENT_ALLOCATION_STATUS)
        || is_composite_query(call, CANIC_ROOT_STORE_BOOTSTRAP_STATUS)
        || is_composite_query(call, CANIC_FLEET_REGISTRY_SYNC_STATUS)
        || is_composite_query(call, CANIC_FLEET_REGISTRY_MIRROR_STATUS)
        || is_composite_query(call, CANIC_ROOT_COMPONENT_REGISTRY_STATUS)
        || is_update(
            call,
            &[
                CANIC_FLEET_REGISTRY_ACTIVATE_MIRROR,
                CANIC_FLEET_REGISTRY_SYNCHRONIZE,
                CANIC_PREPARE_FLEET_ACTIVATION,
                CANIC_RESUME_FLEET_ACTIVATION,
                CANIC_ROOT_COMPONENT_ALLOCATE,
                CANIC_ROOT_COMPONENT_CREATE,
                CANIC_ROOT_COMPONENT_INSTALL,
                CANIC_ROOT_COMPONENT_REGISTRY_PREPARE,
                CANIC_ROOT_STORE_BOOTSTRAP,
                CANIC_TEMPLATE_PREPARE_ADMIN,
                CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN,
                CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN,
            ],
        )
    {
        return Ok(());
    }
    fenced(call)
}

fn is_status_query(call: EndpointCall) -> bool {
    is_query(call, CANIC_FLEET_ACTIVATION_STATUS)
}

fn is_query(call: EndpointCall, endpoint: &str) -> bool {
    call.kind == EndpointCallKind::Query && call.endpoint.name == endpoint
}

fn is_composite_query(call: EndpointCall, endpoint: &str) -> bool {
    call.kind == EndpointCallKind::QueryComposite && call.endpoint.name == endpoint
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
    fn prepared_root_admits_exact_staging_and_activation_updates() {
        for (endpoint, kind) in [
            (CANIC_FLEET_ACTIVATION_STATUS, EndpointCallKind::Query),
            (CANIC_FLEET_SUBNET_ROOT_AUTHORITY, EndpointCallKind::Query),
            (CANIC_FLEET_REGISTRY_SYNCHRONIZE, EndpointCallKind::Update),
            (
                CANIC_FLEET_REGISTRY_ACTIVATE_MIRROR,
                EndpointCallKind::Update,
            ),
            (
                CANIC_FLEET_REGISTRY_SYNC_STATUS,
                EndpointCallKind::QueryComposite,
            ),
            (
                CANIC_FLEET_REGISTRY_MIRROR_STATUS,
                EndpointCallKind::QueryComposite,
            ),
            (
                CANIC_ROOT_COMPONENT_REGISTRY_PREPARE,
                EndpointCallKind::Update,
            ),
            (
                CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
                EndpointCallKind::QueryComposite,
            ),
            (CANIC_ROOT_COMPONENT_ALLOCATE, EndpointCallKind::Update),
            (CANIC_ROOT_COMPONENT_CREATE, EndpointCallKind::Update),
            (CANIC_ROOT_COMPONENT_INSTALL, EndpointCallKind::Update),
            (
                CANIC_ROOT_COMPONENT_ALLOCATION_STATUS,
                EndpointCallKind::Query,
            ),
            (CANIC_PREPARE_FLEET_ACTIVATION, EndpointCallKind::Update),
            (CANIC_RESUME_FLEET_ACTIVATION, EndpointCallKind::Update),
            (CANIC_ROOT_STORE_BOOTSTRAP, EndpointCallKind::Update),
            (
                CANIC_ROOT_STORE_BOOTSTRAP_STATUS,
                EndpointCallKind::QueryComposite,
            ),
            (CANIC_TEMPLATE_PREPARE_ADMIN, EndpointCallKind::Update),
            (CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN, EndpointCallKind::Update),
            (
                CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN,
                EndpointCallKind::Update,
            ),
        ] {
            assert_eq!(require_prepared_root_endpoint(call(endpoint, kind)), Ok(()));
        }
    }

    #[test]
    fn prepared_root_rejects_ordinary_and_wrong_kind_calls() {
        for (endpoint, kind) in [
            ("application_update", EndpointCallKind::Update),
            (CANIC_SYNC_STATE, EndpointCallKind::Update),
            ("canic_upsert_root_issuer_policy", EndpointCallKind::Update),
            (CANIC_FLEET_ACTIVATION_STATUS, EndpointCallKind::Update),
            (CANIC_FLEET_SUBNET_ROOT_AUTHORITY, EndpointCallKind::Update),
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

    #[test]
    fn prepared_nonroot_uses_its_own_exact_recovery_allowlist() {
        for (endpoint, kind) in [
            (CANIC_FLEET_ACTIVATION_STATUS, EndpointCallKind::Query),
            (CANIC_MANAGED_CANISTER_BINDING, EndpointCallKind::Query),
            (CANIC_WASM_STORE_CATALOG, EndpointCallKind::Query),
            (CANIC_WASM_STORE_STATUS, EndpointCallKind::Query),
            (CANIC_SYNC_STATE, EndpointCallKind::Update),
            (CANIC_SYNC_TOPOLOGY, EndpointCallKind::Update),
            (
                CANIC_PREPARE_FLEET_CREDENTIAL_GENERATION,
                EndpointCallKind::Update,
            ),
            (CANIC_ACTIVATE_FLEET, EndpointCallKind::Update),
            (CANIC_WASM_STORE_INFO, EndpointCallKind::Update),
            (CANIC_WASM_STORE_PREPARE, EndpointCallKind::Update),
            (CANIC_WASM_STORE_PUBLISH_CHUNK, EndpointCallKind::Update),
            (CANIC_WASM_STORE_STAGE_MANIFEST, EndpointCallKind::Update),
        ] {
            assert_eq!(
                require_prepared_nonroot_endpoint(call(endpoint, kind)),
                Ok(())
            );
        }

        for (endpoint, kind) in [
            ("application_update", EndpointCallKind::Update),
            (
                "canic_active_delegation_proof_status",
                EndpointCallKind::Query,
            ),
            (CANIC_FLEET_ACTIVATION_STATUS, EndpointCallKind::Update),
        ] {
            assert_eq!(
                require_prepared_nonroot_endpoint(call(endpoint, kind)),
                Err(FleetActivationEndpointPolicyError::Fenced { endpoint, kind })
            );
        }
    }
}
