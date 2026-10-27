//! Controller-only automatic funding history on capable application roles.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_cycle_topups_endpoint {
    () => {
        #[$crate::canic_query(requires(caller::is_controller()), wire(id = ::canic::protocol::cycle_topups::METHOD_ID, request = ::canic::protocol::cycle_topups::REQUEST_LIMITS, response = ::canic::protocol::cycle_topups::RESPONSE_LIMITS))]
        fn canic_cycle_topups(page: ::canic::dto::page::PageRequest) -> Result<::canic::dto::page::Page<::canic::dto::cycle_topups::CycleTopupEvent>, ::canic::Error> {
            Ok($crate::__internal::core::api::cycles::CycleTrackerQuery::topups(page).into())
        }
    };
}
