//! Module: dto::observability
//!
//! Responsibility: carry exact controller-owned runtime observations across Canic boundaries.
//! Does not own: authorization, observation lookup, or Fleet routing.
//! Boundary: sensitive cycle and performance values use one shared bounded transport contract.

use crate::dto::{
    cycles::{CycleTopupEvent, CycleTrackerEntry},
    memory::MemoryAllocationsResponse,
    metrics::MetricEntry,
    page::{Page, PageRequest},
    prelude::*,
    role::{CycleBalanceStatusResponse, MetricsStatusRequest},
};

/// Exact sensitive runtime observation selected by an authenticated controller path.

#[derive(Clone, Debug)]
pub enum CanisterObservabilityRequest {
    ChildFunding(Principal),
    CycleBalance,
    CycleHistory(PageRequest),
    CycleTopups(PageRequest),
    MemoryAllocations,
    Metrics(MetricsStatusRequest),
}

/// Exact sensitive runtime observation returned to an authenticated controller path.

pub enum CanisterObservabilityResponse {
    ChildFunding(ChildFundingUsage),
    CycleBalance(CycleBalanceStatusResponse),
    CycleHistory(Page<CycleTrackerEntry>),
    CycleTopups(Page<CycleTopupEvent>),
    MemoryAllocations(MemoryAllocationsResponse),
    Metrics(Page<MetricEntry>),
}

/// Parent-local ledger and unresolved transfer evidence for one exact child.
/// Accounted cycles can include an in-flight grant; missing reservation evidence is unknown.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct ChildFundingUsage {
    pub parent: Principal,
    pub child: Principal,
    pub observed_at_ns: u64,
    pub accounted_cycles: crate::cycles::Cycles,
    pub last_accounted_at_secs: u64,
    pub pending_operations: u32,
    pub reserved_cycles: Option<crate::cycles::Cycles>,
}
