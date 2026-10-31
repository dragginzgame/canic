//! One direct live-balance endpoint across Root, managed, local and Store roles.

#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_cycle_balance_endpoint {
    (before = [$($before:stmt)*]) => {
        #[$crate::query(requires(caller::is_controller()), wire(id = ::canic::protocol::cycle_balance::METHOD_ID, request = ::canic::protocol::cycle_balance::REQUEST_LIMITS, response = ::canic::protocol::cycle_balance::RESPONSE_LIMITS))]
        fn canic_cycle_balance() -> Result<::canic::dto::role::CycleBalanceStatusResponse, ::canic::Error> {
            $($before)*
            Ok(::canic::dto::role::CycleBalanceStatusResponse {
                cycles: $crate::__internal::cdk::api::canister_cycle_balance(),
            })
        }
    };
}
