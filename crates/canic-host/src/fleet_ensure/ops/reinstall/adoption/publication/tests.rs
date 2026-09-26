//! Local publication crash boundaries; no source admission or IC effects are simulated here.

use super::*;
use crate::fleet_ensure::ops::lock_operation;
use crate::fleet_ensure::{
    model::*,
    ops::{read_state, reinstall::adoption as storage},
};
use std::{collections::BTreeMap, fs};

struct Fixture {
    root: PathBuf,
    paths: EnsurePaths,
    review: CompletedEstatePublicationReviewRecord,
    before: [Vec<u8>; 3],
    after: [Vec<u8>; 3],
}

impl Fixture {
    fn new() -> Self {
        let (fixture, paths, mut plan) = crate::fleet_ensure::tests::terminal_retirement_fixture();
        let mut desired = fixture.desired.clone();
        // New current authority for this local-publication fixture. It is not derived
        // from the opaque source below and is never submitted to an effect driver.
        desired.bootstrap = Some(DesiredFleetBootstrap {
            admission_identity_origin: None,
            admission: canic_core::shared_support::fleet_admission_policy::compile_fleet_admission_policy_template(vec![candid::Principal::from_text(&desired.operator).unwrap()], Vec::new()).unwrap(),
            app: "publication_test".into(),
            canonical_network_id: "11".repeat(32).parse().unwrap(),
            component_deployment_configuration: canic_core::bootstrap::parse_config_model("[app]\nname = 'publication_test'\n[roles.root]\nkind = 'root'\n").unwrap().compile_component_deployment_configuration().unwrap(),
            coordinator: desired.canisters[0].name.clone(),
            coordinator_subnet: candid::Principal::self_authenticating(b"subnet").into(),
            fleet_id: "22".repeat(32).parse().unwrap(), fresh_estate: false,
            recovery_controllers: Vec::new(),
            release_build_id: "33".repeat(32).parse().unwrap(), root_funding: None, roots: Vec::new(),
        });
        plan.operation_id = "99".repeat(32);
        plan.reviewed_desired = Some(Box::new(ReviewedDesiredFleetRecord::capture(&desired)));
        plan.plan_sha256 = expected_plan_sha256(&plan);
        let journal = FleetEnsureJournalRecord {
            funding_observations: BTreeMap::new(),
            funding_reviews: Vec::new(),
            successor_phases: Vec::new(),
            completion: FleetEnsureCompletion::InProgress,
            estate_funding_required: None,
            effects: Vec::new(),
            fleet: plan.fleet.clone(),
            initial_controlled_cycles: plan.conservation.observed_controlled_cycles,
            initial_estate_funding_cycles_by_root: BTreeMap::new(),
            initial_operator_cycles: u128::MAX,
            operation_id: plan.operation_id.clone(),
            plan_sha256: plan.plan_sha256.clone(),
            schema_version: 1,
            stalled_observations: 0,
        };
        let state = FleetEnsureStateRecord {
            active_registry: None,
            completed_reinstall_action_sha256: BTreeMap::new(),
            completed_reinstall_operation_id: None,
            completed_reinstalls: BTreeMap::new(),
            fleet: plan.fleet.clone(),
            pending_principals: BTreeMap::new(),
            principals: desired
                .canisters
                .iter()
                .map(|entry| (entry.name.clone(), entry.principal.clone().unwrap()))
                .collect(),
            retained_cycles_by_principal: BTreeMap::new(),
            schema_version: 1,
            topology: BTreeMap::new(),
        };
        validate_target(&plan, &journal, &state).unwrap();
        let before = [
            br#"{"protocol_actions":[],"reviewed_desired":{"desired":{"bootstrap":{"coordinator":"source"}}}}"#.to_vec(),
            br#"{"completion":"converged","old_receipts":"unchanged"}"#.to_vec(),
            br#"{"active_registry":{"authority":{"binding":{"coordinator":"source"}}}}"#.to_vec(),
        ];
        let source = FleetTerminalSourceRecord {
            operation_id: "44".repeat(32),
            plan_sha256: "55".repeat(32),
            plan_document_sha256: sha256_hex(&before[0]),
            journal_document_sha256: sha256_hex(&before[1]),
            state_document_sha256: sha256_hex(&before[2]),
            phase_document_sha256: BTreeMap::new(),
        };
        for (path, bytes) in [&paths.plan, &paths.journal, &paths.state]
            .into_iter()
            .zip(&before)
        {
            write_bytes(path, bytes).unwrap();
        }
        archive::retain_source(&paths, &source).unwrap();
        let replacement = archive::retain_target(&paths, &plan, &journal, &state).unwrap();
        let artifact = b"exact source interface bytes";
        let artifact_sha256 = sha256_hex(artifact);
        storage::retain(&paths, &artifact_sha256, artifact).unwrap();
        let mut review = CompletedEstatePublicationReviewRecord {
            schema_version: 1,
            environment: plan.environment.clone(),
            fleet: plan.fleet.clone(),
            operation_id: plan.operation_id.clone(),
            plan_sha256: plan.plan_sha256.clone(),
            source,
            custody: fixture_custody(&desired),
            replacement,
            source_artifacts: BTreeMap::from([("source.did".into(), artifact_sha256)]),
            review_sha256: String::new(),
        };
        review.review_sha256 = review_digest(&review).unwrap();
        let after = [
            serialize(&plan, &paths.plan).unwrap(),
            serialize(&journal, &paths.journal).unwrap(),
            serialize(&state, &paths.state).unwrap(),
        ];
        write_current(&review_path(&paths), &review).unwrap();
        Self {
            root: fixture.root,
            paths,
            review,
            before,
            after,
        }
    }

    fn interrupt(&self, replaced: usize) {
        let bytes = serialize(&self.review, &review_path(&self.paths)).unwrap();
        let hash = sha256_hex(&bytes);
        storage::retain(&self.paths, &hash, &bytes).unwrap();
        write_current(
            &marker_path(&self.paths),
            &CompletedEstatePublicationRecord {
                schema_version: 1,
                review_sha256: self.review.review_sha256.clone(),
                review_document_sha256: hash,
                complete: false,
            },
        )
        .unwrap();
        for (index, path) in [&self.paths.plan, &self.paths.journal, &self.paths.state]
            .into_iter()
            .enumerate()
        {
            write_bytes(
                path,
                if index < replaced {
                    &self.after[index]
                } else {
                    &self.before[index]
                },
            )
            .unwrap();
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn all_three_document_crash_boundaries_recover_before_current_state_decode() {
    let fixture = Fixture::new();
    for replaced in 0..=3 {
        fixture.interrupt(replaced);
        let before = [
            &fixture.paths.plan,
            &fixture.paths.journal,
            &fixture.paths.state,
        ]
        .map(|path| fs::read(path).unwrap());
        let preflight = crate::fleet_ensure::ops::retained_contract::check(
            &fixture.root,
            &fixture.review.environment,
            &fixture.review.fleet,
        );
        assert!(matches!(
            preflight,
            Err(crate::fleet_ensure::ops::retained_contract::RetainedContractError::PublicationRecoveryRequired {
                review_sha256, plan_sha256,
            }) if review_sha256 == fixture.review.review_sha256 && plan_sha256 == fixture.review.plan_sha256
        ));
        assert_eq!(
            before,
            [
                &fixture.paths.plan,
                &fixture.paths.journal,
                &fixture.paths.state
            ]
            .map(|path| fs::read(path).unwrap())
        );
        let lock = lock_operation(&fixture.paths).unwrap();
        for (path, expected) in [
            &fixture.paths.plan,
            &fixture.paths.journal,
            &fixture.paths.state,
        ]
        .into_iter()
        .zip(&fixture.after)
        {
            assert_eq!(fs::read(path).unwrap(), *expected);
        }
        assert!(marker(&fixture.paths).unwrap().unwrap().complete);
        assert_eq!(
            read_state(&fixture.paths, &fixture.review.fleet)
                .unwrap()
                .fleet,
            fixture.review.fleet
        );
        drop(lock);
    }
    for ((_, digest, _), original) in documents(&fixture.paths, &fixture.review)
        .into_iter()
        .zip(&fixture.before)
    {
        assert_eq!(
            storage::exact_bytes(&storage::object_path(&fixture.paths, digest), digest).unwrap(),
            *original
        );
    }
}

#[test]
fn review_and_wrong_approval_preserve_active_source_and_do_not_commit() {
    let fixture = Fixture::new();
    assert_eq!(
        review(&fixture.paths).unwrap(),
        Some(fixture.review.clone())
    );
    assert!(matches!(
        adopt(&fixture.paths, &"88".repeat(32), None),
        Err(EnsureStateError::CompletedHandoffConflict)
    ));
    assert!(marker(&fixture.paths).unwrap().is_none());
    for (path, expected) in [
        &fixture.paths.plan,
        &fixture.paths.journal,
        &fixture.paths.state,
    ]
    .into_iter()
    .zip(&fixture.before)
    {
        assert_eq!(fs::read(path).unwrap(), *expected);
    }
}

#[test]
fn committed_recovery_uses_archives_when_review_and_source_artifacts_are_gone() {
    let fixture = Fixture::new();
    fixture.interrupt(1);
    fs::remove_file(review_path(&fixture.paths)).unwrap();
    // The mutable source interface has never existed in this private workspace;
    // only its immutable archived bytes remain after intent.
    assert!(!fixture.root.join("source.did").exists());
    let lock = lock_operation(&fixture.paths).unwrap();
    assert!(marker(&fixture.paths).unwrap().unwrap().complete);
    for (path, expected) in [
        &fixture.paths.plan,
        &fixture.paths.journal,
        &fixture.paths.state,
    ]
    .into_iter()
    .zip(&fixture.after)
    {
        assert_eq!(fs::read(path).unwrap(), *expected);
    }
    drop(lock);
}

#[test]
fn completed_replay_preserves_later_plan_journal_and_state_without_writes() {
    let fixture = Fixture::new();
    commit(&fixture.paths, &fixture.review).unwrap();
    for path in [
        &fixture.paths.plan,
        &fixture.paths.journal,
        &fixture.paths.state,
    ] {
        write_bytes(path, b"later current operation progress").unwrap();
    }
    let paths = [
        &fixture.paths.plan,
        &fixture.paths.journal,
        &fixture.paths.state,
        &marker_path(&fixture.paths),
    ];
    let before = paths
        .iter()
        .map(|path| {
            (
                fs::read(path).unwrap(),
                fs::metadata(path).unwrap().modified().unwrap(),
            )
        })
        .collect::<Vec<_>>();
    adopt(&fixture.paths, &fixture.review.review_sha256, None).unwrap();
    assert!(review(&fixture.paths).unwrap().is_none());
    for (path, expected) in paths.into_iter().zip(before) {
        assert_eq!(
            (
                fs::read(path).unwrap(),
                fs::metadata(path).unwrap().modified().unwrap()
            ),
            expected
        );
    }
    assert!(matches!(
        adopt(&fixture.paths, &"88".repeat(32), None),
        Err(EnsureStateError::CompletedHandoffConflict)
    ));
}

#[test]
fn every_drift_rejects_before_any_active_document_is_replaced() {
    for changed in ["plan", "journal", "state", "archive", "target", "artifact"] {
        let fixture = Fixture::new();
        fixture.interrupt(1);
        let target = match changed {
            "plan" => fixture.paths.plan.clone(),
            "journal" => fixture.paths.journal.clone(),
            "state" => fixture.paths.state.clone(),
            "archive" => {
                storage::object_path(&fixture.paths, &fixture.review.source.state_document_sha256)
            }
            "target" => {
                storage::object_path(&fixture.paths, &fixture.review.replacement.state_sha256)
            }
            "artifact" => storage::object_path(
                &fixture.paths,
                fixture.review.source_artifacts.values().next().unwrap(),
            ),
            _ => unreachable!(),
        };
        write_bytes(&target, b"changed").unwrap();
        let before = [
            &fixture.paths.plan,
            &fixture.paths.journal,
            &fixture.paths.state,
        ]
        .map(|path| fs::read(path).unwrap());
        assert!(
            matches!(
                lock_operation(&fixture.paths),
                Err(EnsureStateError::CompletedHandoffConflict
                    | EnsureStateError::ActivationResetAdoptionConflict)
            ),
            "{changed}"
        );
        let after = [
            &fixture.paths.plan,
            &fixture.paths.journal,
            &fixture.paths.state,
        ]
        .map(|path| fs::read(path).unwrap());
        assert_eq!(before, after);
        assert!(!marker(&fixture.paths).unwrap().unwrap().complete);
    }
}

#[test]
fn current_target_state_and_journal_cannot_carry_predecessor_progress() {
    let fixture = Fixture::new();
    let plan: FleetEnsurePlan = serde_json::from_slice(&fixture.after[0]).unwrap();
    let journal: FleetEnsureJournalRecord = serde_json::from_slice(&fixture.after[1]).unwrap();
    let state: FleetEnsureStateRecord = serde_json::from_slice(&fixture.after[2]).unwrap();
    let mut changed = state.clone();
    changed.completed_reinstall_operation_id = Some("77".repeat(32));
    assert!(matches!(
        validate_target(&plan, &journal, &changed),
        Err(EnsureStateError::CompletedHandoffConflict)
    ));
    let mut changed = journal.clone();
    changed.stalled_observations = 1;
    assert!(matches!(
        validate_target(&plan, &changed, &state),
        Err(EnsureStateError::CompletedHandoffConflict)
    ));
    let mut changed = journal;
    changed.initial_controlled_cycles += 1;
    assert!(matches!(
        validate_target(&plan, &changed, &state),
        Err(EnsureStateError::CompletedHandoffConflict)
    ));
}

#[test]
fn oversized_publication_marker_rejects_without_touching_active_files() {
    let fixture = Fixture::new();
    write_bytes(&marker_path(&fixture.paths), &vec![b' '; 4097]).unwrap();
    assert!(matches!(
        lock_operation(&fixture.paths),
        Err(EnsureStateError::Io { .. })
    ));
    for (path, expected) in [
        &fixture.paths.plan,
        &fixture.paths.journal,
        &fixture.paths.state,
    ]
    .into_iter()
    .zip(&fixture.before)
    {
        assert_eq!(fs::read(path).unwrap(), *expected);
    }
}

#[test]
fn uncommitted_publication_requires_fresh_unchanged_certified_custody() {
    use crate::fleet_ensure::view::terminal_source::inventory::CompletedCanisterCustodyView;
    use std::time::{Duration, Instant};
    let fixture = Fixture::new();
    let hash = &fixture.review.review_sha256;
    assert!(matches!(
        adopt(&fixture.paths, hash, None),
        Err(EnsureStateError::CompletedHandoffCustodyRequired)
    ));
    let mut custody = CompletedEstateCustodyView {
        observed_at: Instant::now(),
        documents: fixture.review.source.clone(),
        network: fixture.review.custody.network,
        operator: fixture.review.custody.operator,
        canisters: fixture
            .review
            .custody
            .canisters
            .iter()
            .map(|(name, entry)| {
                (
                    name.clone(),
                    CompletedCanisterCustodyView {
                        principal: entry.binding.principal,
                        subnet: entry.binding.subnet,
                        controllers: entry.binding.controllers.clone(),
                        module_sha256: entry.binding.module_sha256.clone(),
                        certificate_tree_sha256: [9; 32],
                    },
                )
            })
            .collect(),
    };
    let fresh = custody::capture(&custody, &fixture.review.source).unwrap();
    assert!(custody::same_authority(&fixture.review.custody, &fresh));
    custody.observed_at = Instant::now().checked_sub(Duration::from_secs(61)).unwrap();
    assert!(
        matches!(adopt(&fixture.paths, hash, Some(&custody)), Err(EnsureStateError::CompletedHandoffCustody(error)) if matches!(*error, crate::fleet_ensure::ops::retained_contract::CompletedCustodyError::Expired))
    );
    custody.observed_at = Instant::now();
    custody
        .canisters
        .values_mut()
        .next()
        .unwrap()
        .controllers
        .push(candid::Principal::anonymous());
    assert!(matches!(
        adopt(&fixture.paths, hash, Some(&custody)),
        Err(EnsureStateError::CompletedHandoffConflict)
    ));
    assert!(marker(&fixture.paths).unwrap().is_none());
    assert_eq!(fs::read(&fixture.paths.plan).unwrap(), fixture.before[0]);
    assert_eq!(fs::read(&fixture.paths.journal).unwrap(), fixture.before[1]);
    assert_eq!(fs::read(&fixture.paths.state).unwrap(), fixture.before[2]);
}

fn fixture_custody(desired: &DesiredFleet) -> completed_handoff::CompletedEstateCustodyRecord {
    completed_handoff::CompletedEstateCustodyRecord {
        network: desired.bootstrap.as_ref().unwrap().canonical_network_id,
        operator: candid::Principal::from_text(&desired.operator).unwrap(),
        canisters: desired
            .canisters
            .iter()
            .map(|entry| {
                (
                    entry.name.clone(),
                    completed_handoff::CompletedCanisterCustodyRecord {
                        binding: completed_handoff::CompletedPhysicalBindingRecord {
                            principal: candid::Principal::from_text(
                                entry.principal.as_ref().unwrap(),
                            )
                            .unwrap(),
                            subnet: canic_core::ids::SubnetId::from_principal(
                                candid::Principal::from_text(&entry.subnet).unwrap(),
                            ),
                            controllers: vec![
                                candid::Principal::from_text(&desired.operator).unwrap(),
                            ],
                            module_sha256: None,
                        },
                        certificate_tree_sha256: [1; 32],
                    },
                )
            })
            .collect(),
    }
}
