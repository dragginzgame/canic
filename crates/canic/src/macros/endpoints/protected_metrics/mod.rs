//! Controller-only metrics with role-owned activation fencing.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_protected_metrics_endpoint {
    (before = [$($before:stmt)*]) => {
        #[$crate::query(requires(caller::is_controller()), wire(id = ::canic::protocol::protected_metrics::METHOD_ID, request = ::canic::protocol::protected_metrics::REQUEST_LIMITS, response = ::canic::protocol::protected_metrics::RESPONSE_LIMITS))]
        fn canic_metrics(request: ::canic::dto::role::MetricsStatusRequest) -> Result<::canic::dto::page::Page<::canic::dto::metric_page::MetricEntry>, ::canic::Error> {
            $($before)*
            $crate::__canic_role_metrics_status!(request).map(Into::into)
        }
    };
}
