// -----------------------------------------------------------------------------
// Cycle endpoint emitters
// -----------------------------------------------------------------------------

// Leaf emitter for the standard cycle-tracker view.
#[macro_export]
macro_rules! canic_emit_cycle_tracker_endpoints {
    () => {
        #[$crate::canic_query]
        fn canic_cycle_tracker(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::cycles::CycleTrackerEntry>, ::canic::Error>
        {
            Ok($crate::__internal::core::api::cycles::CycleTrackerQuery::page(page))
        }
    };
}
