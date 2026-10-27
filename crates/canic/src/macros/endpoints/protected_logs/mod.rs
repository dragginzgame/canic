//! Controller-only logs with role-owned activation fencing.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_protected_logs_endpoint {
    (before = [$($before:stmt)*]) => {
        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::protected_logs::METHOD_ID, request = ::canic::protocol::protected_logs::REQUEST_LIMITS, response = ::canic::protocol::protected_logs::RESPONSE_LIMITS))]
        fn canic_logs(request: ::canic::dto::role::LogStatusRequest) -> Result<::canic::dto::page::Page<::canic::dto::log::LogEntry>, ::canic::Error> {
            $($before)*
            Ok($crate::__internal::core::api::log::LogQuery::page(request.crate_name, request.topic, request.min_level, request.page))
        }
    };
}
