//! Receipts for operating grants confirmed by a managed payer.
use crate::method::Contract;
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};

/// Last 64 confirmed payments across the payer's children are retained.
pub const RETAINED_RECEIPTS: usize = 64;

/// Successful deposit on the immutable payer/child edge identified in the report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FundingReceipt {
    /// Original attempt number, starting at one.
    pub sequence: u64,
    /// Payer's IC time at confirmation, in nanoseconds since the Unix epoch.
    pub confirmed_at_ns: u64,
    /// Whole cycles deposited.
    pub amount_cycles: u64,
}

/// Settlement history for one immutable operating grant.
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FundingReceipts {
    /// Principal that confirmed the deposits.
    pub payer: Principal,
    /// Principal that received the deposits.
    pub child: Principal,
    /// Earlier successful sequences through this number lack retained timestamps.
    pub unavailable_through: u64,
    /// The payer lacks a final outcome for this attempt.
    pub unresolved: Option<u64>,
    /// Retained successes in ascending sequence order, following the unavailable prefix.
    pub receipts: Vec<FundingReceipt>,
}

/// Replicated read at the payer. The IC caller selects its own grant.
/// An unknown caller has no report. Reading never dispatches a payment.
pub struct ObserveChildFunding;
impl Contract for ObserveChildFunding {
    const NAME: &'static str = "observe_child_funding";
    const ID: u64 = ic_wire::method_id("canic.owner.observe_child_funding.v1");
    type Input = ();
    type Output = Option<FundingReceipts>;
}
