//! Controller-authenticated, fixed typed Root observation updates.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_root_observation_updates {
    () => {
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::root_observation::child_funding::METHOD_ID, request = ::canic::protocol::root_observation::child_funding::REQUEST_LIMITS, response = ::canic::protocol::root_observation::child_funding::RESPONSE_LIMITS))]
        async fn canic_root_observe_child_funding(request: ::canic::dto::root_observation::Request<$crate::__internal::ic_platform::Principal>) -> Result<::canic::dto::child_funding::ChildFundingUsage, ::canic::Error> {
            $crate::__internal::core::api::observability::ObservabilityApi::observe_root_controlled_canister(request.canister_id.into(), ::canic::dto::observability::CanisterObservabilityRequest::ChildFunding(request.input.into())).await
        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::root_observation::cycle_balance::METHOD_ID, request = ::canic::protocol::root_observation::cycle_balance::REQUEST_LIMITS, response = ::canic::protocol::root_observation::cycle_balance::RESPONSE_LIMITS))]
        async fn canic_root_observe_cycle_balance(request: ::canic::dto::root_observation::Request<()>) -> Result<::canic::dto::role::CycleBalanceStatusResponse, ::canic::Error> {
            $crate::__internal::core::api::observability::ObservabilityApi::observe_root_controlled_canister(request.canister_id.into(), ::canic::dto::observability::CanisterObservabilityRequest::CycleBalance).await
        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::root_observation::cycle_history::METHOD_ID, request = ::canic::protocol::root_observation::cycle_history::REQUEST_LIMITS, response = ::canic::protocol::root_observation::cycle_history::RESPONSE_LIMITS))]
        async fn canic_root_observe_cycle_history(request: ::canic::dto::root_observation::Request<::canic::dto::page::PageRequest>) -> Result<::canic::dto::page::Page<::canic::dto::cycles::CycleTrackerEntry>, ::canic::Error> {
            $crate::__internal::core::api::observability::ObservabilityApi::observe_root_controlled_canister(request.canister_id.into(), ::canic::dto::observability::CanisterObservabilityRequest::CycleHistory(request.input)).await
        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::root_observation::cycle_topups::METHOD_ID, request = ::canic::protocol::root_observation::cycle_topups::REQUEST_LIMITS, response = ::canic::protocol::root_observation::cycle_topups::RESPONSE_LIMITS))]
        async fn canic_root_observe_cycle_topups(request: ::canic::dto::root_observation::Request<::canic::dto::page::PageRequest>) -> Result<::canic::dto::page::Page<::canic::dto::cycle_topups::CycleTopupEvent>, ::canic::Error> {
            $crate::__internal::core::api::observability::ObservabilityApi::observe_root_controlled_canister(request.canister_id.into(), ::canic::dto::observability::CanisterObservabilityRequest::CycleTopups(request.input)).await
        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::root_observation::memory_allocations::METHOD_ID, request = ::canic::protocol::root_observation::memory_allocations::REQUEST_LIMITS, response = ::canic::protocol::root_observation::memory_allocations::RESPONSE_LIMITS))]
        async fn canic_root_observe_memory_allocations(request: ::canic::dto::root_observation::Request<()>) -> Result<::canic::dto::memory::MemoryAllocationsResponse, ::canic::Error> {
            $crate::__internal::core::api::observability::ObservabilityApi::observe_root_controlled_canister(request.canister_id.into(), ::canic::dto::observability::CanisterObservabilityRequest::MemoryAllocations).await
        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::root_observation::metrics::METHOD_ID, request = ::canic::protocol::root_observation::metrics::REQUEST_LIMITS, response = ::canic::protocol::root_observation::metrics::RESPONSE_LIMITS))]
        async fn canic_root_observe_metrics(request: ::canic::dto::root_observation::Request<::canic::dto::role::MetricsStatusRequest>) -> Result<::canic::dto::page::Page<::canic::dto::metric_page::MetricEntry>, ::canic::Error> {
            $crate::__internal::core::api::observability::ObservabilityApi::observe_root_controlled_canister(request.canister_id.into(), ::canic::dto::observability::CanisterObservabilityRequest::Metrics(request.input)).await
        }
    };
}
