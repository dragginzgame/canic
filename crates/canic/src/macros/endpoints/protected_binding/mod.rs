//! Typed binding observations retain their role-owned activation fence.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_protected_binding_endpoint {
    (before = [$($before:stmt)*]) => {
        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::protected_binding::METHOD_ID, request = ::canic::protocol::protected_binding::REQUEST_LIMITS, response = ::canic::protocol::protected_binding::RESPONSE_LIMITS))]
        fn canic_binding() -> Result<::canic::dto::component_directory::ManagedCanisterBinding, ::canic::Error> {
            $($before)*
            $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::managed_binding().map(Into::into)
        }
    };
}
