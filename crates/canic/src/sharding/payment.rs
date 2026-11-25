//! A hub pays for a child under its approved placement grant.
use super::Placement;
use crate::method::{CallError, Contract};
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};

/// Original logical allocation selected by the application.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub enum Demand {
    /// Opaque creation request under the calling registry’s grant.
    Instance([u8; 32]),
    /// Named worker pool and ordinal.
    Worker(String, u8),
    /// A whole shared shard for the hub’s locally retained routes.
    ShardCapacity(String, u8),
    /// One child of this role belonging to the calling registry instance.
    Singleton(String),
}

/// Exact quote retained by Root for the original allocation.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Quote {
    /// Current same-subnet create fee at quotation.
    pub creation_fee: u128,
    /// Operating balance included in the charge.
    pub working_cycles: u128,
    /// Reserved principal, or a fresh create on a reserve miss.
    pub spare: Option<Principal>,
}

/// A funded acquisition can settle immediately. A quote retains the terms for settlement.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Payment {
    /// Root has reserved a source for this request until the pool grant expires.
    Quoted(Quote),
    /// A recovered or newly reserved placement, its original charge .
    Placed(Placement, Option<Quote>),
}

/// Root's refusal of a funded placement attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
pub enum Refusal {
    /// The Root requires an explicit capacity approval.
    MissingCapacity,
    /// The requested pool or singleton has no grant.
    UnknownGrant,
    /// The caller does not own the grant.
    Unauthorized,
    /// The approved ceiling cannot cover a fresh canister.
    MaximumTooLow,
    /// The retained payment binds a different payer or ceiling.
    PaymentMismatch,
    /// A different request or maximum already holds the quote.
    QuoteConflict,
    /// The grant has insufficient creation budget.
    BudgetExhausted,
    /// Root could not verify the reserved spare.
    SpareUnverified,
    /// The observed quote has expired or been superseded.
    QuoteExpired,
    /// No additional subnet slot is available.
    SubnetFull,
    /// The quote total exceeds the cycle amount representation.
    ChargeOverflow,
    /// Attached cycles do not cover the retained quote.
    InsufficientPayment,
    /// The grant deadline has passed.
    GrantExpired,
    /// A new quote requires a positive debit ceiling.
    ZeroMaximum,
}

/// Failure of a single client attempt.
#[derive(Debug)]
pub enum AcquireError {
    /// The client did not submit cycles above its own ceiling.
    AttachmentExceedsMaximum,
    /// Submission, delivery or decoding failed at the recorded stage.
    Call(CallError),
    /// Root returned a decoded refusal.
    Refused(Refusal),
}

/// One retained requester-funded placement. The maximum debit is immutable while quoted.
pub struct AcquirePlacement;
impl Contract for AcquirePlacement {
    const NAME: &'static str = "acquire_placement";
    const ID: u64 = ic_wire::method_id("canic.root.acquire_placement.v1");
    const REQUEST: ic_wire::Limits = super::LIMITS;
    type Input = (Demand, u128);
    type Output = Result<Payment, Refusal>;
}

/// Request or settle an allocation on the installed routing Root. Attach zero to obtain the quote.
/// # Errors
/// The caller retains the original demand and maximum across transport uncertainty.
pub async fn acquire_placement(
    root: Principal,
    demand: Demand,
    maximum: u128,
    attached: u128,
) -> Result<Payment, AcquireError> {
    if attached > maximum {
        return Err(AcquireError::AttachmentExceedsMaximum);
    }
    crate::method::call_with_cycles::<AcquirePlacement>(root, &(demand, maximum), attached)
        .await
        .map_err(AcquireError::Call)?
        .map_err(AcquireError::Refused)
}
