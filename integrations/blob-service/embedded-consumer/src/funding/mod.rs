//! Installed acceptance for the opt-in host API in an isolated framework fixture.
//! This control endpoint accepts no evidence or payment instructions from ingress.

mod evidence;
mod paid;

use canic_blob_service::funding;
use ic_blob_storage::{
    model::billing::journal::{
        FundingIntent, FundingJournalScope,
        renewal::{FundingBudgetRenewal, FundingRenewalError},
    },
    ops::{
        caffeine::funding::transport::CashierTopUpStatus, service::funding::FundingJournalError,
    },
    policy::billing::{
        RecoveryState,
        admission::{
            FundingActivity, attempt::FundingAttemptEvidence, evidence::FundingAdmissionEvidence,
            journal::FundingHostEvidence,
        },
    },
    workflow::funding::{
        FundingPreparationResult,
        dispatch::{FundingDispatchEvidence, FundingDispatchHolds, FundingDispatchResult},
    },
};
use ic_platform::Principal;

fn intent() -> FundingIntent {
    FundingIntent {
        service: ic_platform::cdk::api::canister_self(),
        cashier: Principal::from_text("um5iw-rqaaa-aaaaq-qaaba-cai").unwrap(),
        account: Principal::from_slice(&[8; 29]),
        namespace: 7.try_into().unwrap(),
        operation: 1.try_into().unwrap(),
        offered: 100.try_into().unwrap(),
        target_balance: None,
    }
}

fn preparation(input: FundingIntent, known: bool) -> FundingHostEvidence {
    FundingHostEvidence {
        scope: FundingJournalScope {
            service: input.service,
            cashier: input.cashier,
            account: input.account,
            namespace: input.namespace,
        },
        provider_qualified: known,
        admission: FundingAdmissionEvidence {
            available_cycles: known.then(ic_platform::cdk::api::canister_cycle_balance),
            recovery: known.then_some(RecoveryState::Reconciled),
            activity: known.then_some(FundingActivity::Clear),
        },
    }
}

fn dispatch(input: FundingIntent, known: bool) -> FundingDispatchEvidence {
    FundingDispatchEvidence {
        attempt: FundingAttemptEvidence {
            intent: input,
            provider_qualified: known,
            available_for_offer: known.then(ic_platform::cdk::api::canister_cycle_balance),
            recovery: known.then_some(RecoveryState::Reconciled),
            other_activity: known.then_some(FundingActivity::Clear),
        },
        holds: known.then_some(FundingDispatchHolds {
            operating_reserve: 100_000_000.try_into().unwrap(),
            other_liabilities: 0,
        }),
    }
}

#[canic::canic_raw_update(public, payload(max_bytes = 0))]
async fn test_funding() {
    let input = intent();
    if ic_platform::cdk::api::msg_caller() != Principal::from_slice(&[7; 29]) {
        assert_eq!(
            funding::prepare(input, || panic!("unauthorized evidence callback")),
            Err(FundingJournalError::NotOperator)
        );
        assert_eq!(
            funding::dispatch(input, || panic!("unauthorized dispatch callback")).await,
            Err(FundingJournalError::NotOperator)
        );
        assert_eq!(
            funding::renew(renewal(input)),
            Err(FundingJournalError::NotOperator)
        );
    } else {
        qualify(input).await;
        qualify(FundingIntent {
            operation: 2.try_into().unwrap(),
            ..input
        })
        .await;
    }
    ic_platform::cdk::api::msg_reply([]);
}

async fn qualify(input: FundingIntent) {
    let wrong = FundingIntent {
        namespace: 8.try_into().unwrap(),
        ..input
    };
    assert_eq!(
        funding::prepare(wrong, || panic!("wrong-scope evidence callback")),
        Err(FundingJournalError::Binding)
    );
    assert!(matches!(
        funding::prepare(input, || preparation(input, false)).unwrap(),
        FundingPreparationResult::Blocked(_)
    ));
    assert!(matches!(
        funding::prepare(input, || preparation(input, true)).unwrap(),
        FundingPreparationResult::Prepared(_)
    ));
    assert!(matches!(
        funding::dispatch(input, || dispatch(input, false))
            .await
            .unwrap(),
        FundingDispatchResult::Blocked {
            holds_unknown: true,
            ..
        }
    ));
    evidence::check(input).await;
    // The first held reply is malformed; the second exhausts the provider slot.
    // Both refund all cycles. Neither result establishes provider credit.
    let FundingDispatchResult::Settled { observation, .. } =
        funding::dispatch(input, || dispatch(input, true))
            .await
            .unwrap()
    else {
        panic!("expected exact callback settlement")
    };
    if input.operation.get() == 1 {
        assert!(matches!(
            observation.status(),
            CashierTopUpStatus::Replied(Err(_))
        ));
    } else {
        assert!(matches!(
            observation.status(),
            CashierTopUpStatus::Rejected(_)
        ));
    }
    assert_eq!(observation.transfer().refunded(), Some(input.offered.get()));
    assert_eq!(observation.transfer().accepted(), Some(0));
    assert_eq!(
        funding::renew(renewal(input)),
        Err(FundingJournalError::Renewal(
            FundingRenewalError::CreditRequired
        ))
    );
    assert!(matches!(
        funding::dispatch(input, || dispatch(input, true))
            .await
            .unwrap(),
        FundingDispatchResult::Blocked { .. }
    ));
}

fn renewal(intent: FundingIntent) -> FundingBudgetRenewal {
    FundingBudgetRenewal {
        intent,
        additional: intent.offered,
    }
}

/// Test-only concurrent ingress while the first provider response remains held.
#[canic::canic_raw_update(public, payload(max_bytes = 0))]
async fn test_funding_pending() {
    assert_eq!(
        ic_platform::cdk::api::msg_caller(),
        Principal::from_slice(&[7; 29])
    );
    let original = intent();
    assert!(matches!(
        funding::prepare(original, || preparation(original, true)).unwrap(),
        FundingPreparationResult::Blocked(_)
    ));
    assert!(matches!(
        funding::dispatch(original, || dispatch(original, true))
            .await
            .unwrap(),
        FundingDispatchResult::Blocked { .. }
    ));
    let next = FundingIntent {
        operation: 2.try_into().unwrap(),
        ..original
    };
    assert!(matches!(
        funding::prepare(next, || preparation(next, true)).unwrap(),
        FundingPreparationResult::Blocked(_)
    ));
    ic_platform::cdk::api::msg_reply([]);
}
