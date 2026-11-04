//! Explicit controller recovery for existing Root and Coordinator business journals.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_operation_retry_endpoints {
    () => {
        #[$crate::query(requires(caller::is_controller()), wire(id = ::canic::protocol::operation_retry::status::METHOD_ID, request = ::canic::protocol::operation_retry::REQUEST_LIMITS, response = ::canic::protocol::operation_retry::RESPONSE_LIMITS))]
        fn canic_operation_retry_status(operation: ::canic::dto::operation_retry::Operation) -> Result<::canic::dto::operation_retry::Status, ::canic::Error> {
            $crate::__internal::control_plane::api::operation_retry::OperationRetryApi::status(operation)
        }
        #[$crate::update(requires(caller::is_controller()), wire(id = ::canic::protocol::operation_retry::resume::METHOD_ID, request = ::canic::protocol::operation_retry::REQUEST_LIMITS, response = ::canic::protocol::operation_retry::RESPONSE_LIMITS))]
        fn canic_operation_retry_resume(request: ::canic::dto::operation_retry::ResumeRequest) -> Result<::canic::dto::operation_retry::Status, ::canic::Error> {
            $crate::__internal::core::api::authority_restore::AuthorityRestoreApi::require_command_variant_allowed(false)?;
            $crate::__internal::control_plane::api::operation_retry::OperationRetryApi::resume(request)
        }
    };
}
