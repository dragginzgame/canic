use super::Placement;
use crate::method::Contract;
#[cfg(any(feature = "storage", test))]
use crate::method::{self, CallError};
use ic_platform_types::Principal;

/// Maximum cycles attached by the standard placement clients for one child.
pub const ACQUISITION_CYCLES: u128 = 1_000_000_000_000;

/// Select an existing allocation under the calling hub's grant.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
pub enum Lookup {
    /// Dedicated registry claim.
    Instance([u8; 32]),
    /// Named worker pool and ordinal.
    Worker(String, u8),
}

/// Read an existing allocation without reserving a quote or accepting cycles.
pub struct LookupPlacement;

impl Contract for LookupPlacement {
    const NAME: &'static str = "lookup_placement";
    const ID: u64 = ic_wire::method_id("canic.root.lookup_placement.v1");
    const REQUEST: ic_wire::Limits = super::LIMITS;
    type Input = Lookup;
    type Output = Placement;
}

/// Request creation or recover the same payment for this registry request.
#[cfg(any(feature = "storage", test))]
pub(super) async fn acquire_instance(
    root: Principal,
    request: [u8; 32],
) -> Result<Placement, super::payment::AcquireError> {
    use super::payment::{Demand, Payment, acquire_placement};
    match acquire_placement(
        root,
        Demand::Instance(request),
        ACQUISITION_CYCLES,
        ACQUISITION_CYCLES,
    )
    .await
    {
        Ok(Payment::Placed(placement, _)) => Ok(placement),
        Ok(Payment::Quoted(_))
        | Err(super::payment::AcquireError::Refused(super::payment::Refusal::QuoteConflict)) => {
            Ok(Placement::Busy)
        }
        Err(super::payment::AcquireError::Refused(super::payment::Refusal::SubnetFull)) => {
            Ok(Placement::Full)
        }
        Err(super::payment::AcquireError::Refused(
            super::payment::Refusal::BudgetExhausted | super::payment::Refusal::GrantExpired,
        )) => Ok(Placement::GrowthRefused),
        Err(error) => Err(error),
    }
}

/// Look up the original project claim without allocating.
/// # Errors
/// Returns an error for a missing or invalid reply.
#[cfg(any(feature = "storage", test))]
pub(super) async fn observe_instance(
    root: Principal,
    request: [u8; 32],
) -> Result<Placement, CallError> {
    method::call::<LookupPlacement>(root, 10, &Lookup::Instance(request)).await
}

/// Recover the same ordinal after reply loss.
/// # Errors
/// A transport or framing error leaves the original request unresolved.
pub async fn grow_shard(
    root: Principal,
    pool: &str,
    ordinal: u8,
) -> Result<Placement, super::payment::AcquireError> {
    use super::payment::{Demand, Payment, acquire_placement};
    match acquire_placement(
        root,
        Demand::ShardCapacity(pool.to_owned(), ordinal),
        ACQUISITION_CYCLES,
        ACQUISITION_CYCLES,
    )
    .await
    {
        Ok(Payment::Placed(placement, _)) => Ok(placement),
        Err(super::payment::AcquireError::Refused(
            super::payment::Refusal::BudgetExhausted | super::payment::Refusal::GrantExpired,
        )) => Ok(Placement::GrowthRefused),
        Err(super::payment::AcquireError::Refused(super::payment::Refusal::SubnetFull)) => {
            Ok(Placement::Full)
        }
        Err(error) => Err(error),
        Ok(Payment::Quoted(_)) => Ok(Placement::Busy),
    }
}
