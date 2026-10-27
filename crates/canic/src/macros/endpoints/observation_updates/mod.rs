//! Replicated observations preserve controller admission and optional provider refusal.

/// Select a compiled provider without evaluating or retaining a disabled provider body.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_optional_observation {
    ($capability:meta, $body:expr) => {{
        #[cfg($capability)]
        {
            $body
        }
        #[cfg(not($capability))]
        {
            Err(::canic::Error::from_registered(
                ::canic::diagnostics::codes::REQUEST_INVALID,
            ))
        }
    }};
}

/// Emit fixed typed observation updates for Root's inter-canister relay.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_observation_updates {
    () => {
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::observability_relay::child_funding::METHOD_ID, request = ::canic::protocol::observability_relay::child_funding::REQUEST_LIMITS, response = ::canic::protocol::observability_relay::child_funding::RESPONSE_LIMITS))]
        fn canic_observe_child_funding(child: $crate::__internal::ic_platform::Principal) -> Result<::canic::dto::child_funding::ChildFundingUsage, ::canic::Error> {

            $crate::__internal::core::api::observability::ObservabilityApi::child_funding(child.into()).map(Into::into)
        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::observability_relay::cycle_balance::METHOD_ID, request = ::canic::protocol::observability_relay::cycle_balance::REQUEST_LIMITS, response = ::canic::protocol::observability_relay::cycle_balance::RESPONSE_LIMITS))]
        fn canic_observe_cycle_balance() -> Result<::canic::dto::role::CycleBalanceStatusResponse, ::canic::Error> {

            Ok(::canic::dto::role::CycleBalanceStatusResponse { cycles: $crate::__internal::cdk::api::canister_cycle_balance() })
        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::observability_relay::cycle_history::METHOD_ID, request = ::canic::protocol::observability_relay::cycle_history::REQUEST_LIMITS, response = ::canic::protocol::observability_relay::cycle_history::RESPONSE_LIMITS))]
        fn canic_observe_cycle_history(page: ::canic::dto::page::PageRequest) -> Result<::canic::dto::page::Page<::canic::dto::cycles::CycleTrackerEntry>, ::canic::Error> {
            let _ = &page;
            $crate::__canic_optional_observation!(canic_capability_observability_history, Ok($crate::__internal::core::api::cycles::CycleTrackerQuery::page(page)))
        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::observability_relay::cycle_topups::METHOD_ID, request = ::canic::protocol::observability_relay::cycle_topups::REQUEST_LIMITS, response = ::canic::protocol::observability_relay::cycle_topups::RESPONSE_LIMITS))]
        fn canic_observe_cycle_topups(page: ::canic::dto::page::PageRequest) -> Result<::canic::dto::page::Page<::canic::dto::cycle_topups::CycleTopupEvent>, ::canic::Error> {
            let _ = &page;
            $crate::__canic_optional_observation!(all(canic_capability_observability_history, canic_capability_automatic_topup), Ok($crate::__internal::core::api::cycles::CycleTrackerQuery::topups(page).into()))
        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::observability_relay::memory_allocations::METHOD_ID, request = ::canic::protocol::observability_relay::memory_allocations::REQUEST_LIMITS, response = ::canic::protocol::observability_relay::memory_allocations::RESPONSE_LIMITS))]
        fn canic_observe_memory_allocations() -> Result<::canic::dto::memory::MemoryAllocationsResponse, ::canic::Error> {

            $crate::__canic_optional_observation!(canic_capability_observability_diagnostics, $crate::__internal::core::api::memory::MemoryQuery::allocations())
        }
        #[$crate::canic_update(requires(caller::is_controller()), wire(id = ::canic::protocol::observability_relay::metrics::METHOD_ID, request = ::canic::protocol::observability_relay::metrics::REQUEST_LIMITS, response = ::canic::protocol::observability_relay::metrics::RESPONSE_LIMITS))]
        fn canic_observe_metrics(request: ::canic::dto::role::MetricsStatusRequest) -> Result<::canic::dto::page::Page<::canic::dto::metric_page::MetricEntry>, ::canic::Error> {
            let _ = &request;
            $crate::__canic_optional_observation!(canic_capability_observability_metrics, $crate::__canic_role_metrics_status!(request).map(Into::into))
        }
    };
}
