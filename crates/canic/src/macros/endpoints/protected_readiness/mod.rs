//! Typed readiness observations retain their role-owned activation fence.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_protected_readiness_endpoint {
    (before = [$($before:stmt)*]) => {
        #[$crate::query(requires(caller::is_controller()), wire(id = ::canic::protocol::protected_readiness::METHOD_ID, request = ::canic::protocol::protected_readiness::REQUEST_LIMITS, response = ::canic::protocol::protected_readiness::RESPONSE_LIMITS))]
        fn canic_readiness() -> Result<::canic::dto::runtime_observation::CanicReadinessStatus, ::canic::Error> {
            $($before)*
            Ok($crate::__internal::core::api::runtime::RuntimeIntrospectionApi::readiness($crate::__internal::cdk::api::time()).into())
        }
    };
}
