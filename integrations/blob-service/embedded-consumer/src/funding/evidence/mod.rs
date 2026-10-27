//! Installed adapter refusals preserve the original reservation for fresh evidence.

use super::{FundingDispatchResult, FundingIntent, FundingJournalError, dispatch, funding};
use ic_blob_storage::policy::billing::{
    RecoveryState,
    admission::{
        FundingActivity, FundingAdmissionBlocker,
        attempt::{FundingAttemptBlocker, FundingAttemptEvidence},
        evidence::FundingEvidenceBlocker,
    },
};

pub(super) async fn check(input: FundingIntent) {
    let valid = dispatch(input, true);
    for (attempt, expected) in cases(valid.attempt) {
        let evidence = super::FundingDispatchEvidence { attempt, ..valid };
        let FundingDispatchResult::Blocked {
            attempt,
            holds_unknown,
        } = funding::dispatch(input, || evidence).await.unwrap()
        else {
            panic!("unsafe evidence must not mark or dispatch the reservation")
        };
        assert!(!holds_unknown);
        assert_eq!(attempt.blockers(), &[expected]);
    }
    // Complete attempt evidence cannot substitute missing account liability holds.
    let FundingDispatchResult::Blocked {
        attempt,
        holds_unknown,
    } = funding::dispatch(input, || super::FundingDispatchEvidence {
        holds: None,
        ..valid
    })
    .await
    .unwrap()
    else {
        panic!("unknown holds must not dispatch")
    };
    assert!(holds_unknown);
    assert!(attempt.blockers().is_empty());
    for intent in [
        FundingIntent {
            service: super::Principal::anonymous(),
            ..input
        },
        FundingIntent {
            cashier: super::Principal::anonymous(),
            ..input
        },
        FundingIntent {
            account: super::Principal::anonymous(),
            ..input
        },
        FundingIntent {
            namespace: 8.try_into().unwrap(),
            ..input
        },
        FundingIntent {
            operation: 99.try_into().unwrap(),
            ..input
        },
        FundingIntent {
            offered: 101.try_into().unwrap(),
            ..input
        },
        FundingIntent {
            target_balance: Some(200.try_into().unwrap()),
            ..input
        },
    ] {
        assert_eq!(
            funding::dispatch(input, || super::FundingDispatchEvidence {
                attempt: FundingAttemptEvidence {
                    intent,
                    ..valid.attempt
                },
                ..valid
            })
            .await,
            Err(FundingJournalError::Binding)
        );
    }
    // The caller next settles this same intent. Exact final totals and populated
    // inspection prove these refusals neither consumed it nor reserved another one.
}

fn cases(valid: FundingAttemptEvidence) -> Vec<(FundingAttemptEvidence, FundingAttemptBlocker)> {
    use FundingAttemptBlocker::{Evidence, ProviderUnqualified};
    use FundingEvidenceBlocker::{Admission, FundingUnknown, RecoveryUnknown, SpendabilityUnknown};

    vec![
        (
            FundingAttemptEvidence {
                provider_qualified: false,
                ..valid
            },
            ProviderUnqualified,
        ),
        (
            FundingAttemptEvidence {
                available_for_offer: None,
                ..valid
            },
            Evidence(SpendabilityUnknown),
        ),
        (
            FundingAttemptEvidence {
                recovery: None,
                ..valid
            },
            Evidence(RecoveryUnknown),
        ),
        (
            FundingAttemptEvidence {
                other_activity: None,
                ..valid
            },
            Evidence(FundingUnknown),
        ),
        (
            FundingAttemptEvidence {
                recovery: Some(RecoveryState::Fenced),
                ..valid
            },
            Evidence(Admission(FundingAdmissionBlocker::RecoveryFenced)),
        ),
        (
            FundingAttemptEvidence {
                other_activity: Some(FundingActivity::InProgress),
                ..valid
            },
            Evidence(Admission(FundingAdmissionBlocker::FundingInProgress)),
        ),
        (
            FundingAttemptEvidence {
                other_activity: Some(FundingActivity::Uncertain),
                ..valid
            },
            Evidence(Admission(FundingAdmissionBlocker::FundingUncertain)),
        ),
        (
            FundingAttemptEvidence {
                available_for_offer: Some(0),
                ..valid
            },
            Evidence(Admission(FundingAdmissionBlocker::ReserveWouldBeViolated {
                requested_cycles: valid.intent.offered,
                transferable_cycles: 0,
            })),
        ),
    ]
}
