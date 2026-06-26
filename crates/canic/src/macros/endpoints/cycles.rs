//! Module: macros::endpoints::cycles
//!
//! Responsibility: emit cycle-tracker endpoint macros for consumer canisters.
//! Does not own: cycle accounting state or page query semantics.
//! Boundary: exposes facade macros that delegate immediately to core APIs.

/// Emit the standard cycle-tracker query endpoints.
#[macro_export]
macro_rules! canic_emit_cycle_tracker_endpoints {
    () => {
        #[$crate::canic_query(public)]
        fn canic_cycle_tracker(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::cycles::CycleTrackerEntry>, ::canic::Error>
        {
            Ok($crate::__internal::core::api::cycles::CycleTrackerQuery::page(page))
        }

        #[$crate::canic_query(public)]
        fn canic_cycle_topups(
            page: ::canic::dto::page::PageRequest,
        ) -> Result<::canic::dto::page::Page<::canic::dto::cycles::CycleTopupEvent>, ::canic::Error>
        {
            Ok($crate::__internal::core::api::cycles::CycleTrackerQuery::topups(page))
        }
    };
}
