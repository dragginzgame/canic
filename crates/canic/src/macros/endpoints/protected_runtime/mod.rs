//! Typed runtime observations retain their role-owned activation fence.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_protected_runtime_endpoint {
    (before = [$($before:stmt)*]) => {
        #[$crate::query(requires(caller::is_controller()), wire(id = ::canic::protocol::protected_runtime::METHOD_ID, request = ::canic::protocol::protected_runtime::REQUEST_LIMITS, response = ::canic::protocol::protected_runtime::RESPONSE_LIMITS))]
        fn canic_runtime() -> Result<::canic::dto::runtime_observation::CanicRuntimeStatus, ::canic::Error> {
            $($before)*
            Ok($crate::__internal::core::api::runtime::RuntimeIntrospectionApi::runtime_status($crate::__internal::cdk::api::time(), env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"), $crate::VERSION, $crate::__internal::cdk::api::canister_version()).into())
        }
    };
}
