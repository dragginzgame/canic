//! Acknowledgment does not extend the installed ceiling or replenish lifetime slots.

use super::{CashierTopUpStatus, FundingDispatchResult, FundingPreparationResult, payment};
use crate::funding::{dispatch, funding, preparation, renewal};
use ic_blob_storage::{
    model::billing::{allocation::FundingAllocationError, journal::renewal::FundingBudgetRenewal},
    ops::{caffeine::funding::TopUpReply, service::funding::FundingJournalError},
    policy::billing::admission::journal::FundingPreparationBlocker,
};

#[canic::canic_raw_update(public, payload(max_bytes = 0))]
async fn test_funding_budget() {
    for operation in [5, 6] {
        let input = payment(operation);
        assert!(matches!(
            funding::prepare(input, || preparation(input, true)).unwrap(),
            FundingPreparationResult::Prepared(_)
        ));
        let FundingDispatchResult::Settled { observation, .. } =
            funding::dispatch(input, || dispatch(input, true))
                .await
                .unwrap()
        else {
            panic!("expected exact acknowledged payment")
        };
        assert!(matches!(
            observation.status(),
            CashierTopUpStatus::Replied(Ok(TopUpReply::ReportedSuccess { .. }))
        ));
        assert_eq!(observation.transfer().accepted(), Some(75));
        assert_eq!(observation.transfer().refunded(), Some(25));
        ceiling(operation);
    }
    exhausted();
    ic_platform::cdk::api::msg_reply([]);
}

fn ceiling(operation: u128) {
    assert_eq!(
        funding::renew(FundingBudgetRenewal {
            additional: 1.try_into().unwrap(),
            ..renewal(payment(operation))
        }),
        Err(FundingJournalError::Allocation(
            FundingAllocationError::RenewalCeiling
        ))
    );
}

fn exhausted() {
    let input = payment(7);
    let FundingPreparationResult::Blocked(report) =
        funding::prepare(input, || preparation(input, true)).unwrap()
    else {
        panic!("lifetime capacity must not be replenished by renewal")
    };
    assert_eq!(report.blockers(), &[FundingPreparationBlocker::JournalFull]);
}

#[canic::canic_raw_update(public, payload(max_bytes = 0))]
async fn test_funding_budget_replay() {
    ceiling(6);
    exhausted();
    for operation in [3, 4] {
        assert_eq!(
            funding::renew(FundingBudgetRenewal {
                additional: 75.try_into().unwrap(),
                ..renewal(payment(operation))
            }),
            Ok(false)
        );
    }
    for operation in [5, 6] {
        let input = payment(operation);
        assert!(matches!(
            funding::dispatch(input, || dispatch(input, true))
                .await
                .unwrap(),
            FundingDispatchResult::Blocked { .. }
        ));
    }
    ic_platform::cdk::api::msg_reply([]);
}
