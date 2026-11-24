//! A hub pays for a child only under an explicitly requester-funded grant.
use super::{Key, Placement};
use crate::method::Contract;
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};

/// Original logical allocation selected by the application.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub enum Demand {
    /// Dedicated project key under the caller's registry.
    Instance(Key),
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

/// First request obtains a quote; subsequent calls settle or recover that request.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Payment {
    /// Root has reserved a source for this request until the pool grant expires.
    Quoted(Quote),
    /// A recovered or newly reserved placement and any charge for its creation.
    Placed(Placement, Option<Quote>),
}

/// One retained requester-funded placement. The maximum debit is immutable while quoted.
pub struct FundPlacement;
impl Contract for FundPlacement {
    const NAME: &'static str = "fund_placement";
    const ID: u64 = ic_wire::method_id("canic.placement.fund_placement.v1");
    const REQUEST: ic_wire::Limits = super::LIMITS;
    type Input = (Demand, u128);
    type Output = Result<Payment, String>;
}

/// Request or settle an allocation on the installed routing Root. Attach zero to obtain the quote.
/// # Errors
/// The caller retains the original demand and maximum across transport uncertainty.
pub async fn fund(
    root: Principal,
    demand: Demand,
    maximum: u128,
    attached: u128,
) -> Result<Payment, String> {
    if attached > maximum {
        return Err("attachment exceeds allocation maximum".into());
    }
    crate::method::call_with_cycles::<FundPlacement>(root, &(demand, maximum), attached)
        .await
        .map_err(|e| format!("funded placement: {e:?}"))?
}
