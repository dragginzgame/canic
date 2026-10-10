//! Passive protected observability transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::page::Page;
use candid::{CandidType, Principal};
use serde::Deserialize;

/// Bounded RootStatusRequestFragment wire projection for protected observability.
#[derive(CandidType)]
pub enum RootStatusRequestFragment {
    ChildFunding(Principal),
    CycleBalance,
    CycleHistory(crate::dto::page::PageRequest),
    MemoryAllocations,
    Metrics(crate::dto::role::MetricsStatusRequest),
}

/// Bounded RootStatusResponseFragment wire projection for protected observability.
#[derive(CandidType, Deserialize)]
pub enum RootStatusResponseFragment {
    ChildFunding(crate::dto::observability::ChildFundingUsage),
    CycleBalance(crate::dto::role::CycleBalanceStatusResponse),
    CycleHistory(Page<crate::dto::cycles::CycleTrackerEntry>),
    MemoryAllocations(crate::dto::memory::MemoryAllocationsResponse),
    Metrics(Page<crate::dto::metrics::MetricEntry>),
}

/// Bounded StoreStatusRequestFragment wire projection for protected observability.
#[derive(CandidType)]
pub enum StoreStatusRequestFragment {
    CycleHistory(crate::dto::page::PageRequest),
}

/// Bounded StoreStatusResponseFragment wire projection for protected observability.
#[derive(CandidType, Deserialize)]
pub enum StoreStatusResponseFragment {
    CycleHistory(Page<crate::dto::cycles::CycleTrackerEntry>),
}
