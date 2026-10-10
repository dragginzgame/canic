#![expect(
    clippy::large_enum_variant,
    reason = "complete canonical role unions retain the existing inline wire payloads"
)]

//! Module: dto::wire::local
//!
//! Responsibility: own the local wire union declarations.
//! Does not own: endpoint dispatch, authority, state, or capability resolution.
//! Boundary: ordinary consumers use the complete types; endpoint emitters select
//! the same declarations with the build-resolved capability attributes.

/// Emit the local contract with an explicit capability selection.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_local_wire_types {
    (
        serde_crate = $serde_crate:literal,
        metrics_attributes = [$($metrics_attributes:tt)*],
        history_attributes = [$($history_attributes:tt)*],
        child_provisioning_attributes = [$($child_provisioning_attributes:tt)*],
        automatic_topup_attributes = [$($automatic_topup_attributes:tt)*],
        logs_attributes = [$($logs_attributes:tt)*],
        diagnostics_attributes = [$($diagnostics_attributes:tt)*],
    ) => {
        #[derive(
            $crate::__reexports::candid::CandidType, $crate::__reexports::serde::Deserialize,
        )]
        #[serde(crate = $serde_crate)]
        /// PublicStatusRequest wire union selected from the role's compiled capabilities.
        pub enum PublicStatusRequest {
            Health,
            $($metrics_attributes)*
            Metrics($crate::dto::public_status::PublicMetricsRequest),
            $($history_attributes)*
            History($crate::dto::public_status::PublicHistoryRequest),
            $($child_provisioning_attributes)*
            Children($crate::dto::page::PageRequest),
        }

#[derive(
            $crate::__reexports::candid::CandidType, $crate::__reexports::serde::Deserialize,
        )]
        #[serde(crate = $serde_crate)]
        /// PublicStatusResponse wire union selected from the role's compiled capabilities.
        pub enum PublicStatusResponse {
            Health($crate::dto::public_status::PublicHealth),
            $($metrics_attributes)*
            Metrics($crate::dto::public_status::PublicMetricsSnapshot),
            $($history_attributes)*
            History($crate::dto::public_status::PublicHistorySnapshot),
            $($child_provisioning_attributes)*
            Children($crate::dto::page::Page<$crate::dto::canister::CanisterInfo>),
        }

#[derive(
            $crate::__reexports::candid::CandidType, $crate::__reexports::serde::Deserialize,
        )]
        #[serde(crate = $serde_crate)]
        /// ObservabilityRequest wire union selected from the role's compiled capabilities.
        pub enum ObservabilityRequest {
            ChildFunding($crate::__reexports::candid::Principal),
            CycleBalance,
            $($history_attributes)*
            CycleHistory($crate::dto::page::PageRequest),
            $($automatic_topup_attributes)*
            $($history_attributes)*
            CycleTopups($crate::dto::page::PageRequest),
            Health,
            $($logs_attributes)*
            Logs($crate::dto::role::LogStatusRequest),
            $($metrics_attributes)*
            Metrics($crate::dto::role::MetricsStatusRequest),
            Readiness,
            $($diagnostics_attributes)*
            Runtime,
        }

#[derive(
            $crate::__reexports::candid::CandidType, $crate::__reexports::serde::Deserialize,
        )]
        #[serde(crate = $serde_crate)]
        /// ObservabilityResponse wire union selected from the role's compiled capabilities.
        pub enum ObservabilityResponse {
            ChildFunding($crate::dto::observability::ChildFundingUsage),
            CycleBalance($crate::dto::role::CycleBalanceStatusResponse),
            $($history_attributes)*
            CycleHistory($crate::dto::page::Page<$crate::dto::cycles::CycleTrackerEntry>),
            $($automatic_topup_attributes)*
            $($history_attributes)*
            CycleTopups($crate::dto::page::Page<$crate::dto::cycles::CycleTopupEvent>),
            Health($crate::dto::runtime::CanicHealthStatus),
            $($logs_attributes)*
            Logs($crate::dto::page::Page<$crate::dto::log::LogEntry>),
            $($metrics_attributes)*
            Metrics($crate::dto::page::Page<$crate::dto::metrics::MetricEntry>),
            Readiness($crate::dto::runtime::CanicReadinessStatus),
            $($diagnostics_attributes)*
            Runtime($crate::dto::runtime::CanicRuntimeStatus),
        }
    };
}

crate::__canic_local_wire_types! {
    serde_crate = "serde",
    metrics_attributes = [],
    history_attributes = [],
    child_provisioning_attributes = [],
    automatic_topup_attributes = [],
    logs_attributes = [],
    diagnostics_attributes = [],
}
