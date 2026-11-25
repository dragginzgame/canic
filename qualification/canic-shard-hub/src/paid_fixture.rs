//! Installed qualification driver for a controller-triggered placement by this canister.
use canic::sharding::{
    Placement,
    payment::{self, AcquireError, Demand, Payment, Refusal},
};
use ic_platform_types::Principal;

// acquire_fixture
// Spend the fixture's own cycles on one exact demand, recovering existing results for free.
// guard  : actual fixture controller
// limits : placement Wire limits
#[canic::update(guard = canic::guard::require_controller, limits = canic::sharding::LIMITS)]
async fn acquire_fixture((root, demand, maximum): (Principal, Demand, u128)) -> Placement {
    let quote = payment::acquire_placement(root, demand.clone(), maximum, 0).await;
    let result = match quote {
        Ok(Payment::Quoted(q)) => {
            payment::acquire_placement(root, demand, maximum, q.working_cycles + q.creation_fee)
                .await
        }
        other => other,
    };
    match result {
        Ok(Payment::Placed(placement, _)) => placement,
        Err(AcquireError::Refused(Refusal::SubnetFull)) => Placement::Full,
        Err(AcquireError::Refused(Refusal::QuoteConflict)) | Ok(Payment::Quoted(_)) => {
            Placement::Busy
        }
        Err(AcquireError::Refused(Refusal::BudgetExhausted | Refusal::MaximumTooLow)) => {
            Placement::GrowthRefused
        }
        other => panic!("fixture acquisition failed: {other:?}"),
    }
}
