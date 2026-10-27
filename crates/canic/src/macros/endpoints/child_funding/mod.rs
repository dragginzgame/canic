//! Direct funding observations preserve role-owned activation admission.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_child_funding_endpoint {
    (before = [$($before:stmt)*]) => {
        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::child_funding::METHOD_ID, request = ::canic::protocol::child_funding::REQUEST_LIMITS, response = ::canic::protocol::child_funding::RESPONSE_LIMITS))]
        fn canic_child_funding(child: $crate::__internal::ic_platform::Principal) -> Result<::canic::dto::child_funding::ChildFundingUsage, ::canic::Error> {
            $($before)*
            $crate::__internal::core::api::observability::ObservabilityApi::child_funding(child.into()).map(Into::into)
        }
    };
}
