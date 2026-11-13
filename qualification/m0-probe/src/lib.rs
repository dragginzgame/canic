//! Test-only management-effect probes and their bounded Wire protocol.
//!
//! This is a single-attempt experiment, not the Canic runtime. The maintenance
//! rehearsal preserves both slots as separately bounded stable-memory frames.

use serde::{Deserialize, Serialize};

#[cfg(target_family = "wasm")]
mod canister;
#[cfg(test)]
mod tests;

/// Wire identity of the sole creation attempt.
pub const START: u64 = 1;
/// Wire identity of the read-only evidence endpoint.
pub const STATUS: u64 = 2;
/// Wire identity of the private subsequent-work endpoint.
pub const LATER_WORK: u64 = 3;
/// Wire identity of the sole direct-cycle funding attempt.
pub const FUND: u64 = 4;
/// Wire identity of the funding evidence endpoint.
pub const FUNDING_STATUS: u64 = 5;
/// Wire identity of the private funding continuation.
pub const FUNDING_LATER: u64 = 6;
/// Wire identity of the upgrade argument: whether to trap after restoring state.
pub const UPGRADE: u64 = 8;
/// Wire identity of the executable marker, independent of the checkpoint format.
pub const EXECUTABLE: u64 = 9;
/// Maximum synthetic cycles admitted for one direct deposit.
pub const MAX_FUNDING_CYCLES: u128 = 1_000_000_000_000;
/// Finite limits for every probe request and reply.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 4096,
    allocation: 32768,
    nodes: 4096,
    depth: 16,
};
/// Maximum retained external request, leaving room for evidence in a status frame.
pub const MAX_MANAGEMENT_REQUEST_BYTES: usize = 1024;
/// Synthetic cycles attached to the one allowed creation attempt.
pub const CREATION_CYCLES: u128 = 2_000_000_000_000;

/// The message execution in which the experiment deliberately traps.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Fault {
    /// Complete the effect and return its retained result.
    None,
    /// Poll the call to enqueue it, then trap before that execution commits.
    Dispatch,
    /// Decode the returned evidence, then trap before recording it.
    BeforeResultWrite,
    /// Record the evidence, then trap in that same callback execution.
    AfterResultWrite,
    /// Commit the evidence, then fail in a separate self-call execution.
    LaterWork,
}

/// Committed state of the one experiment slot.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Phase {
    /// No intent or dispatch has committed.
    Empty,
    /// An attempt may have issued; repeating it is prohibited.
    Issued,
    /// The outcome and matching refund observation have committed.
    Recorded,
}

/// Evidence observable without issuing another creation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Status {
    /// Current committed phase.
    pub phase: Phase,
    /// Admitted fault selection, also binding duplicate commands.
    pub fault: Option<Fault>,
    /// Number of admitted attempts; never reset after an uncertain result.
    pub attempts: u8,
    /// Exact external management request retained at dispatch.
    #[serde(with = "serde_bytes")]
    pub request: Vec<u8>,
    /// Attached-cycle exposure, retained until result recording commits.
    pub reserved_cycles: u128,
    /// Returned principal bytes, absent when the result was not committed.
    pub child: Option<Vec<u8>>,
    /// Matching-callback refund observation, captured before any further await.
    pub refunded_cycles: Option<u128>,
    /// Written by subsequent work; a trap must roll this write back.
    pub later_work_entered: bool,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            phase: Phase::Empty,
            fault: None,
            attempts: 0,
            request: Vec::new(),
            reserved_cycles: 0,
            child: None,
            refunded_cycles: None,
            later_work_entered: false,
        }
    }
}

/// Exact direct-deposit intent; a replay must match every field.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FundingRequest {
    /// Destination principal bytes, validated before admission.
    #[serde(with = "serde_bytes")]
    pub target: Vec<u8>,
    /// Cycles to attach, bounded by [`MAX_FUNDING_CYCLES`].
    pub cycles: u128,
    /// Deliberate interruption window.
    pub fault: Fault,
}

/// Evidence from the matching unbounded management callback.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FundingOutcome {
    /// The management method replied successfully with the full deposit accepted.
    Deposited,
    /// The callback rejected; refund evidence is retained separately.
    Rejected {
        /// Raw reject code, including codes unknown to this executable.
        code: u32,
    },
}

/// Independent single-attempt slot for the direct-cycle funding experiment.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FundingStatus {
    /// Current committed phase.
    pub phase: Phase,
    /// Exact destination, attachment and fault bound to this attempt.
    pub intent: Option<FundingRequest>,
    /// Number of admitted attempts, never replenished after uncertainty.
    pub attempts: u8,
    /// Exact external management request retained before dispatch.
    #[serde(with = "serde_bytes")]
    pub request: Vec<u8>,
    /// Unresolved attached-cycle exposure, never inferred from balance changes.
    pub reserved_cycles: u128,
    /// Callback outcome, absent until its recording commits.
    pub outcome: Option<FundingOutcome>,
    /// Matching-callback refund, captured before subsequent work.
    pub refunded_cycles: Option<u128>,
    /// Subsequent-work marker whose write must roll back on a trap.
    pub later_work_entered: bool,
}

impl Default for FundingStatus {
    fn default() -> Self {
        Self {
            phase: Phase::Empty,
            intent: None,
            attempts: 0,
            request: Vec::new(),
            reserved_cycles: 0,
            outcome: None,
            refunded_cycles: None,
            later_work_entered: false,
        }
    }
}
