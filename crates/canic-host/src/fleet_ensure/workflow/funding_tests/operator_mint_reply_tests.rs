//! Persisted reply observations are recovery evidence, never cycle credit.

use super::{Fixture, operator_mint_tests::fixture};
use crate::fleet_ensure::{
    model::{
        FleetEnsureJournalRecord,
        operator_mint::{
            OperatorMintIntentRecord, OperatorMintNotificationOutcomeRecord,
            OperatorMintReviewRecord, OperatorMintTransferOutcomeRecord,
        },
    },
    ops::{operator_mint::notification_argument, read_journal, write_journal},
    workflow::{EnsureWorkflowError, operator_mint as workflow},
};
use candid::{CandidType, Nat, encode_one};
use std::fs;

#[derive(CandidType)]
enum TransferFailure {
    TxDuplicate { duplicate_of: u64 },
    TxTooOld { allowed_window_nanos: u64 },
}

#[derive(CandidType)]
struct NotificationSuccess {
    block_index: Nat,
    minted: Nat,
    balance: Nat,
}

#[derive(CandidType)]
enum NotificationFailure {
    Processing,
    TransactionTooOld(u64),
    Refunded {
        block_index: Option<u64>,
        reason: String,
    },
}

fn transfer(result: Result<u64, TransferFailure>) -> Vec<u8> {
    encode_one(result).unwrap()
}

fn notification(result: Result<NotificationSuccess, NotificationFailure>) -> Vec<u8> {
    encode_one(result).unwrap()
}

fn minted(block: u128) -> Vec<u8> {
    notification(Ok(NotificationSuccess {
        block_index: block.into(),
        minted: 1000u64.into(),
        balance: 9000u64.into(),
    }))
}

fn approved() -> (Fixture, OperatorMintIntentRecord, OperatorMintReviewRecord) {
    let (fixture, intent) = fixture();
    let reviewed = workflow::review(&fixture.paths, &intent).unwrap();
    let approved =
        workflow::approve(&fixture.paths, &intent.authority, &reviewed.review_sha256).unwrap();
    (fixture, intent, approved)
}

#[test]
fn operator_mint_replies_require_approval_notification_intent_and_exact_authority() {
    let (fixture, intent) = fixture();
    let reviewed = workflow::review(&fixture.paths, &intent).unwrap();
    let before = fs::read(&fixture.paths.journal).unwrap();
    assert!(matches!(
        workflow::record_transfer_reply(
            &fixture.paths,
            &intent.authority,
            &reviewed.review_sha256,
            &transfer(Ok(42))
        ),
        Err(EnsureWorkflowError::OperatorMintReviewConflict)
    ));
    assert!(matches!(
        workflow::prepare_notification(&fixture.paths, &intent.authority, &reviewed.review_sha256),
        Err(EnsureWorkflowError::OperatorMintReviewConflict)
    ));
    assert_eq!(fs::read(&fixture.paths.journal).unwrap(), before);
    workflow::approve(&fixture.paths, &intent.authority, &reviewed.review_sha256).unwrap();
    let before = fs::read(&fixture.paths.journal).unwrap();
    assert!(matches!(
        workflow::record_notification_reply(
            &fixture.paths,
            &intent.authority,
            &reviewed.review_sha256,
            &minted(20)
        ),
        Err(EnsureWorkflowError::OperatorMintReviewConflict)
    ));
    let mut changed = intent.authority;
    changed.network_identity_sha256[0] ^= 1;
    assert!(matches!(
        workflow::record_transfer_reply(
            &fixture.paths,
            &changed,
            &reviewed.review_sha256,
            &transfer(Ok(42))
        ),
        Err(EnsureWorkflowError::OperatorMintReviewConflict)
    ));
    assert!(matches!(
        workflow::prepare_notification(&fixture.paths, &changed, &reviewed.review_sha256),
        Err(EnsureWorkflowError::OperatorMintReviewConflict)
    ));
    assert!(matches!(
        workflow::record_notification_reply(
            &fixture.paths,
            &changed,
            &reviewed.review_sha256,
            &minted(20)
        ),
        Err(EnsureWorkflowError::OperatorMintReviewConflict)
    ));
    assert_eq!(fs::read(&fixture.paths.journal).unwrap(), before);
}

#[test]
fn operator_mint_reply_restart_preserves_locators_and_never_credits_a_cached_success() {
    let (mut fixture, intent, approved) = approved();
    let original = read_journal(&fixture.paths).unwrap().unwrap();
    let plan = fs::read(&fixture.paths.plan).unwrap();
    let state = fs::read(&fixture.paths.state).unwrap();
    let digest = &approved.review_sha256;
    // A lost reply leaves the approved bytes/time unchanged on restart.
    assert_eq!(
        workflow::approve(&fixture.paths, &intent.authority, digest).unwrap(),
        approved
    );
    workflow::record_transfer_reply(&fixture.paths, &intent.authority, digest, &transfer(Ok(42)))
        .unwrap();
    let before = fs::read(&fixture.paths.journal).unwrap();
    workflow::record_transfer_reply(
        &fixture.paths,
        &intent.authority,
        digest,
        &transfer(Err(TransferFailure::TxDuplicate { duplicate_of: 42 })),
    )
    .unwrap();
    assert_eq!(fs::read(&fixture.paths.journal).unwrap(), before);
    for reply in [
        transfer(Ok(43)),
        transfer(Err(TransferFailure::TxTooOld {
            allowed_window_nanos: 1,
        })),
    ] {
        assert!(matches!(
            workflow::record_transfer_reply(&fixture.paths, &intent.authority, digest, &reply),
            Err(EnsureWorkflowError::OperatorMintReplyConflict)
        ));
    }
    assert_eq!(fs::read(&fixture.paths.journal).unwrap(), before);
    let prepared =
        workflow::prepare_notification(&fixture.paths, &intent.authority, digest).unwrap();
    assert_eq!(
        prepared.notification.as_ref().unwrap().argument,
        notification_argument(&intent, 42).unwrap()
    );
    let before = fs::read(&fixture.paths.journal).unwrap();
    assert_eq!(
        workflow::prepare_notification(&fixture.paths, &intent.authority, digest).unwrap(),
        prepared
    );
    assert_eq!(fs::read(&fixture.paths.journal).unwrap(), before);
    workflow::record_notification_reply(
        &fixture.paths,
        &intent.authority,
        digest,
        &notification(Err(NotificationFailure::Processing)),
    )
    .unwrap();
    let finished = workflow::record_notification_reply(
        &fixture.paths,
        &intent.authority,
        digest,
        &minted(u128::MAX),
    )
    .unwrap();
    assert!(matches!(
        finished.notification.as_ref().unwrap().outcome,
        Some(OperatorMintNotificationOutcomeRecord::Minted {
            deposit_block_index: u128::MAX,
            gross_minted_cycles: 1000,
            historical_balance_cycles: 9000
        })
    ));
    let before = fs::read(&fixture.paths.journal).unwrap();
    assert_eq!(
        workflow::record_notification_reply(
            &fixture.paths,
            &intent.authority,
            digest,
            &minted(u128::MAX)
        )
        .unwrap(),
        finished
    );
    assert!(matches!(
        workflow::record_notification_reply(&fixture.paths, &intent.authority, digest, &minted(7)),
        Err(EnsureWorkflowError::OperatorMintReplyConflict)
    ));
    assert_eq!(fs::read(&fixture.paths.journal).unwrap(), before);
    let mut expected = original.clone();
    expected.funding_reviews[0].operator_mint = Some(finished);
    assert_eq!(read_journal(&fixture.paths).unwrap().unwrap(), expected);
    assert_eq!(fs::read(&fixture.paths.plan).unwrap(), plan);
    assert_eq!(fs::read(&fixture.paths.state).unwrap(), state);
    assert!(matches!(
        fixture.apply(&original.funding_reviews[0].review_sha256),
        Err(EnsureWorkflowError::OperatorMintPending { .. })
    ));
    assert!(fixture.platform.transfers.is_empty());
}

#[test]
fn operator_mint_expiry_and_unknown_replies_preserve_the_original_payment() {
    let (fixture, intent, approved) = approved();
    let retained = workflow::record_transfer_reply(
        &fixture.paths,
        &intent.authority,
        &approved.review_sha256,
        &transfer(Err(TransferFailure::TxTooOld {
            allowed_window_nanos: 1,
        })),
    )
    .unwrap();
    assert_eq!(retained.transfer_argument, approved.transfer_argument);
    assert_eq!(retained.intent, intent);
    assert_eq!(
        retained.transfer_outcome,
        Some(OperatorMintTransferOutcomeRecord::TooOld {
            allowed_window_ns: 1
        })
    );
    let before = fs::read(&fixture.paths.journal).unwrap();
    assert!(matches!(
        workflow::prepare_notification(&fixture.paths, &intent.authority, &approved.review_sha256),
        Err(EnsureWorkflowError::OperatorMintReviewConflict)
    ));
    assert!(matches!(
        workflow::record_transfer_reply(
            &fixture.paths,
            &intent.authority,
            &approved.review_sha256,
            b"invalid reply"
        ),
        Err(EnsureWorkflowError::OperatorMintWire(_))
    ));
    assert_eq!(
        workflow::approve(&fixture.paths, &intent.authority, &approved.review_sha256).unwrap(),
        retained
    );
    assert_eq!(fs::read(&fixture.paths.journal).unwrap(), before);
}

#[test]
fn operator_mint_purged_history_and_unattributed_refund_stay_uncredited() {
    let (mut fixture, intent, approved) = approved();
    workflow::record_transfer_reply(
        &fixture.paths,
        &intent.authority,
        &approved.review_sha256,
        &transfer(Err(TransferFailure::TxDuplicate { duplicate_of: 42 })),
    )
    .unwrap();
    workflow::prepare_notification(&fixture.paths, &intent.authority, &approved.review_sha256)
        .unwrap();
    let purged = workflow::record_notification_reply(
        &fixture.paths,
        &intent.authority,
        &approved.review_sha256,
        &notification(Err(NotificationFailure::TransactionTooOld(43))),
    )
    .unwrap();
    assert_eq!(
        purged.notification.unwrap().outcome,
        Some(OperatorMintNotificationOutcomeRecord::TransactionTooOld {
            oldest_block_index: 43
        })
    );
    let refund = notification(Err(NotificationFailure::Refunded {
        block_index: None,
        reason: "receipt unavailable".into(),
    }));
    let retained = workflow::record_notification_reply(
        &fixture.paths,
        &intent.authority,
        &approved.review_sha256,
        &refund,
    )
    .unwrap();
    assert!(matches!(
        retained.notification.unwrap().outcome,
        Some(OperatorMintNotificationOutcomeRecord::Refunded {
            refund_block_index: None,
            ..
        })
    ));
    let before = fs::read(&fixture.paths.journal).unwrap();
    assert!(matches!(
        workflow::record_notification_reply(
            &fixture.paths,
            &intent.authority,
            &approved.review_sha256,
            &minted(20)
        ),
        Err(EnsureWorkflowError::OperatorMintReplyConflict)
    ));
    let withdrawal_review = read_journal(&fixture.paths)
        .unwrap()
        .unwrap()
        .funding_reviews[0]
        .review_sha256
        .clone();
    assert!(matches!(
        fixture.apply(&withdrawal_review),
        Err(EnsureWorkflowError::OperatorMintPending { .. })
    ));
    assert!(fixture.platform.transfers.is_empty());
    assert_eq!(fs::read(&fixture.paths.journal).unwrap(), before);
}

#[test]
fn operator_mint_notification_corruption_and_missing_fields_reject_on_restart() {
    let (fixture, intent, approved) = approved();
    workflow::record_transfer_reply(
        &fixture.paths,
        &intent.authority,
        &approved.review_sha256,
        &transfer(Ok(42)),
    )
    .unwrap();
    workflow::prepare_notification(&fixture.paths, &intent.authority, &approved.review_sha256)
        .unwrap();
    let original = read_journal(&fixture.paths).unwrap().unwrap();
    for path in [
        "/funding_reviews/0/operator_mint",
        "/funding_reviews/0/operator_mint/notification",
    ] {
        let value = serde_json::to_value(&original).unwrap();
        for key in value.pointer(path).unwrap().as_object().unwrap().keys() {
            let mut changed = value.clone();
            changed
                .pointer_mut(path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(key);
            assert!(serde_json::from_value::<FleetEnsureJournalRecord>(changed).is_err());
        }
    }
    let mutations: [fn(&mut OperatorMintReviewRecord); 3] = [
        |r| r.notification.as_mut().unwrap().icp_block_index += 1,
        |r| r.notification.as_mut().unwrap().argument[0] ^= 1,
        |r| r.transfer_argument = None,
    ];
    for mutate in mutations {
        let mut changed = original.clone();
        mutate(changed.funding_reviews[0].operator_mint.as_mut().unwrap());
        write_journal(&fixture.paths, &changed).unwrap();
        assert!(matches!(
            workflow::approve(&fixture.paths, &intent.authority, &approved.review_sha256),
            Err(EnsureWorkflowError::JournalIntegrity)
        ));
    }
}
