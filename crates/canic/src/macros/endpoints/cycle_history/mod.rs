//! One bounded history endpoint; role capability selection remains with its emitter.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_cycle_history_endpoint {
    (before = [$($before:stmt)*]) => {
        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::cycle_history::METHOD_ID, request = ::canic::protocol::cycle_history::REQUEST_LIMITS, response = ::canic::protocol::cycle_history::RESPONSE_LIMITS))]
        fn canic_cycle_history(page: ::canic::dto::page::PageRequest) -> Result<::canic::dto::page::Page<::canic::dto::cycles::CycleTrackerEntry>, ::canic::Error> {
            $($before)*
            Ok($crate::__internal::core::api::cycles::CycleTrackerQuery::page(page))
        }
    };
}
