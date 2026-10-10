//! Module: dto::wire::relay
//!
//! Responsibility: own the relay wire union declarations.
//! Does not own: endpoint dispatch, authority, state, or capability resolution.
//! Boundary: ordinary consumers use the complete types; endpoint emitters select
//! the same declarations with the build-resolved capability attributes.

/// Emit the relay contract with an explicit capability selection.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_relay_wire_types {
    (
        serde_crate = $serde_crate:literal,
        history_attributes = [$($history_attributes:tt)*],
        topup_history_attributes = [$($topup_history_attributes:tt)*],
        diagnostics_attributes = [$($diagnostics_attributes:tt)*],
        metrics_attributes = [$($metrics_attributes:tt)*],
    ) => {
        #[derive(
            $crate::__reexports::candid::CandidType, $crate::__reexports::serde::Deserialize,
        )]
        #[serde(crate = $serde_crate)]
        /// RelayedObservabilityResponse wire union selected from the role's compiled capabilities.
        pub enum RelayedObservabilityResponse {
            ChildFunding($crate::dto::observability::ChildFundingUsage),
            CycleBalance($crate::dto::role::CycleBalanceStatusResponse),
            $($history_attributes)*
            CycleHistory($crate::dto::page::Page<$crate::dto::cycles::CycleTrackerEntry>),
            $($topup_history_attributes)*
            CycleTopups($crate::dto::page::Page<$crate::dto::cycles::CycleTopupEvent>),
            $($diagnostics_attributes)*
            MemoryAllocations($crate::dto::memory::MemoryAllocationsResponse),
            $($metrics_attributes)*
            Metrics($crate::dto::page::Page<$crate::dto::metrics::MetricEntry>),
        }
    };
}

crate::__canic_relay_wire_types! {
    serde_crate = "serde",
    history_attributes = [],
    topup_history_attributes = [],
    diagnostics_attributes = [],
    metrics_attributes = [],
}
