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
#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum CanisterObservabilityRequest {
    ChildFunding(Principal),
    CycleBalance,
    CycleHistory(PageRequest),
    CycleTopups(PageRequest),
    MemoryAllocations,
    Metrics(MetricsStatusRequest),
}

/// Exact sensitive runtime observation returned to an authenticated controller path.
#[derive(CandidType, Deserialize)]
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
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct ChildFundingUsage {
    pub parent: Principal,
    pub child: Principal,
    pub observed_at_ns: u64,
    pub accounted_cycles: crate::cdk::types::Cycles,
    pub last_accounted_at_secs: u64,
    pub pending_operations: u32,
    pub reserved_cycles: Option<crate::cdk::types::Cycles>,
}

/// Root-routed request for one Root-controlled canister's sensitive observations.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct FleetCanisterObservabilityRequest {
    pub canister_id: Principal,
    pub request: CanisterObservabilityRequest,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fleet_observability_request_round_trips_through_candid() {
        let request = FleetCanisterObservabilityRequest {
            canister_id: Principal::from_slice(&[7; 29]),
            request: CanisterObservabilityRequest::CycleHistory(PageRequest {
                offset: 3,
                limit: 5,
            }),
        };

        let bytes = candid::encode_one(&request).expect("encode observability request");
        let decoded: FleetCanisterObservabilityRequest =
            candid::decode_one(&bytes).expect("decode observability request");

        assert_eq!(decoded.canister_id, request.canister_id);
        let CanisterObservabilityRequest::CycleHistory(page) = decoded.request else {
            panic!("expected CycleHistory request");
        };
        assert_eq!(page.offset, 3);
        assert_eq!(page.limit, 5);
    }
}
