//! Native state mutation receipts; stored state and cascade execution retain their owners.
use crate::dto::{cascade::StateCascadeTargetResult, error::Error, state::SetStateResponse};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Exact target and observed cascade outcome; unconfirmed is not a proven refusal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TargetOutcome {
    pub canister_id: Principal,
    pub result: StateCascadeTargetResult,
}

/// Bounded detail with complete aggregate counts.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CascadeReport {
    pub successful_targets: u64,
    pub reconciliation_failures: u64,
    pub unconfirmed_targets: u64,
    pub omitted_targets: u64,
    pub targets: Vec<TargetOutcome>,
}

/// Local mutation result and independently observed downstream progress.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CommandResult<T> {
    pub change: SetStateResponse<T>,
    pub propagation: CascadeReport,
    pub reconciliation_error: Option<Error>,
}
