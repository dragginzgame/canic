//! Module: dto::observability
//!
//! Responsibility: carry exact controller-owned runtime observations across Canic boundaries.
//! Does not own: authorization, observation lookup, or Fleet routing.
//! Boundary: sensitive cycle and performance values use one shared bounded transport contract.

use crate::dto::{
    child_funding::ChildFundingUsage,
    cycle_topups::CycleTopupEvent,
    cycles::CycleTrackerEntry,
    memory::MemoryAllocationsResponse,
    metric_page::MetricEntry,
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
    PortablePolicy,
    PortableSessions(crate::dto::portable_inspection::Request),
}

/// Exact sensitive runtime observation returned to an authenticated controller path.

pub enum CanisterObservabilityResponse {
    ChildFunding(ChildFundingUsage),
    CycleBalance(CycleBalanceStatusResponse),
    CycleHistory(Page<CycleTrackerEntry>),
    CycleTopups(Page<CycleTopupEvent>),
    MemoryAllocations(MemoryAllocationsResponse),
    Metrics(Page<MetricEntry>),
    PortablePolicy(
        Result<
            crate::dto::portable_inspection::PolicyReport,
            crate::dto::portable_inspection::Refusal,
        >,
    ),
    PortableSessions(
        Result<crate::dto::portable_inspection::Page, crate::dto::portable_inspection::Refusal>,
    ),
}
