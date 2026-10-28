//! Controller-only health diagnostics with role-owned readiness fences.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_protected_health_endpoint {
    (before = [$($before:stmt)*]) => {
        #[$crate::query(requires(caller::is_controller()), wire(id = ::canic::protocol::protected_health::METHOD_ID, request = ::canic::protocol::protected_health::REQUEST_LIMITS, response = ::canic::protocol::protected_health::RESPONSE_LIMITS))]
        fn canic_health() -> Result<::canic::dto::runtime::CanicHealthStatus, ::canic::Error> {
            $($before)*
            Ok($crate::__internal::core::api::runtime::RuntimeIntrospectionApi::health(Some(
                $crate::__internal::cdk::api::time(),
            )))
        }
    };
}
