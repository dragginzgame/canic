//! Local intent/replay invariants. These tests do not simulate successful IC admission or calls.

use super::*;
use crate::fleet_ensure::{
    model::{
        FleetTerminalSourceRecord,
        completed_handoff::{
            CompletedCanisterCustodyRecord, CompletedEstateCustodyRecord,
            CompletedPhysicalBindingRecord,
        },
    },
    workflow::completed_preparation as workflow,
};
use std::{collections::BTreeMap, fs};

struct Fixture {
    paths: EnsurePaths,
    review: CompletedPreparationReviewRecord,
}

impl Fixture {
    fn new() -> Self {
        let workspace = crate::test_support::temp_dir("completed-preparation");
        let paths = EnsurePaths::under(&workspace, "local", "preparation");
        let operator = Principal::self_authenticating(b"operator");
        let source = FleetTerminalSourceRecord {
            operation_id: "11".repeat(32),
            plan_sha256: "22".repeat(32),
            plan_document_sha256: "33".repeat(32),
            journal_document_sha256: "44".repeat(32),
            state_document_sha256: "55".repeat(32),
            phase_document_sha256: BTreeMap::new(),
        };
        let mut review = CompletedPreparationReviewRecord {
            schema_version: 1,
            cli_release: env!("CARGO_PKG_VERSION").into(),
            environment: "local".into(),
            fleet: "preparation".into(),
            operation_id: sha256_hex(
                &serde_json::to_vec(&(
                    "canic:completed-preparation:v1",
                    env!("CARGO_PKG_VERSION"),
                    &source,
                ))
                .unwrap(),
            ),
            source,
            custody: CompletedEstateCustodyRecord {
                network: "11".repeat(32).parse().unwrap(),
                operator,
                canisters: BTreeMap::new(),
            },
            source_artifacts: BTreeMap::new(),
            actions: Vec::new(),
            maximum_attempts_per_action: ATTEMPTS,
            maximum_observations_per_action: OBSERVATIONS,
            maximum_execution_burn_cycles: 2 * (ACTION_BURN + INSPECTION_BURN),
            inspection_roots: BTreeMap::new(),
            source_accounting: CompletedSourceAccountingRecord {
                initial_native_cycles: 6 * ACTION_BURN,
                recorded_funding_cycles: 0,
                maximum_source_burn_cycles: ACTION_BURN,
            },
            review_sha256: String::new(),
        };
        for (name, kind) in [
            ("coordinator", DesiredCanisterKind::Coordinator),
            ("root", DesiredCanisterKind::Root),
        ] {
            let principal = Principal::self_authenticating(name.as_bytes());
            review.inspection_roots.insert(name.into(), None);
            review.custody.canisters.insert(
                name.into(),
                CompletedCanisterCustodyRecord {
                    binding: CompletedPhysicalBindingRecord {
                        principal,
                        subnet: Principal::from_slice(&[1]).into(),
                        controllers: vec![operator],
                        module_sha256: Some("11".repeat(32)),
                    },
                    certificate_tree_sha256: [2; 32],
                },
            );
            review.actions.push(EnsureAction::SealAuthority {
                authority_kind: kind,
                name: name.into(),
                principal: principal.to_text(),
                candid: format!("interfaces/{name}.did"),
                candid_sha256: "66".repeat(32),
            });
        }
        review.review_sha256 = digest(&review).unwrap();
        stage(&paths, &review).unwrap();
        Self { paths, review }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.paths.workspace).unwrap();
    }
}
fn balance(native: u128, reserved: u128) -> CompletedPreparationBalanceRecord {
    CompletedPreparationBalanceRecord {
        status: crate::fleet_ensure::model::CanisterRuntimeStatus::Running,
        native_cycles: native,
        reserved_cycles: reserved,
    }
}

#[test]
fn interrupted_intent_retains_allowance_and_excludes_other_fleet_work() {
    let fixture = Fixture::new();
    let mut retained = begin(&fixture.paths, &fixture.review).unwrap();
    assert!(matches!(
        ops::lock_operation(&fixture.paths),
        Err(EnsureStateError::CompletedPreparationInProgress)
    ));
    assert!(ops::lock_completed_preparation(&fixture.paths).is_ok());
    consume(&fixture.paths, &mut retained, 0, true).unwrap();
    retain_balance(
        &fixture.paths,
        &mut retained,
        0,
        balance(3 * ACTION_BURN, 7),
        false,
    )
    .unwrap();
    for _ in 0..ATTEMPTS {
        let mut restarted = journal(&fixture.paths, &fixture.review).unwrap().unwrap();
        consume(&fixture.paths, &mut restarted, 0, false).unwrap();
    }
    let mut restarted = journal(&fixture.paths, &fixture.review).unwrap().unwrap();
    assert_eq!(restarted.effects[0].before, retained.effects[0].before);
    assert!(matches!(
        consume(&fixture.paths, &mut restarted, 0, false),
        Err(CompletedPreparationError::Budget)
    ));
    assert!(matches!(
        stage(&fixture.paths, &fixture.review),
        Err(CompletedPreparationError::Conflict)
    ));
    assert_eq!(
        journal(&fixture.paths, &fixture.review)
            .unwrap()
            .unwrap()
            .effects[0]
            .submission_attempts,
        ATTEMPTS
    );
}

#[test]
fn exhausted_observation_intent_cannot_be_rebased() {
    let fixture = Fixture::new();
    let mut retained = begin(&fixture.paths, &fixture.review).unwrap();
    for _ in 0..OBSERVATIONS {
        consume(&fixture.paths, &mut retained, 0, true).unwrap();
    }
    let mut restarted = journal(&fixture.paths, &fixture.review).unwrap().unwrap();
    assert!(matches!(
        consume(&fixture.paths, &mut restarted, 0, true),
        Err(CompletedPreparationError::Budget)
    ));
    assert_eq!(restarted.effects[0].before, None);
}

#[test]
fn reserved_cycles_cannot_fund_preparation_and_credits_do_not_mask_debits() {
    let fixture = Fixture::new();
    let mut retained = begin(&fixture.paths, &fixture.review).unwrap();
    consume(&fixture.paths, &mut retained, 0, true).unwrap();
    assert!(matches!(
        retain_balance(
            &fixture.paths,
            &mut retained,
            0,
            balance(1, 10 * ACTION_BURN),
            false
        ),
        Err(CompletedPreparationError::Conservation)
    ));
    let before = balance(3 * ACTION_BURN, 100);
    for after in [
        balance(1, 100),
        balance(3 * ACTION_BURN + 1, 100),
        balance(u128::MAX, 1),
    ] {
        assert!(matches!(
            validate_debit(before, after),
            Err(CompletedPreparationError::Conservation)
        ));
    }
    validate_debit(before, balance(3 * ACTION_BURN - 100, 100)).unwrap();
}

#[test]
fn completed_local_replay_never_contacts_icp_or_rewrites_source() {
    let fixture = Fixture::new();
    let original = br#"{"completed":true,"bootstrap":{"coordinator":"original"}}"#;
    for path in [
        &fixture.paths.plan,
        &fixture.paths.journal,
        &fixture.paths.state,
    ] {
        fs::write(path, original).unwrap();
    }
    let mut retained = begin(&fixture.paths, &fixture.review).unwrap();
    for index in 0..fixture.review.actions.len() {
        consume(&fixture.paths, &mut retained, index, true).unwrap();
        retain_balance(
            &fixture.paths,
            &mut retained,
            index,
            balance(3 * ACTION_BURN, 17),
            false,
        )
        .unwrap();
        consume(&fixture.paths, &mut retained, index, false).unwrap();
        mark_applied(&fixture.paths, &mut retained, index).unwrap();
        consume(&fixture.paths, &mut retained, index, true).unwrap();
        retain_balance(
            &fixture.paths,
            &mut retained,
            index,
            balance(3 * ACTION_BURN - 10, 17),
            true,
        )
        .unwrap();
    }
    for name in inventory::pending(&fixture.review, &retained) {
        inventory::consume(&fixture.paths, &mut retained, &name).unwrap();
        inventory::retain(
            &fixture.paths,
            &mut retained,
            &name,
            balance(3 * ACTION_BURN - 20, 17),
        )
        .unwrap();
    }
    finish(&fixture.paths, &fixture.review, &mut retained).unwrap();
    let bytes = fs::read(journal_path(&fixture.paths)).unwrap();
    let icp = IcpCli::new(
        "/nonexistent/completed-preparation-icp",
        Some("local".into()),
    );
    for _ in 0..2 {
        assert_eq!(
            workflow::apply(
                &fixture.paths.workspace,
                "local",
                "preparation",
                &fixture.review.review_sha256,
                &icp
            )
            .unwrap(),
            retained
        );
        assert_eq!(fs::read(journal_path(&fixture.paths)).unwrap(), bytes);
    }
    assert!(matches!(
        workflow::apply(
            &fixture.paths.workspace,
            "local",
            "preparation",
            &"00".repeat(32),
            &icp
        ),
        Err(CompletedPreparationError::Conflict)
    ));
    for path in [
        &fixture.paths.plan,
        &fixture.paths.journal,
        &fixture.paths.state,
    ] {
        assert_eq!(fs::read(path).unwrap(), original);
    }
}

#[test]
fn partial_or_reordered_completion_cannot_become_terminal() {
    let fixture = Fixture::new();
    let initial = begin(&fixture.paths, &fixture.review).unwrap();
    for invalid in ["applied", "out_of_order", "credit", "overflow", "terminal"] {
        let mut retained = initial.clone();
        match invalid {
            "applied" => retained.effects[0].applied = true,
            "out_of_order" => retained.effects[1].observation_attempts = 1,
            "credit" | "overflow" => {
                let effect = &mut retained.effects[0];
                effect.before = Some(balance(3 * ACTION_BURN, 0));
                effect.after = Some(if invalid == "credit" {
                    balance(4 * ACTION_BURN, 0)
                } else {
                    balance(u128::MAX, 1)
                });
                effect.observation_attempts = 2;
                effect.applied = true;
            }
            "terminal" => retained.prepared = true,
            _ => unreachable!(),
        }
        save(&fixture.paths, &retained).unwrap();
        assert!(
            matches!(
                journal(&fixture.paths, &fixture.review),
                Err(CompletedPreparationError::Conflict | CompletedPreparationError::Conservation)
            ),
            "{invalid}"
        );
    }
    let mut retained = initial;
    assert!(matches!(
        finish(&fixture.paths, &fixture.review, &mut retained),
        Err(CompletedPreparationError::Conflict)
    ));
}

#[test]
fn preparation_review_binds_release_action_order_and_observation_limit() {
    let fixture = Fixture::new();
    for mutation in [
        "release",
        "observations",
        "order",
        "duplicate",
        "path",
        "owner",
        "operation",
    ] {
        let mut review = fixture.review.clone();
        match mutation {
            "release" => review.cli_release = "other".into(),
            "observations" => review.maximum_observations_per_action += 1,
            "order" => review.actions.swap(0, 1),
            "duplicate" => review.actions[1] = review.actions[0].clone(),
            "path" => {
                if let EnsureAction::SealAuthority { candid, .. } = &mut review.actions[0] {
                    *candid = "../other.did".into();
                }
            }
            "owner" => review
                .custody
                .canisters
                .get_mut("root")
                .unwrap()
                .binding
                .controllers
                .clear(),
            "operation" => review.operation_id = "99".repeat(32),
            _ => unreachable!(),
        }
        review.review_sha256 = digest(&review).unwrap();
        assert!(
            matches!(verify(&review), Err(CompletedPreparationError::Conflict)),
            "{mutation}"
        );
    }
    let mut raw = serde_json::to_value(begin(&fixture.paths, &fixture.review).unwrap()).unwrap();
    raw["effects"][0].as_object_mut().unwrap().remove("before");
    assert!(serde_json::from_value::<CompletedPreparationJournalRecord>(raw).is_err());
}

#[test]
fn early_preflight_identifies_preparation_before_old_executable_decode() {
    let fixture = Fixture::new();
    let original = br#"{"reviewed_desired":{"desired":{"bootstrap":{"coordinator":"old"}}}}"#;
    fs::write(&fixture.paths.plan, original).unwrap();
    begin(&fixture.paths, &fixture.review).unwrap();
    assert!(matches!(ops::retained_contract::check(
        &fixture.paths.workspace, "local", "preparation",
    ), Err(ops::retained_contract::RetainedContractError::PreparationRecoveryRequired {
        review_sha256, prepared: false,
    }) if review_sha256 == fixture.review.review_sha256));
    assert_eq!(fs::read(&fixture.paths.plan).unwrap(), original);
    let mut moved = fixture.review.clone();
    moved.fleet = "other".into();
    moved.review_sha256 = digest(&moved).unwrap();
    ops::write_current(&review_path(&fixture.paths), &moved).unwrap();
    assert!(matches!(
        review(&fixture.paths),
        Err(CompletedPreparationError::Conflict)
    ));
}

#[test]
fn estate_inventory_requires_complete_samples_and_preserves_consumed_attempts() {
    let fixture = Fixture::new();
    let mut retained = begin(&fixture.paths, &fixture.review).unwrap();
    assert!(matches!(
        inventory::consume(&fixture.paths, &mut retained, "root"),
        Err(CompletedPreparationError::Conflict)
    ));
    for effect in &mut retained.effects {
        effect.applied = true;
        effect.observation_attempts = 2;
        effect.before = Some(balance(3 * ACTION_BURN, 0));
        effect.after = Some(balance(3 * ACTION_BURN - 10, 0));
    }
    save(&fixture.paths, &retained).unwrap();
    for _ in 0..ATTEMPTS {
        let mut restarted = journal(&fixture.paths, &fixture.review).unwrap().unwrap();
        inventory::consume(&fixture.paths, &mut restarted, "root").unwrap();
    }
    let mut restarted = journal(&fixture.paths, &fixture.review).unwrap().unwrap();
    assert!(matches!(
        inventory::consume(&fixture.paths, &mut restarted, "root"),
        Err(CompletedPreparationError::Budget)
    ));
    assert!(matches!(
        finish(&fixture.paths, &fixture.review, &mut restarted),
        Err(CompletedPreparationError::Conflict)
    ));
    inventory::retain(
        &fixture.paths,
        &mut restarted,
        "root",
        balance(3 * ACTION_BURN - 20, 5),
    )
    .unwrap();
    assert!(matches!(
        inventory::retain(&fixture.paths, &mut restarted, "root", balance(1, 0)),
        Err(CompletedPreparationError::Conflict)
    ));
    assert_eq!(
        inventory::pending(&fixture.review, &restarted),
        ["coordinator"]
    );
}

#[test]
fn estate_conservation_keeps_the_source_baseline_and_reservations_separate() {
    let fixture = Fixture::new();
    let mut retained = begin(&fixture.paths, &fixture.review).unwrap();
    for effect in &mut retained.effects {
        effect.before = Some(balance(3 * ACTION_BURN, 900));
    }
    for inspection in retained.inspections.values_mut() {
        inspection.attempts = 1;
        inspection.balance = Some(balance(3 * ACTION_BURN - 100, 900));
    }
    assert_eq!(
        inventory::conservation(&fixture.review, &retained).unwrap(),
        200
    );
    for inspection in retained.inspections.values_mut() {
        inspection.balance = Some(balance(0, 3 * ACTION_BURN));
    }
    assert!(matches!(
        inventory::conservation(&fixture.review, &retained),
        Err(CompletedPreparationError::Conservation)
    ));
    assert!(matches!(
        inventory::require_headroom(
            &fixture.review,
            "root",
            balance(ACTION_BURN, u128::MAX - ACTION_BURN)
        ),
        Err(CompletedPreparationError::Conservation)
    ));
    inventory::require_headroom(
        &fixture.review,
        "root",
        balance(ACTION_BURN + INSPECTION_BURN, 0),
    )
    .unwrap();
}
