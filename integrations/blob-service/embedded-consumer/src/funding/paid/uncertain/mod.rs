//! Accepted malformed callbacks remain spent and cannot authorize another payment.

use super::{CashierTopUpStatus, FundingDispatchResult, FundingPreparationResult, payment};
use crate::funding::{dispatch, funding, preparation, renewal};
use ic_blob_storage::{
    model::billing::journal::renewal::FundingRenewalError,
    ops::service::funding::FundingJournalError,
};

#[canic::canic_raw_update(public, payload(max_bytes = 0))]
async fn test_funding_uncertain() {
    execute(false).await;
    ic_platform::cdk::api::msg_reply([]);
}

#[canic::canic_raw_update(public, payload(max_bytes = 0))]
async fn test_funding_provider_error() {
    execute(true).await;
    ic_platform::cdk::api::msg_reply([]);
}

async fn execute(provider_error: bool) {
    let input = payment(5);
    assert!(matches!(
        funding::prepare(input, || preparation(input, true)).unwrap(),
        FundingPreparationResult::Prepared(_)
    ));
    let FundingDispatchResult::Settled { observation, .. } =
        funding::dispatch(input, || dispatch(input, true))
            .await
            .unwrap()
    else {
        panic!("expected original callback")
    };
    if provider_error {
        use ic_blob_storage::ops::caffeine::funding::{TopUpProviderError, TopUpReply};
        assert_eq!(
            observation.status(),
            CashierTopUpStatus::Replied(Ok(TopUpReply::ProviderFailure(
                TopUpProviderError::AccountBalanceOverflow
            )))
        );
    } else {
        assert!(matches!(
            observation.status(),
            CashierTopUpStatus::Replied(Err(_))
        ));
    }
    assert_eq!(observation.transfer().accepted(), Some(75));
    assert_eq!(observation.transfer().refunded(), Some(25));
    refuse().await;
}

async fn refuse() {
    let input = payment(5);
    assert_eq!(
        funding::renew(renewal(input)),
        Err(FundingJournalError::Renewal(
            FundingRenewalError::CreditRequired
        ))
    );
    blocked().await;
}

async fn blocked() {
    let input = payment(5);
    assert!(matches!(
        funding::dispatch(input, || dispatch(input, true))
            .await
            .unwrap(),
        FundingDispatchResult::Blocked { .. }
    ));
    let next = payment(6);
    assert!(matches!(
        funding::prepare(next, || preparation(next, true)).unwrap(),
        FundingPreparationResult::Blocked(_)
    ));
}

#[canic::canic_raw_update(public, payload(max_bytes = 0))]
async fn test_funding_uncertain_replay() {
    refuse().await;
    ic_platform::cdk::api::msg_reply([]);
}

#[canic::canic_raw_update(public, payload(max_bytes = 0))]
async fn test_funding_uncertain_pending() {
    blocked().await;
    ic_platform::cdk::api::msg_reply([]);
}

/// Trap in the settlement callback after shared journal writes, rolling them back.
#[canic::canic_raw_update(public, payload(max_bytes = 0))]
async fn test_funding_callback_trap() {
    execute(true).await;
    ic_platform::cdk::api::trap("controlled settlement callback failure");
}
