//! Controlled accepted payments and immutable budget grants through the installed adapter.

mod budget;
mod liquidity;
mod uncertain;

use super::{dispatch, funding, intent, preparation, renewal};
use ic_blob_storage::{
    model::billing::journal::{FundingIntent, renewal::FundingRenewalError},
    ops::{
        caffeine::funding::transport::CashierTopUpStatus, service::funding::FundingJournalError,
    },
    workflow::funding::{FundingPreparationResult, dispatch::FundingDispatchResult},
};

fn payment(operation: u128) -> FundingIntent {
    FundingIntent {
        operation: operation.try_into().unwrap(),
        ..intent()
    }
}

#[canic::canic_raw_update(public, payload(max_bytes = 0))]
async fn test_funding_paid() {
    for operation in [3, 4] {
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
            panic!("expected accepted callback")
        };
        assert!(matches!(
            observation.status(),
            CashierTopUpStatus::Replied(Ok(
                ic_blob_storage::ops::caffeine::funding::TopUpReply::ReportedSuccess { .. }
            ))
        ));
        assert_eq!(observation.transfer().accepted(), Some(75));
        assert_eq!(observation.transfer().refunded(), Some(25));
        assert_eq!(
            funding::renew(renewal(input)),
            Err(FundingJournalError::Renewal(FundingRenewalError::Amount))
        );
        let grant = ic_blob_storage::model::billing::journal::renewal::FundingBudgetRenewal {
            additional: 75.try_into().unwrap(),
            ..renewal(input)
        };
        assert_eq!(funding::renew(grant), Ok(true));
        assert_eq!(funding::renew(grant), Ok(false));
    }
    ic_platform::cdk::api::msg_reply([]);
}

#[canic::canic_raw_update(public, payload(max_bytes = 0))]
async fn test_funding_paid_replay() {
    for operation in [3, 4] {
        let input = payment(operation);
        assert!(matches!(
            funding::dispatch(input, || dispatch(input, true))
                .await
                .unwrap(),
            FundingDispatchResult::Blocked { .. }
        ));
        let grant = ic_blob_storage::model::billing::journal::renewal::FundingBudgetRenewal {
            additional: 75.try_into().unwrap(),
            ..renewal(input)
        };
        assert_eq!(funding::renew(grant), Ok(false));
    }
    ic_platform::cdk::api::msg_reply([]);
}
