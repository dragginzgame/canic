//! Module: domain::policy::pure::fleet_activation
//!
//! Responsibility: decide which exact managed endpoints may run while a Canister is Prepared.
//! Does not own: activation state reads, endpoint dispatch, caller authorization, or mutation.
//! Boundary: workflow supplies the current role and endpoint call after reading protected state.

use crate::protocol::{
    CANIC_ADMISSION_STATUS, CANIC_PUBLIC_CHILDREN, CANIC_PUBLIC_HEALTH, CANIC_PUBLIC_HISTORY,
    CANIC_PUBLIC_METRICS, CANIC_PUBLIC_OVERVIEW, CANIC_ROOT_FIXTURE_STATUS, CANIC_WASM_STORE_CHUNK,
    CANIC_WASM_STORE_FIXTURE_CHUNK, CANIC_WASM_STORE_PUBLISH_CHUNK,
    CANIC_WASM_STORE_PUBLISH_FIXTURE,
};
use canic_contracts::ids::{EndpointCall, EndpointCallKind};
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
            CANIC_PUBLIC_HEALTH,
            CANIC_PUBLIC_METRICS,
            CANIC_PUBLIC_HISTORY,
            CANIC_PUBLIC_OVERVIEW,
            crate::protocol::cycle_balance::METHOD,
            crate::protocol::cycle_history::METHOD,
            crate::protocol::store_status::authority::METHOD,
            crate::protocol::store_status::activation::METHOD,
            crate::protocol::store_status::gc::METHOD,
            crate::protocol::store_catalog::fixture::METHOD,
            crate::protocol::store_catalog::grant::METHOD,
            crate::protocol::store_catalog::template::METHOD,
            crate::protocol::store_inventory::catalog::METHOD,
            crate::protocol::store_inventory::storage::METHOD,
        ]
    } else {
        &[
            CANIC_PUBLIC_HEALTH,
            CANIC_PUBLIC_METRICS,
            CANIC_PUBLIC_HISTORY,
            CANIC_PUBLIC_OVERVIEW,
            crate::protocol::cycle_balance::METHOD,
            crate::protocol::cycle_history::METHOD,
            crate::protocol::protected_logs::METHOD,
            crate::protocol::protected_metrics::METHOD,
            crate::protocol::protected_memory_allocations::METHOD,
            crate::protocol::child_funding::METHOD,
            crate::protocol::protected_health::METHOD,
            crate::protocol::cycle_topups::METHOD,
            CANIC_PUBLIC_CHILDREN,
            crate::protocol::protected_readiness::METHOD,
            crate::protocol::protected_runtime::METHOD,
            crate::protocol::protected_binding::METHOD,
            crate::protocol::issuer_proof::status::METHOD,
            CANIC_ADMISSION_STATUS,
            crate::protocol::caller_authority_status::METHOD,
            crate::protocol::runtime_reporting::METHOD,
        ]
    };
    let store_data = is_wasm_store
        && is_update(
            call,
            &[
                crate::protocol::store_control::grant::METHOD,
                crate::protocol::store_control::reclaim::METHOD,
                crate::protocol::store_publication::fixture_prepare::METHOD,
                crate::protocol::store_gc::METHOD,
                crate::protocol::store_publication::inspect::METHOD,
                crate::protocol::store_publication::prepare::METHOD,
                crate::protocol::store_publication::stage::METHOD,
                crate::protocol::store_activation::prepare::METHOD,
                crate::protocol::store_activation::activate::METHOD,
                crate::protocol::cascade::topology::METHOD,
                CANIC_WASM_STORE_CHUNK,
                CANIC_WASM_STORE_FIXTURE_CHUNK,
                CANIC_WASM_STORE_PUBLISH_CHUNK,
                CANIC_WASM_STORE_PUBLISH_FIXTURE,
            ],
        );
    let managed_updates = !is_wasm_store
        && is_update(
            call,
            &[
                crate::protocol::runtime_configuration::METHOD,
                crate::protocol::issuer_proof::INSTALL,
                crate::protocol::admission_control::target_prepare::METHOD,
                crate::protocol::admission_control::target_activate::METHOD,
                crate::protocol::admission_control::target_open::METHOD,
                crate::protocol::caller_authority::PREPARE,
                crate::protocol::caller_authority::COMMIT,
                crate::protocol::caller_authority::COMPLETE,
                crate::protocol::caller_authority::RELEASE,
                crate::protocol::observability_relay::child_funding::METHOD,
                crate::protocol::observability_relay::cycle_balance::METHOD,
                crate::protocol::observability_relay::cycle_history::METHOD,
                crate::protocol::observability_relay::cycle_topups::METHOD,
                crate::protocol::observability_relay::memory_allocations::METHOD,
                crate::protocol::observability_relay::metrics::METHOD,
            ],
        );
    if is_query(call, reads)
        || is_update(
            call,
            &[
                crate::protocol::cycles_capability::METHOD,
                crate::protocol::cascade::state::METHOD,
            ],
        )
        || store_data
        || managed_updates
    {
        return Ok(());
    }
    fenced(call)
}

/// Infrastructure traffic remains available before fixture completion, with its own auth checks.
#[must_use]
pub fn is_nonroot_infrastructure_endpoint(call: EndpointCall) -> bool {
    require_prepared_nonroot_endpoint(call, false).is_ok()
}

/// Require one exact recovery endpoint admitted for a Prepared root.
pub fn require_prepared_root_endpoint(
    call: EndpointCall,
) -> Result<(), FleetActivationEndpointPolicyError> {
    let fixture_status = call.endpoint.name == CANIC_ROOT_FIXTURE_STATUS
        && call.kind == EndpointCallKind::QueryComposite;
    if fixture_status
        || is_query(
            call,
            &[
                CANIC_PUBLIC_HEALTH,
                CANIC_PUBLIC_METRICS,
                CANIC_PUBLIC_HISTORY,
                CANIC_PUBLIC_OVERVIEW,
                crate::protocol::cycle_balance::METHOD,
                crate::protocol::cycle_history::METHOD,
                CANIC_PUBLIC_CHILDREN,
                crate::protocol::component_directory::HEAD,
                crate::protocol::component_directory::PAGE,
                crate::protocol::component_directory::MEMBER_PAGE,
                crate::protocol::inspection_reserve::METHOD,
                crate::protocol::protected_readiness::METHOD,
                crate::protocol::protected_runtime::METHOD,
                crate::protocol::protected_logs::METHOD,
                crate::protocol::protected_metrics::METHOD,
                crate::protocol::protected_memory_allocations::METHOD,
                crate::protocol::child_funding::METHOD,
                crate::protocol::protected_health::METHOD,
                crate::protocol::component_read::provisioning::METHOD,
                crate::protocol::root_store_control::adoption::METHOD,
                crate::protocol::root_activation::status::METHOD,
                crate::protocol::registry_control::synchronization_status::METHOD,
                crate::protocol::cycle_refill::status::METHOD,
                crate::protocol::root_store_control::bootstrap_status::METHOD,
                crate::protocol::component_read::child_allocation::METHOD,
                crate::protocol::component_read::allocation::METHOD,
                crate::protocol::subtree_read::METHOD,
                crate::protocol::root_removal_read::METHOD,
                crate::protocol::component_removal_read::METHOD,
                crate::protocol::root_config::METHOD,
                crate::protocol::root_authority::METHOD,
                crate::protocol::registry_read::status::METHOD,
                crate::protocol::registry_read::partition::METHOD,
                crate::protocol::registry_read::active_partition::METHOD,
                crate::protocol::protected_state::authority_restore::METHOD,
                crate::protocol::protected_state::fleet_state::METHOD,
                crate::protocol::pool_import_read::status::METHOD,
                crate::protocol::pool_import_read::context::METHOD,
                crate::protocol::pool_census::METHOD,
                crate::protocol::pool_inventory::METHOD,
                crate::protocol::provisioning_census::METHOD,
                crate::protocol::root_store_reporting::overview::METHOD,
            ],
        )
        || is_update(
            call,
            &[
                crate::protocol::root_store_control::adopt::METHOD,
                crate::protocol::root_store_control::bootstrap::METHOD,
                crate::protocol::root_store_control::prepare::METHOD,
                crate::protocol::root_store_control::publish::METHOD,
                crate::protocol::pool_control::handoff::METHOD,
                crate::protocol::pool_control::import::METHOD,
                crate::protocol::pool_control::maintain::METHOD,
                crate::protocol::registry_control::synchronize::METHOD,
                crate::protocol::component_control::provision::METHOD,
                crate::protocol::component_control::prepare::METHOD,
                crate::protocol::batch_control::accept::METHOD,
                crate::protocol::batch_control::synchronize::METHOD,
                crate::protocol::inspection_history::METHOD,
                crate::protocol::import_control::reserve::METHOD,
                crate::protocol::import_control::advance::METHOD,
                crate::protocol::import_control::settle::METHOD,
                crate::protocol::import_control::release::METHOD,
                crate::protocol::funding_control::root_accept::METHOD,
                crate::protocol::root_activation::prepare::METHOD,
                crate::protocol::root_activation::resume::METHOD,
                crate::protocol::component_initialization::METHOD,
                crate::protocol::root_capability::acknowledge::METHOD,
                crate::protocol::root_capability::allocate::METHOD,
                crate::protocol::root_capability::create::METHOD,
                crate::protocol::root_capability::recycle::METHOD,
                crate::protocol::root_capability::cycles::METHOD,
                crate::protocol::inspection::METHOD,
                crate::protocol::fleet_state_control::funding::METHOD,
                crate::protocol::fleet_state_control::status::METHOD,
                crate::protocol::pool_retry::refill::METHOD,
                crate::protocol::pool_retry::reset::METHOD,
                crate::protocol::admission_control::root_prepare::METHOD,
                crate::protocol::admission_control::root_activate::METHOD,
                crate::protocol::admission_control::root_open::METHOD,
                crate::protocol::root_observation::child_funding::METHOD,
                crate::protocol::root_observation::cycle_balance::METHOD,
                crate::protocol::root_observation::cycle_history::METHOD,
                crate::protocol::root_observation::cycle_topups::METHOD,
                crate::protocol::root_observation::memory_allocations::METHOD,
                crate::protocol::root_observation::metrics::METHOD,
            ],
        )
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
    use canic_contracts::ids::EndpointId;

    fn call(name: &'static str, kind: EndpointCallKind) -> EndpointCall {
        EndpointCall {
            endpoint: EndpointId::new(name),
            kind,
        }
    }

    #[test]
    fn fixture_gate_exempts_only_exact_infrastructure_names_and_call_modes() {
        for name in [
            CANIC_PUBLIC_HEALTH,
            CANIC_PUBLIC_METRICS,
            CANIC_PUBLIC_HISTORY,
            CANIC_PUBLIC_OVERVIEW,
            crate::protocol::cycle_balance::METHOD,
            crate::protocol::cycle_history::METHOD,
            crate::protocol::protected_logs::METHOD,
            crate::protocol::protected_metrics::METHOD,
            crate::protocol::protected_memory_allocations::METHOD,
            crate::protocol::child_funding::METHOD,
            crate::protocol::protected_health::METHOD,
            crate::protocol::cycle_topups::METHOD,
            CANIC_PUBLIC_CHILDREN,
            crate::protocol::protected_readiness::METHOD,
            crate::protocol::protected_runtime::METHOD,
            crate::protocol::protected_binding::METHOD,
            crate::protocol::caller_authority_status::METHOD,
            crate::protocol::runtime_reporting::METHOD,
            crate::protocol::issuer_proof::status::METHOD,
            CANIC_ADMISSION_STATUS,
        ] {
            assert!(is_nonroot_infrastructure_endpoint(call(
                name,
                EndpointCallKind::Query
            )));
            assert!(!is_nonroot_infrastructure_endpoint(call(
                name,
                EndpointCallKind::Update
            )));
            assert!(!is_nonroot_infrastructure_endpoint(call(
                name,
                EndpointCallKind::QueryComposite
            )));
        }
        assert!(is_nonroot_infrastructure_endpoint(call(
            crate::protocol::cycles_capability::METHOD,
            EndpointCallKind::Update
        )));
        assert!(!is_nonroot_infrastructure_endpoint(call(
            crate::protocol::cycles_capability::METHOD,
            EndpointCallKind::Query
        )));
        for name in [
            "canic_application_query",
            "create_account",
            "canic_coordinator_funding_extra",
        ] {
            assert!(!is_nonroot_infrastructure_endpoint(call(
                name,
                EndpointCallKind::Query
            )));
            assert!(!is_nonroot_infrastructure_endpoint(call(
                name,
                EndpointCallKind::Update
            )));
        }
    }

    #[test]
    fn prepared_root_admits_only_the_role_owned_entrypoints() {
        use EndpointCallKind::{Query, QueryComposite, Update};
        for (endpoint, kind) in [
            (crate::protocol::pool_inventory::METHOD, Query),
            (crate::protocol::inspection::METHOD, Update),
            (crate::protocol::root_config::METHOD, Query),
            (crate::protocol::root_authority::METHOD, Query),
            (crate::protocol::registry_read::status::METHOD, Query),
            (crate::protocol::registry_read::partition::METHOD, Query),
            (
                crate::protocol::registry_read::active_partition::METHOD,
                Query,
            ),
            (crate::protocol::pool_import_read::status::METHOD, Query),
            (crate::protocol::pool_import_read::context::METHOD, Query),
            (crate::protocol::pool_census::METHOD, Query),
            (crate::protocol::provisioning_census::METHOD, Query),
            (
                crate::protocol::root_store_reporting::overview::METHOD,
                Query,
            ),
            (
                crate::protocol::protected_state::authority_restore::METHOD,
                Query,
            ),
            (crate::protocol::protected_state::fleet_state::METHOD, Query),
            (crate::protocol::protected_logs::METHOD, Query),
            (crate::protocol::protected_metrics::METHOD, Query),
            (crate::protocol::protected_memory_allocations::METHOD, Query),
            (crate::protocol::child_funding::METHOD, Query),
            (crate::protocol::protected_health::METHOD, Query),
            (CANIC_ROOT_FIXTURE_STATUS, QueryComposite),
            (crate::protocol::root_removal_read::METHOD, Query),
            (
                crate::protocol::registry_control::synchronization_status::METHOD,
                Query,
            ),
            (crate::protocol::root_activation::status::METHOD, Query),
            (crate::protocol::cycle_refill::status::METHOD, Query),
            (crate::protocol::component_read::provisioning::METHOD, Query),
            (crate::protocol::root_store_control::adoption::METHOD, Query),
            (
                crate::protocol::root_store_control::bootstrap_status::METHOD,
                Query,
            ),
            (
                crate::protocol::component_read::child_allocation::METHOD,
                Query,
            ),
            (crate::protocol::component_read::allocation::METHOD, Query),
            (crate::protocol::subtree_read::METHOD, Query),
            (CANIC_PUBLIC_HEALTH, Query),
            (CANIC_PUBLIC_METRICS, Query),
            (CANIC_PUBLIC_HISTORY, Query),
            (crate::protocol::cycle_balance::METHOD, Query),
            (crate::protocol::cycle_history::METHOD, Query),
            (CANIC_PUBLIC_OVERVIEW, Query),
            (CANIC_PUBLIC_CHILDREN, Query),
            (crate::protocol::inspection_reserve::METHOD, Query),
            (crate::protocol::protected_readiness::METHOD, Query),
            (crate::protocol::protected_runtime::METHOD, Query),
        ] {
            assert_eq!(require_prepared_root_endpoint(call(endpoint, kind)), Ok(()));
        }
    }

    #[test]
    fn prepared_root_rejects_ordinary_and_wrong_kind_calls() {
        for (endpoint, kind) in [
            (
                crate::protocol::funding_read::release::METHOD,
                EndpointCallKind::Query,
            ),
            ("application_update", EndpointCallKind::Update),
            (crate::protocol::inspection::METHOD, EndpointCallKind::Query),
            (
                crate::protocol::root_store_reporting::overview::METHOD,
                EndpointCallKind::Update,
            ),
            (
                crate::protocol::root_store_reporting::overview::METHOD,
                EndpointCallKind::QueryComposite,
            ),
            (
                crate::protocol::provisioning_census::METHOD,
                EndpointCallKind::Update,
            ),
            (
                crate::protocol::provisioning_census::METHOD,
                EndpointCallKind::QueryComposite,
            ),
            (
                crate::protocol::pool_census::METHOD,
                EndpointCallKind::Update,
            ),
            (
                crate::protocol::pool_census::METHOD,
                EndpointCallKind::QueryComposite,
            ),
            (
                crate::protocol::pool_import_read::context::METHOD,
                EndpointCallKind::Update,
            ),
            (
                crate::protocol::pool_import_read::context::METHOD,
                EndpointCallKind::QueryComposite,
            ),
            (
                crate::protocol::pool_import_read::status::METHOD,
                EndpointCallKind::Update,
            ),
            (
                crate::protocol::pool_import_read::status::METHOD,
                EndpointCallKind::QueryComposite,
            ),
            (
                crate::protocol::funding_read::root_status::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::funding_read::root_status::METHOD,
                EndpointCallKind::Update,
            ),
            (
                crate::protocol::funding_read::root_status::METHOD,
                EndpointCallKind::QueryComposite,
            ),
            (CANIC_ROOT_FIXTURE_STATUS, EndpointCallKind::Query),
            (CANIC_ROOT_FIXTURE_STATUS, EndpointCallKind::Update),
        ] {
            assert_eq!(
                require_prepared_root_endpoint(call(endpoint, kind)),
                Err(FleetActivationEndpointPolicyError::Fenced { endpoint, kind })
            );
        }
    }

    #[test]
    fn caller_publication_updates_are_managed_recovery_only() {
        use crate::protocol::caller_authority as protocol;
        for endpoint in [
            protocol::PREPARE,
            protocol::COMMIT,
            protocol::COMPLETE,
            protocol::RELEASE,
        ] {
            let update = call(endpoint, EndpointCallKind::Update);
            assert!(require_prepared_nonroot_endpoint(update, false).is_ok());
            assert!(is_nonroot_infrastructure_endpoint(update));
            assert!(require_prepared_nonroot_endpoint(update, true).is_err());
            assert!(require_prepared_root_endpoint(update).is_err());
            assert!(
                require_prepared_nonroot_endpoint(call(endpoint, EndpointCallKind::Query), false)
                    .is_err()
            );
        }
    }

    #[test]
    fn replicated_observations_preserve_prepared_managed_admission() {
        use crate::protocol::observability_relay as protocol;
        for endpoint in [
            protocol::child_funding::METHOD,
            protocol::cycle_balance::METHOD,
            protocol::cycle_history::METHOD,
            protocol::cycle_topups::METHOD,
            protocol::memory_allocations::METHOD,
            protocol::metrics::METHOD,
        ] {
            let update = call(endpoint, EndpointCallKind::Update);
            assert!(require_prepared_nonroot_endpoint(update, false).is_ok());
            assert!(is_nonroot_infrastructure_endpoint(update));
            assert!(require_prepared_nonroot_endpoint(update, true).is_err());
            assert!(require_prepared_root_endpoint(update).is_err());
            assert!(
                require_prepared_nonroot_endpoint(call(endpoint, EndpointCallKind::Query), false)
                    .is_err()
            );
        }
    }

    #[test]
    fn prepared_nonroot_uses_the_ordinary_role_recovery_allowlist() {
        for (endpoint, kind) in [
            (
                crate::protocol::caller_authority_status::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::runtime_reporting::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::issuer_proof::status::METHOD,
                EndpointCallKind::Query,
            ),
            (CANIC_ADMISSION_STATUS, EndpointCallKind::Query),
            (CANIC_PUBLIC_HEALTH, EndpointCallKind::Query),
            (CANIC_PUBLIC_METRICS, EndpointCallKind::Query),
            (CANIC_PUBLIC_HISTORY, EndpointCallKind::Query),
            (
                crate::protocol::cycle_balance::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::cycle_history::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::cycle_topups::METHOD,
                EndpointCallKind::Query,
            ),
            (CANIC_PUBLIC_OVERVIEW, EndpointCallKind::Query),
            (CANIC_PUBLIC_CHILDREN, EndpointCallKind::Query),
            (
                crate::protocol::protected_readiness::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::protected_runtime::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::protected_binding::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::cycles_capability::METHOD,
                EndpointCallKind::Update,
            ),
        ] {
            assert_eq!(
                require_prepared_nonroot_endpoint(call(endpoint, kind), false),
                Ok(())
            );
        }

        for (endpoint, kind) in [
            ("application_update", EndpointCallKind::Update),
            ("application_query", EndpointCallKind::Query),
            (CANIC_ROOT_FIXTURE_STATUS, EndpointCallKind::QueryComposite),
            (
                crate::protocol::store_catalog::fixture::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::store_catalog::grant::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::store_catalog::template::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::runtime_reporting::METHOD,
                EndpointCallKind::Update,
            ),
            (
                crate::protocol::cycles_capability::METHOD,
                EndpointCallKind::Query,
            ),
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
            (
                crate::protocol::store_status::authority::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::store_status::activation::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::store_status::gc::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::store_catalog::fixture::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::store_catalog::grant::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::store_catalog::template::METHOD,
                EndpointCallKind::Query,
            ),
            (CANIC_PUBLIC_HEALTH, EndpointCallKind::Query),
            (CANIC_PUBLIC_METRICS, EndpointCallKind::Query),
            (CANIC_PUBLIC_HISTORY, EndpointCallKind::Query),
            (
                crate::protocol::cycle_balance::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::cycle_history::METHOD,
                EndpointCallKind::Query,
            ),
            (CANIC_PUBLIC_OVERVIEW, EndpointCallKind::Query),
            (
                crate::protocol::store_control::grant::METHOD,
                EndpointCallKind::Update,
            ),
        ] {
            assert_eq!(
                require_prepared_nonroot_endpoint(call(endpoint, kind), true),
                Ok(())
            );
        }
        for (endpoint, kind) in [
            (
                crate::protocol::caller_authority_status::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::runtime_reporting::METHOD,
                EndpointCallKind::Query,
            ),
            (
                crate::protocol::issuer_proof::INSTALL,
                EndpointCallKind::Update,
            ),
            (
                crate::protocol::store_status::authority::METHOD,
                EndpointCallKind::Update,
            ),
            (
                crate::protocol::store_status::activation::METHOD,
                EndpointCallKind::Update,
            ),
            (
                crate::protocol::store_status::gc::METHOD,
                EndpointCallKind::Update,
            ),
            (CANIC_ROOT_FIXTURE_STATUS, EndpointCallKind::QueryComposite),
            (
                crate::protocol::store_catalog::fixture::METHOD,
                EndpointCallKind::Update,
            ),
            (
                crate::protocol::store_catalog::grant::METHOD,
                EndpointCallKind::Update,
            ),
            (
                crate::protocol::store_catalog::template::METHOD,
                EndpointCallKind::Update,
            ),
            (
                crate::protocol::issuer_proof::status::METHOD,
                EndpointCallKind::Query,
            ),
            (CANIC_ADMISSION_STATUS, EndpointCallKind::Query),
            (
                crate::protocol::store_control::grant::METHOD,
                EndpointCallKind::Query,
            ),
        ] {
            assert_eq!(
                require_prepared_nonroot_endpoint(call(endpoint, kind), true),
                Err(FleetActivationEndpointPolicyError::Fenced { endpoint, kind })
            );
        }
    }

    #[test]
    fn prepared_store_data_requires_exact_role_name_and_update_kind() {
        for endpoint in [
            CANIC_WASM_STORE_CHUNK,
            CANIC_WASM_STORE_FIXTURE_CHUNK,
            CANIC_WASM_STORE_PUBLISH_CHUNK,
            CANIC_WASM_STORE_PUBLISH_FIXTURE,
        ] {
            for kind in [
                EndpointCallKind::Update,
                EndpointCallKind::Query,
                EndpointCallKind::QueryComposite,
            ] {
                let call = call(endpoint, kind);
                assert_eq!(
                    require_prepared_nonroot_endpoint(call, true),
                    if kind == EndpointCallKind::Update {
                        Ok(())
                    } else {
                        Err(FleetActivationEndpointPolicyError::Fenced { endpoint, kind })
                    }
                );
                assert_eq!(
                    require_prepared_nonroot_endpoint(call, false),
                    Err(FleetActivationEndpointPolicyError::Fenced { endpoint, kind })
                );
                assert!(!is_nonroot_infrastructure_endpoint(call));
                assert!(require_prepared_root_endpoint(call).is_err());
            }
        }
        let endpoint = "application_update";
        let kind = EndpointCallKind::Update;
        assert_eq!(
            require_prepared_nonroot_endpoint(call(endpoint, kind), true),
            Err(FleetActivationEndpointPolicyError::Fenced { endpoint, kind })
        );
    }
    #[test]
    fn outer_observations_preserve_prepared_root_admission_only() {
        for method in [
            crate::protocol::root_observation::child_funding::METHOD,
            crate::protocol::root_observation::cycle_balance::METHOD,
            crate::protocol::root_observation::cycle_history::METHOD,
            crate::protocol::root_observation::cycle_topups::METHOD,
            crate::protocol::root_observation::memory_allocations::METHOD,
            crate::protocol::root_observation::metrics::METHOD,
        ] {
            assert_eq!(
                require_prepared_root_endpoint(call(method, EndpointCallKind::Update)),
                Ok(())
            );
            assert!(require_prepared_root_endpoint(call(method, EndpointCallKind::Query)).is_err());
            for store in [false, true] {
                assert!(
                    require_prepared_nonroot_endpoint(
                        call(method, EndpointCallKind::Update),
                        store
                    )
                    .is_err()
                );
            }
        }
    }
}

#[cfg(test)]
mod native_admission_tests {
    use super::*;
    #[test]
    fn native_target_transitions_retain_infrastructure_admission() {
        for method in [
            crate::protocol::admission_control::target_prepare::METHOD,
            crate::protocol::admission_control::target_activate::METHOD,
            crate::protocol::admission_control::target_open::METHOD,
        ] {
            let call = crate::ids::EndpointCall {
                endpoint: crate::ids::EndpointId::new(method),
                kind: crate::ids::EndpointCallKind::Update,
            };
            assert!(is_nonroot_infrastructure_endpoint(call));
            assert!(require_prepared_nonroot_endpoint(call, true).is_err());
            let query = EndpointCall {
                kind: EndpointCallKind::Query,
                ..call
            };
            assert!(!is_nonroot_infrastructure_endpoint(query));
        }
    }
}

#[cfg(test)]
mod root_admission_transport_tests {
    use super::*;

    #[test]
    fn root_recovery_methods_are_root_only_update_infrastructure() {
        for method in [
            crate::protocol::fleet_state_control::funding::METHOD,
            crate::protocol::fleet_state_control::status::METHOD,
            crate::protocol::pool_retry::refill::METHOD,
            crate::protocol::pool_retry::reset::METHOD,
            crate::protocol::admission_control::root_prepare::METHOD,
            crate::protocol::admission_control::root_activate::METHOD,
            crate::protocol::admission_control::root_open::METHOD,
            crate::protocol::root_activation::prepare::METHOD,
            crate::protocol::root_activation::resume::METHOD,
            crate::protocol::funding_control::root_accept::METHOD,
            crate::protocol::root_store_control::adopt::METHOD,
            crate::protocol::root_store_control::bootstrap::METHOD,
            crate::protocol::root_store_control::prepare::METHOD,
            crate::protocol::root_store_control::publish::METHOD,
            crate::protocol::pool_control::handoff::METHOD,
            crate::protocol::pool_control::import::METHOD,
            crate::protocol::pool_control::maintain::METHOD,
            crate::protocol::registry_control::synchronize::METHOD,
            crate::protocol::component_control::provision::METHOD,
            crate::protocol::component_control::prepare::METHOD,
            crate::protocol::batch_control::accept::METHOD,
            crate::protocol::batch_control::synchronize::METHOD,
            crate::protocol::inspection_history::METHOD,
            crate::protocol::import_control::reserve::METHOD,
            crate::protocol::import_control::advance::METHOD,
            crate::protocol::import_control::settle::METHOD,
            crate::protocol::import_control::release::METHOD,
            crate::protocol::component_initialization::METHOD,
            crate::protocol::root_capability::acknowledge::METHOD,
            crate::protocol::root_capability::allocate::METHOD,
            crate::protocol::root_capability::create::METHOD,
            crate::protocol::root_capability::recycle::METHOD,
            crate::protocol::root_capability::cycles::METHOD,
        ] {
            let call = EndpointCall {
                endpoint: crate::ids::EndpointId::new(method),
                kind: EndpointCallKind::Update,
            };
            assert_eq!(require_prepared_root_endpoint(call), Ok(()));
            assert!(!is_nonroot_infrastructure_endpoint(call));
            for store in [false, true] {
                assert!(require_prepared_nonroot_endpoint(call, store).is_err());
            }
            for kind in [EndpointCallKind::Query, EndpointCallKind::QueryComposite] {
                assert!(require_prepared_root_endpoint(EndpointCall { kind, ..call }).is_err());
            }
        }
    }
}

#[cfg(test)]
mod native_cascade_admission_tests {
    use super::*;

    #[test]
    fn cascade_and_configuration_preserve_exact_role_and_call_kind() {
        for (method, managed, store) in [
            (crate::protocol::cascade::state::METHOD, true, true),
            (crate::protocol::cycles_capability::METHOD, true, true),
            (crate::protocol::cascade::topology::METHOD, false, true),
            (crate::protocol::runtime_configuration::METHOD, true, false),
            (crate::protocol::issuer_proof::INSTALL, true, false),
        ] {
            for kind in [
                EndpointCallKind::Update,
                EndpointCallKind::Query,
                EndpointCallKind::QueryComposite,
            ] {
                let call = EndpointCall {
                    endpoint: crate::ids::EndpointId::new(method),
                    kind,
                };
                assert_eq!(
                    require_prepared_nonroot_endpoint(call, false).is_ok(),
                    managed && kind == EndpointCallKind::Update
                );
                assert_eq!(
                    require_prepared_nonroot_endpoint(call, true).is_ok(),
                    store && kind == EndpointCallKind::Update
                );
                assert!(require_prepared_root_endpoint(call).is_err());
            }
        }
    }
}

#[cfg(test)]
mod auth_native_admission_tests {
    use super::*;

    #[test]
    fn issuer_and_attestation_methods_remain_closed_while_prepared() {
        for method in [
            crate::protocol::issuer_renewal::METHOD,
            crate::protocol::issuer_control::configure::METHOD,
            crate::protocol::issuer_control::public_key::METHOD,
            crate::protocol::issuer_control::proof::METHOD,
            crate::protocol::role_attestation::get::METHOD,
            crate::protocol::role_attestation::prepare::METHOD,
        ] {
            for kind in [
                EndpointCallKind::Update,
                EndpointCallKind::Query,
                EndpointCallKind::QueryComposite,
            ] {
                let call = EndpointCall {
                    endpoint: crate::ids::EndpointId::new(method),
                    kind,
                };
                assert!(require_prepared_root_endpoint(call).is_err());
                assert!(require_prepared_nonroot_endpoint(call, false).is_err());
                assert!(require_prepared_nonroot_endpoint(call, true).is_err());
            }
        }
    }
}

#[cfg(test)]
mod store_inventory_admission_tests {
    use super::*;

    #[test]
    fn inventory_is_a_prepared_store_read_only() {
        for method in [
            crate::protocol::store_inventory::catalog::METHOD,
            crate::protocol::store_inventory::storage::METHOD,
        ] {
            for kind in [
                EndpointCallKind::Query,
                EndpointCallKind::QueryComposite,
                EndpointCallKind::Update,
            ] {
                let call = EndpointCall {
                    endpoint: crate::ids::EndpointId::new(method),
                    kind,
                };
                assert_eq!(
                    require_prepared_nonroot_endpoint(call, true).is_ok(),
                    kind == EndpointCallKind::Query
                );
                assert!(require_prepared_nonroot_endpoint(call, false).is_err());
                assert!(require_prepared_root_endpoint(call).is_err());
            }
        }
    }
}

#[cfg(test)]
mod store_control_admission_tests {
    use super::*;

    #[test]
    fn publication_and_gc_keep_prepared_store_update_admission_only() {
        for method in [
            crate::protocol::store_publication::inspect::METHOD,
            crate::protocol::store_publication::prepare::METHOD,
            crate::protocol::store_publication::stage::METHOD,
            crate::protocol::store_control::grant::METHOD,
            crate::protocol::store_control::reclaim::METHOD,
            crate::protocol::store_activation::prepare::METHOD,
            crate::protocol::store_activation::activate::METHOD,
            crate::protocol::store_publication::fixture_prepare::METHOD,
            crate::protocol::store_gc::METHOD,
        ] {
            for kind in [
                EndpointCallKind::Update,
                EndpointCallKind::Query,
                EndpointCallKind::QueryComposite,
            ] {
                let call = EndpointCall {
                    endpoint: crate::ids::EndpointId::new(method),
                    kind,
                };
                assert_eq!(
                    require_prepared_nonroot_endpoint(call, true).is_ok(),
                    kind == EndpointCallKind::Update
                );
                assert!(require_prepared_nonroot_endpoint(call, false).is_err());
                assert!(require_prepared_root_endpoint(call).is_err());
            }
        }
    }
}

#[cfg(test)]
mod ordinary_mutation_admission_tests {
    use super::*;
    #[test]
    fn allocation_refill_and_rotation_do_not_bypass_prepared_root() {
        for method in [
            crate::protocol::funding_control::root_prepare::METHOD,
            crate::protocol::funding_control::root_activate::METHOD,
            crate::protocol::cycle_refill::preview::METHOD,
            crate::protocol::cycle_refill::execute::METHOD,
            crate::protocol::component_control::child::METHOD,
            crate::protocol::component_control::peer::METHOD,
            crate::protocol::component_control::remove::METHOD,
            crate::protocol::component_control::subtree::METHOD,
        ] {
            let call = EndpointCall {
                endpoint: crate::ids::EndpointId::new(method),
                kind: EndpointCallKind::Update,
            };
            assert!(require_prepared_root_endpoint(call).is_err());
            assert!(require_prepared_nonroot_endpoint(call, false).is_err());
            assert!(require_prepared_nonroot_endpoint(call, true).is_err());
        }
    }
}
