//! Module: dto::observability
//!
//! Responsibility: carry exact controller-owned runtime observations across Canic boundaries.
//! Does not own: authorization, observation lookup, or Fleet routing.
//! Boundary: sensitive cycle and performance values use one shared bounded transport contract.

use crate::dto::{
    cycles::{CycleTopupEvent, CycleTrackerEntry},
    metrics::MetricEntry,
    page::{Page, PageRequest},
    prelude::*,
    role::{CycleBalanceStatusResponse, MetricsStatusRequest},
};

/// Exact sensitive runtime observation selected by an authenticated controller path.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum CanisterObservabilityRequest {
    CycleBalance,
    CycleHistory(PageRequest),
    CycleTopups(PageRequest),
    Metrics(MetricsStatusRequest),
}

/// Exact sensitive runtime observation returned to an authenticated controller path.
#[derive(CandidType, Deserialize)]
pub enum CanisterObservabilityResponse {
    CycleBalance(CycleBalanceStatusResponse),
    CycleHistory(Page<CycleTrackerEntry>),
    CycleTopups(Page<CycleTopupEvent>),
    Metrics(Page<MetricEntry>),
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
