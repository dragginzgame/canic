//! Conversion approval persistence inside the real retained Fleet journal.

use super::{Fixture, ROOT};
use crate::{
    fleet_ensure::{
        model::{
            FleetEnsureJournalRecord, FundingReviewRecord, ReviewedDesiredFleetRecord,
            operator_mint::{
                OperatorMintAuthority, OperatorMintIntentRecord,
                OperatorMintNotificationOutcomeRecord,
            },
        },
        ops::{
            funding as funding_records, operator_mint as mint_records,
            operator_mint::transport::OperatorMintTransport, read_journal, write_journal,
            write_plan,
        },
        policy::expected_plan_sha256,
        workflow::{EnsureWorkflowError, operator_mint},
    },
    test_support::start_pocket_ic,
};
use candid::{CandidType, Nat, Principal};
use canic_core::cdk::utils::hash::decode_hex;
use ic_agent::{Agent, identity::BasicIdentity};
use ic_testkit::{
    pic::{CandidCallExt, PocketIcBuilder},
    pocket_ic::common::rest::{IcpFeatures, IcpFeaturesConfig},
};
use serde::Deserialize;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(CandidType, Deserialize)]
struct MintTestAccount {
    owner: Principal,
    subaccount: Option<Vec<u8>>,
}

#[derive(CandidType)]
struct MintTestTransfer {
    from_subaccount: Option<Vec<u8>>,
    to: MintTestAccount,
    amount: Nat,
    fee: Option<Nat>,
    memo: Option<Vec<u8>>,
    created_at_time: Option<u64>,
}

#[test]
#[ignore = "governed PocketIC proof uses production ICP Ledger, CMC and Cycles Ledger"]
#[expect(
    clippy::too_many_lines,
    reason = "one governed production-Ledger journey keeps the three interruption boundaries, conserved credit and terminal replay together"
)]
fn governed_pocketic_operator_mint_recovers_receipts() {
    let config = Some(IcpFeaturesConfig::DefaultConfig);
    let mut pic = start_pocket_ic(
        PocketIcBuilder::new()
            .with_application_subnet()
            .with_icp_features(IcpFeatures {
                icp_token: config.clone(),
                cycles_minting: config.clone(),
                cycles_token: config,
                ..IcpFeatures::default()
            }),
    );
    pic.set_time(SystemTime::now().into());
    let ledger = Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai").unwrap();
    let cycles_ledger = Principal::from_text("um5iw-rqaaa-aaaaq-qaaba-cai").unwrap();
    let mut key = [0; 32];
    getrandom::fill(&mut key).unwrap();
    let identity = BasicIdentity::from_raw_key(&key);
    let url = pic.make_live(None);
    let agent = Agent::builder()
        .with_url(url)
        .with_identity(identity)
        .with_max_response_body_size(4 * 1024 * 1024)
        .build()
        .unwrap();
    agent.set_root_key(pic.root_key().unwrap());
    let operator = agent.get_principal().unwrap();
    let result: Result<Nat, icrc_ledger_types::icrc1::transfer::TransferError> = pic
        .update_candid(
            ledger,
            "icrc1_transfer",
            (MintTestTransfer {
                from_subaccount: None,
                to: MintTestAccount {
                    owner: operator,
                    subaccount: None,
                },
                amount: Nat::from(10_000_000_000u64),
                fee: None,
                memo: None,
                created_at_time: None,
            },),
        )
        .unwrap();
    result.unwrap();
    let transport = OperatorMintTransport::from_test_agent(agent);
    let quote = transport
        .quote_blocking(
            ledger,
            Principal::from_text("rkp4c-7iaaa-aaaaa-aaaca-cai").unwrap(),
            cycles_ledger,
        )
        .unwrap();
    assert_eq!(quote.transfer_fee_e8s, 10_000);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    for interruption in 0..4 {
        let (mut fixture, old_intent) = fixture();
        fixture.platform.cycles_ledger = cycles_ledger.to_text();
        fixture.desired.operator = operator.to_text();
        fixture.desired.canisters[0].controllers = vec![operator.to_text()];
        fixture.platform.controllers = vec![operator.to_text()];
        fixture.plan.reviewed_desired = Some(Box::new(ReviewedDesiredFleetRecord::capture(
            &fixture.desired,
        )));
        fixture.plan.plan_sha256 = expected_plan_sha256(&fixture.plan);
        let mut journal = read_journal(&fixture.paths).unwrap().unwrap();
        journal.initial_operator_cycles = transport.operator_balance(cycles_ledger).unwrap();
        fixture.platform.operator = journal.initial_operator_cycles;
        journal.plan_sha256.clone_from(&fixture.plan.plan_sha256);
        let pause = journal.estate_funding_required.as_mut().unwrap();
        pause.plan_sha256.clone_from(&fixture.plan.plan_sha256);
        journal.funding_reviews = vec![funding_records::review(pause, 99)];
        write_plan(&fixture.paths, &fixture.plan).unwrap();
        write_journal(&fixture.paths, &journal).unwrap();
        let intent = mint_records::prepare_intent(
            OperatorMintAuthority {
                plan_sha256: digest(&fixture.plan.plan_sha256),
                funding_review_sha256: digest(&journal.funding_reviews[0].review_sha256),
                operator,
                network_identity_sha256: transport.network_identity(),
                cycles_ledger,
                ..old_intent.authority
            },
            100_000_000,
            10_000,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .try_into()
                .unwrap(),
        )
        .unwrap();
        let reviewed = operator_mint::review(&fixture.paths, &intent).unwrap();
        let expected_argument = mint_records::transfer_argument(&intent).unwrap();
        runtime.block_on(async {
            if interruption < 3 {
                let approved =
                    operator_mint::approve(&fixture.paths, &intent.authority, &reviewed.review_sha256)
                        .unwrap();
                let reply = transport
                    .transfer(&intent, approved.transfer_argument.as_ref().unwrap())
                    .await
                    .unwrap();
                if interruption > 0 {
                    let paid = operator_mint::record_transfer_reply(
                        &fixture.paths,
                        &intent.authority,
                        &reviewed.review_sha256,
                        &reply,
                    )
                    .unwrap();
                    let block = crate::fleet_ensure::policy::operator_mint::progress::transfer_block(
                        paid.transfer_outcome.as_ref().unwrap(),
                    )
                    .unwrap();
                    let transfer = transport.read_transfer(&intent, block).await.unwrap();
                    let notified = operator_mint::prepare_notification(
                        &fixture.paths,
                        &intent.authority,
                        &reviewed.review_sha256,
                    )
                    .unwrap();
                    let reply = transport
                        .notify(&transfer, &notified.notification.unwrap().argument)
                        .await
                        .unwrap();
                    if interruption == 2 {
                        let observed = operator_mint::record_notification_reply(
                            &fixture.paths,
                            &intent.authority,
                            &reviewed.review_sha256,
                            &reply,
                        )
                        .unwrap();
                        assert!(matches!(
                            observed.notification.unwrap().outcome,
                            Some(OperatorMintNotificationOutcomeRecord::Minted { .. })
                        ));
                    }
                }
            }
            let complete = operator_mint::execution::apply(
                &fixture.paths,
                &intent.authority,
                &reviewed.review_sha256,
                &transport,
            )
            .await
            .unwrap();
            let receipt = complete
                .receipt
                .as_ref()
                .expect("both production Ledger transactions authenticated");
            assert_eq!(complete.transfer_argument, Some(expected_argument.clone()));
            if interruption == 0 {
                assert!(matches!(complete.transfer_outcome, Some(crate::fleet_ensure::model::operator_mint::OperatorMintTransferOutcomeRecord::Duplicate { .. })));
            }
            assert_eq!(receipt.deposit_fee_cycles, quote.estimated_deposit_fee_cycles);
            assert_eq!(receipt.gross_minted_cycles, u128::from(intent.amount_e8s) * u128::from(quote.xdr_permyriad_per_icp));
            assert_eq!(transport.read_operator_balance(cycles_ledger).await.unwrap(), journal.initial_operator_cycles + receipt.net_credit_cycles);
            assert_eq!(
                receipt.gross_minted_cycles,
                receipt.net_credit_cycles + receipt.deposit_fee_cycles
            );
            let bytes = fs::read(&fixture.paths.journal).unwrap();
            assert_eq!(
                operator_mint::execution::apply(
                    &fixture.paths,
                    &intent.authority,
                    &reviewed.review_sha256,
                    &transport
                )
                .await
                .unwrap(),
                complete
            );
            assert_eq!(fs::read(&fixture.paths.journal).unwrap(), bytes);
            let after = read_journal(&fixture.paths).unwrap().unwrap();
            assert_eq!(
                after.initial_operator_cycles,
                journal.initial_operator_cycles
            );
            assert_eq!(after.effects, journal.effects);
            fixture.platform.operator += receipt.net_credit_cycles;
        });
        let digest = journal.funding_reviews[0].review_sha256.clone();
        // This host fixture has no Coordinator/runtime topology. Exercise the real
        // retained funding driver up to that existing boundary, then verify its
        // complete monetary equation separately from PocketIC Fleet readiness.
        assert!(matches!(
            fixture.apply(&digest),
            Err(EnsureWorkflowError::Policy(
                crate::fleet_ensure::policy::EnsurePolicyError::InvalidTopology { .. }
            ))
        ));
        let retained = read_journal(&fixture.paths).unwrap().unwrap();
        let state = crate::fleet_ensure::ops::read_state(&fixture.paths, "fleet").unwrap();
        let observed = crate::fleet_ensure::ops::EnsurePlatform::observe(
            &mut fixture.platform,
            &fixture.plan.operation_id,
            &state,
        )
        .unwrap();
        let actual = crate::fleet_ensure::workflow::verify_terminal_conservation::<std::io::Error>(
            &fixture.plan,
            &retained,
            &state,
            &observed,
        )
        .unwrap();
        assert_eq!(
            (
                actual.operator_debit_cycles,
                actual.estate_funding_cycles,
                actual.exact_unavoidable_fee_cycles
            ),
            (65, 60, 5)
        );
        fixture.native_resume(&digest).unwrap();
        assert_eq!(fixture.platform.transfers.len(), 1);
    }
}

fn digest(value: &str) -> [u8; 32] {
    decode_hex(value).unwrap().try_into().unwrap()
}

#[test]
fn fresh_quote_selection_preserves_the_existing_journal() {
    let (fixture, _) = fixture();
    let plan = fs::read(&fixture.paths.plan).unwrap();
    let journal = fs::read(&fixture.paths.journal).unwrap();
    assert!(!operator_mint::fresh_quote_available(&fixture.paths).unwrap());
    assert_eq!(fs::read(&fixture.paths.journal).unwrap(), journal);
    fs::remove_file(&fixture.paths.journal).unwrap();
    assert!(operator_mint::fresh_quote_available(&fixture.paths).unwrap());
    assert!(!fixture.paths.journal.exists());
    assert_eq!(fs::read(&fixture.paths.plan).unwrap(), plan);
}

pub(super) fn fixture() -> (Fixture, OperatorMintIntentRecord) {
    let mut fixture = Fixture::new();
    fixture.review().unwrap();
    // The funding fixture uses readable host-only identifiers. Use full identities
    // for the conversion's wire-bound authority while preserving its live evidence.
    fixture.plan.operation_id = "01".repeat(32);
    let ledger = fixture.desired.cycles_ledger.clone();
    fixture.plan.conservation.estate_funding_domains[0]
        .cycles_ledger
        .clone_from(&ledger);
    fixture.plan.plan_sha256 = expected_plan_sha256(&fixture.plan);
    let mut journal = read_journal(&fixture.paths).unwrap().unwrap();
    journal.operation_id.clone_from(&fixture.plan.operation_id);
    journal.plan_sha256.clone_from(&fixture.plan.plan_sha256);
    let pause = journal.estate_funding_required.as_mut().unwrap();
    pause.operation_id.clone_from(&fixture.plan.operation_id);
    pause.plan_sha256.clone_from(&fixture.plan.plan_sha256);
    pause.cycles_ledger.clone_from(&ledger);
    journal.funding_reviews = vec![funding_records::review(pause, 99)];
    write_plan(&fixture.paths, &fixture.plan).unwrap();
    write_journal(&fixture.paths, &journal).unwrap();
    let intent = mint_records::prepare_intent(
        OperatorMintAuthority {
            operation_id: digest(&fixture.plan.operation_id),
            plan_sha256: digest(&fixture.plan.plan_sha256),
            funding_review_sha256: digest(&journal.funding_reviews[0].review_sha256),
            network_identity_sha256: [2; 32],
            operator: Principal::from_text(ROOT).unwrap(),
            icp_ledger: Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai").unwrap(),
            cmc: Principal::from_text("rkp4c-7iaaa-aaaaa-aaaca-cai").unwrap(),
            cycles_ledger: Principal::from_text(&ledger).unwrap(),
        },
        100_000_000,
        10_000,
        123_456,
    )
    .unwrap();
    (fixture, intent)
}

#[test]
fn operator_mint_review_and_approval_survive_restart_without_rewriting_authority() {
    let (mut fixture, intent) = fixture();
    let original = read_journal(&fixture.paths).unwrap().unwrap();
    let plan_bytes = fs::read(&fixture.paths.plan).unwrap();
    let state_bytes = fs::read(&fixture.paths.state).unwrap();
    let review = operator_mint::review(&fixture.paths, &intent).unwrap();
    assert!(review.transfer_argument.is_none());
    let reviewed_bytes = fs::read(&fixture.paths.journal).unwrap();
    assert_eq!(
        operator_mint::review(&fixture.paths, &intent).unwrap(),
        review
    );
    assert_eq!(fs::read(&fixture.paths.journal).unwrap(), reviewed_bytes);
    let approved =
        operator_mint::approve(&fixture.paths, &intent.authority, &review.review_sha256).unwrap();
    assert_eq!(
        approved.transfer_argument,
        Some(mint_records::transfer_argument(&intent).unwrap())
    );
    assert_eq!(approved.intent, intent);
    let approved_bytes = fs::read(&fixture.paths.journal).unwrap();
    assert_eq!(
        operator_mint::approve(&fixture.paths, &intent.authority, &review.review_sha256).unwrap(),
        approved
    );
    assert_eq!(
        operator_mint::review(&fixture.paths, &intent).unwrap(),
        approved
    );
    assert_eq!(fs::read(&fixture.paths.journal).unwrap(), approved_bytes);
    let mut expected = original.clone();
    expected.funding_reviews[0].operator_mint = Some(approved);
    assert_eq!(read_journal(&fixture.paths).unwrap().unwrap(), expected);
    assert_eq!(fs::read(&fixture.paths.plan).unwrap(), plan_bytes);
    assert_eq!(fs::read(&fixture.paths.state).unwrap(), state_bytes);
    // The normal driver must stop before it can consume the original funding approval.
    assert!(matches!(
        fixture.apply(&original.funding_reviews[0].review_sha256),
        Err(EnsureWorkflowError::OperatorMintPending { .. })
    ));
    assert!(fixture.platform.transfers.is_empty());
    assert_eq!(fs::read(&fixture.paths.journal).unwrap(), approved_bytes);
}

#[test]
fn operator_mint_review_freezes_funding_refresh_until_explicit_cancellation() {
    let (mut fixture, intent) = fixture();
    let before = read_journal(&fixture.paths).unwrap().unwrap();
    let review = operator_mint::review(&fixture.paths, &intent).unwrap();
    assert!(matches!(
        fixture.review(),
        Err(EnsureWorkflowError::OperatorMintPending { .. })
    ));
    assert!(matches!(
        fixture.apply(&fixture.plan.plan_sha256.clone()),
        Err(EnsureWorkflowError::OperatorMintPending { .. })
    ));
    operator_mint::cancel_review(&fixture.paths, &intent.authority, &review.review_sha256).unwrap();
    assert_eq!(read_journal(&fixture.paths).unwrap().unwrap(), before);
    assert!(fixture.platform.transfers.is_empty());
}

#[test]
fn operator_mint_approval_rejects_changed_authority_or_digest_without_a_write() {
    let (fixture, intent) = fixture();
    let review = operator_mint::review(&fixture.paths, &intent).unwrap();
    let before = fs::read(&fixture.paths.journal).unwrap();
    let mutations: [fn(&mut OperatorMintAuthority); 8] = [
        |a| a.operation_id[0] ^= 1,
        |a| a.plan_sha256[0] ^= 1,
        |a| a.funding_review_sha256[0] ^= 1,
        |a| a.network_identity_sha256[0] ^= 1,
        |a| a.operator = Principal::from_slice(&[9, 1]),
        |a| a.icp_ledger = Principal::from_slice(&[9, 1]),
        |a| a.cmc = Principal::from_slice(&[9, 1]),
        |a| a.cycles_ledger = Principal::from_slice(&[9, 1]),
    ];
    for mutate in mutations {
        let mut authority = intent.authority.clone();
        mutate(&mut authority);
        assert!(matches!(
            operator_mint::approve(&fixture.paths, &authority, &review.review_sha256),
            Err(EnsureWorkflowError::OperatorMintReviewConflict)
        ));
    }
    assert!(matches!(
        operator_mint::approve(&fixture.paths, &intent.authority, "wrong-review"),
        Err(EnsureWorkflowError::OperatorMintReviewConflict)
    ));
    assert_eq!(fs::read(&fixture.paths.journal).unwrap(), before);
}

#[test]
fn operator_mint_approved_intent_cannot_be_replaced_or_cancelled() {
    let (fixture, intent) = fixture();
    let review = operator_mint::review(&fixture.paths, &intent).unwrap();
    operator_mint::approve(&fixture.paths, &intent.authority, &review.review_sha256).unwrap();
    let before = fs::read(&fixture.paths.journal).unwrap();
    let changed = mint_records::prepare_intent(
        intent.authority.clone(),
        intent.amount_e8s + 1,
        intent.transfer_fee_e8s,
        intent.created_at_time_ns + 1,
    )
    .unwrap();
    assert!(matches!(
        operator_mint::review(&fixture.paths, &changed),
        Err(EnsureWorkflowError::OperatorMintReviewConflict)
    ));
    assert!(matches!(
        operator_mint::cancel_review(&fixture.paths, &intent.authority, &review.review_sha256),
        Err(EnsureWorkflowError::OperatorMintReviewConflict)
    ));
    assert_eq!(fs::read(&fixture.paths.journal).unwrap(), before);
}

#[test]
fn operator_mint_corrupt_approved_bytes_cannot_resume() {
    let (fixture, intent) = fixture();
    let review = operator_mint::review(&fixture.paths, &intent).unwrap();
    operator_mint::approve(&fixture.paths, &intent.authority, &review.review_sha256).unwrap();
    let mut journal = read_journal(&fixture.paths).unwrap().unwrap();
    journal.funding_reviews[0]
        .operator_mint
        .as_mut()
        .unwrap()
        .transfer_argument
        .as_mut()
        .unwrap()[0] ^= 1;
    write_journal(&fixture.paths, &journal).unwrap();
    let before = fs::read(&fixture.paths.journal).unwrap();
    assert!(matches!(
        operator_mint::approve(&fixture.paths, &intent.authority, &review.review_sha256),
        Err(EnsureWorkflowError::JournalIntegrity)
    ));
    assert_eq!(fs::read(&fixture.paths.journal).unwrap(), before);
}

#[test]
fn operator_mint_journal_requires_explicit_review_and_approval_fields() {
    let (fixture, intent) = fixture();
    let journal = read_journal(&fixture.paths).unwrap().unwrap();
    let mut funding = serde_json::to_value(&journal.funding_reviews[0]).unwrap();
    assert!(funding["operator_mint"].is_null());
    funding.as_object_mut().unwrap().remove("operator_mint");
    assert!(serde_json::from_value::<FundingReviewRecord>(funding).is_err());
    operator_mint::review(&fixture.paths, &intent).unwrap();
    let mut journal = serde_json::to_value(read_journal(&fixture.paths).unwrap().unwrap()).unwrap();
    journal["funding_reviews"][0]["operator_mint"]
        .as_object_mut()
        .unwrap()
        .remove("transfer_argument");
    assert!(serde_json::from_value::<FleetEnsureJournalRecord>(journal).is_err());
}
