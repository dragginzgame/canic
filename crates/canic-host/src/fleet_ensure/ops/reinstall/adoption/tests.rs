use super::*;
use crate::fleet_ensure::model::{
    CycleConservation, FleetActivationResetRecord, FleetActivationSourceRecord,
    FleetEnsureCompletion, FleetEnsurePlanScope, FleetReinstallRecord,
};
use std::{collections::BTreeMap, fs};

/// Exercise durable handoff with the source-parser fixture's real issued-action evidence.
pub(in crate::fleet_ensure::ops) fn assert_review_handoff(
    source_paths: &EnsurePaths,
    source: &FleetActivationSourceRecord,
) {
    let root = crate::test_support::temp_dir("activation-reset-adoption");
    let paths = EnsurePaths::under(&root, "local", "source");
    for (from, to) in [
        (&source_paths.plan, &paths.plan),
        (&source_paths.journal, &paths.journal),
        (&source_paths.state, &paths.state),
    ] {
        write_bytes(to, &fs::read(from).unwrap()).unwrap();
    }
    let plan = fixture_plan(source.clone());
    assert_handoff(&paths, plan);
    fs::remove_dir_all(root).unwrap();
}

pub(in crate::fleet_ensure) fn assert_terminal_handoff(paths: &EnsurePaths, plan: FleetEnsurePlan) {
    let evidence = terminal_retirement(&plan).unwrap().clone();
    assert_handoff(paths, plan);
    for digest in evidence.phase_document_sha256.values() {
        exact_bytes(&object_path(paths, digest), digest).unwrap();
        let mut interrupted = marker(paths).unwrap().unwrap();
        interrupted.complete = false;
        let journal = exact_bytes(
            &object_path(paths, &interrupted.replacement_journal_sha256),
            &interrupted.replacement_journal_sha256,
        )
        .unwrap();
        write_bytes(&paths.journal, &journal).unwrap();
        write_current(&marker_path(paths), &interrupted).unwrap();
        // With an otherwise valid interrupted pair, this archive corruption alone rejects.
        let original_phase = exact_bytes(&object_path(paths, digest), digest).unwrap();
        write_bytes(&object_path(paths, digest), b"changed phase archive").unwrap();
        let before = read_document_bytes(&paths.plan).unwrap();
        assert!(matches!(
            recover(paths),
            Err(EnsureStateError::ActivationResetAdoptionConflict)
        ));
        assert_eq!(read_document_bytes(&paths.plan).unwrap(), before);
        write_bytes(&object_path(paths, digest), &original_phase).unwrap();
        recover(paths).unwrap();
    }
}

fn assert_handoff(paths: &EnsurePaths, plan: FleetEnsurePlan) {
    let before = [
        fs::read(&paths.plan).unwrap(),
        fs::read(&paths.journal).unwrap(),
        fs::read(&paths.state).unwrap(),
    ];
    stage(paths, &plan).unwrap();
    assert_eq!(review(paths).unwrap(), Some(plan.clone()));
    assert_eq!(
        crate::fleet_ensure::workflow::retained_reinstall_apply_plan::<std::io::Error>(
            &paths.workspace,
            &plan.environment,
            &plan.fleet,
            &plan.plan_sha256,
        )
        .unwrap(),
        Some(plan.clone())
    );
    assert!(
        crate::fleet_ensure::workflow::retained_reinstall_apply_plan::<std::io::Error>(
            &paths.workspace,
            &plan.environment,
            &plan.fleet,
            &"0".repeat(64),
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(fs::read(&paths.plan).unwrap(), before[0]);
    assert_eq!(fs::read(&paths.journal).unwrap(), before[1]);
    assert_eq!(fs::read(&paths.state).unwrap(), before[2]);
    let journal = fixture_journal(&plan);
    let mut changed = before[2].clone();
    changed.push(b' ');
    write_bytes(&paths.state, &changed).unwrap();
    assert!(matches!(
        adopt(paths, &plan, &journal),
        Err(EnsureStateError::ActivationResetAdoptionConflict)
    ));
    assert!(!marker_path(paths).exists());
    assert_eq!(fs::read(&paths.plan).unwrap(), before[0]);
    assert_eq!(fs::read(&paths.journal).unwrap(), before[1]);
    write_bytes(&paths.state, &before[2]).unwrap();
    adopt(paths, &plan, &journal).unwrap();
    assert_eq!(read_plan(paths).unwrap(), Some(plan));
    assert_eq!(
        crate::fleet_ensure::ops::read_journal(paths).unwrap(),
        Some(journal)
    );
    assert!(review(paths).unwrap().is_none());
    let completed = marker(paths).unwrap().unwrap();
    assert!(completed.complete);
    for (index, (_, digest)) in source_documents(paths, &completed).into_iter().enumerate() {
        assert_eq!(
            exact_bytes(&object_path(paths, digest), digest).unwrap(),
            before[index]
        );
    }
    // Every crash boundary after durable intent converges to the same replacement pair.
    let after = [
        fs::read(&paths.plan).unwrap(),
        fs::read(&paths.journal).unwrap(),
    ];
    for replaced in 0..=2 {
        let mut interrupted = completed.clone();
        interrupted.complete = false;
        write_current(&marker_path(paths), &interrupted).unwrap();
        write_bytes(
            &paths.plan,
            if replaced >= 1 { &after[0] } else { &before[0] },
        )
        .unwrap();
        write_bytes(
            &paths.journal,
            if replaced >= 2 { &after[1] } else { &before[1] },
        )
        .unwrap();
        recover(paths).unwrap();
        assert_eq!(fs::read(&paths.plan).unwrap(), after[0]);
        assert_eq!(fs::read(&paths.journal).unwrap(), after[1]);
        assert!(marker(paths).unwrap().unwrap().complete);
    }
    // A completed marker must never roll back a journal that has since recorded effects.
    write_bytes(&paths.journal, b"later journal progress").unwrap();
    recover(paths).unwrap();
    assert_eq!(fs::read(&paths.journal).unwrap(), b"later journal progress");
    assert!(review(paths).unwrap().is_none());
}

#[test]
fn interrupted_adoption_rejects_drift_before_replacing_either_document() {
    for changed in ["none", "plan", "journal", "state", "archive", "replacement"] {
        let root = crate::test_support::temp_dir("activation-adoption-conflict");
        let paths = EnsurePaths::under(&root, "local", "source");
        let before = [
            b"source plan".as_slice(),
            b"source journal",
            b"source state",
        ];
        let source = FleetActivationSourceRecord {
            infrastructure: Vec::new(),
            operator: "operator".into(),
            cycles_ledger: "ledger".into(),
            initial_controlled_cycles: 0,
            maximum_execution_burn_cycles: 0,
            initial_estate_funding_cycles_by_root: BTreeMap::new(),
            operation_id: sha256_hex(b"completed source"),
            plan_sha256: sha256_hex(b"plan reference"),
            plan_document_sha256: sha256_hex(before[0]),
            journal_document_sha256: sha256_hex(before[1]),
            state_document_sha256: sha256_hex(before[2]),
            provisioning: crate::fleet_ensure::tests::typed_protocol_action(&sha256_hex(
                b"completed source",
            )),
            registry_preparations: Vec::new(),
            stores: Vec::new(),
        };
        let replacement = fixture_plan(source);
        let replacement_plan = serde_json::to_vec(&replacement).unwrap();
        let replacement_journal = serde_json::to_vec(&fixture_journal(&replacement)).unwrap();
        let intent = ActivationResetAdoptionRecord {
            source_plan_sha256: sha256_hex(before[0]),
            source_journal_sha256: sha256_hex(before[1]),
            source_state_sha256: sha256_hex(before[2]),
            replacement_plan_sha256: sha256_hex(&replacement_plan),
            replacement_journal_sha256: sha256_hex(&replacement_journal),
            complete: false,
        };
        for (index, (path, digest)) in source_documents(&paths, &intent).into_iter().enumerate() {
            write_bytes(path, before[index]).unwrap();
            retain(&paths, digest, before[index]).unwrap();
        }
        retain(&paths, &intent.replacement_plan_sha256, &replacement_plan).unwrap();
        retain(
            &paths,
            &intent.replacement_journal_sha256,
            &replacement_journal,
        )
        .unwrap();
        write_current(&marker_path(&paths), &intent).unwrap();
        if changed == "none" {
            recover(&paths).unwrap();
            assert_eq!(fs::read(&paths.plan).unwrap(), replacement_plan);
            assert_eq!(fs::read(&paths.journal).unwrap(), replacement_journal);
            fs::remove_dir_all(root).unwrap();
            continue;
        }
        let target = match changed {
            "plan" => paths.plan.clone(),
            "journal" => paths.journal.clone(),
            "state" => paths.state.clone(),
            "archive" => object_path(&paths, &intent.source_state_sha256),
            "replacement" => object_path(&paths, &intent.replacement_journal_sha256),
            _ => unreachable!(),
        };
        write_bytes(&target, b"changed").unwrap();
        let original_plan = fs::read(&paths.plan).unwrap();
        let original_journal = fs::read(&paths.journal).unwrap();
        assert!(matches!(
            recover(&paths),
            Err(EnsureStateError::ActivationResetAdoptionConflict)
        ));
        assert_eq!(fs::read(&paths.plan).unwrap(), original_plan);
        assert_eq!(fs::read(&paths.journal).unwrap(), original_journal);
        assert!(!marker(&paths).unwrap().unwrap().complete);
        fs::remove_dir_all(root).unwrap();
    }
}

fn fixture_plan(source: FleetActivationSourceRecord) -> FleetEnsurePlan {
    let operation_id = "b".repeat(64);
    let mut plan = FleetEnsurePlan {
        canisters: Vec::new(),
        continuation: None,
        conservation: CycleConservation {
            estate_funding_domains: Vec::new(),
            expected_post_operation_cycles: 0,
            maximum_execution_burn_cycles: 0,
            maximum_new_funding_cycles: 0,
            maximum_operator_debit_cycles: 0,
            maximum_unavoidable_fee_cycles: 0,
            observed_controlled_cycles: 0,
            retained_in_reused_canisters_cycles: 0,
            scheduled_transfer_cycles: 0,
        },
        desired_sha256: "c".repeat(64),
        environment: "local".into(),
        fleet: "source".into(),
        operation_id: operation_id.clone(),
        plan_sha256: String::new(),
        planned_at_time: 1,
        protocol_actions: Vec::new(),
        recovery_review: None,
        reinstall: Some(Box::new(FleetReinstallRecord {
            target_artifacts_sha256: None,
            source: None,
            operation_id,
            source_operation_id: source.operation_id.clone(),
            authorities: Vec::new(),
            assets: Vec::new(),
            activation_reset: Some(Box::new(FleetActivationResetRecord {
                preparation: None,
                source,
                roots: Vec::new(),
            })),
        })),
        root_reinstall_bindings: Vec::new(),
        root_start_authority: None,
        reviewed_desired: None,
        schema_version: 1,
        scope: FleetEnsurePlanScope::ReinstallPreparation,
        terminal_inventory_operation_id: None,
    };
    plan.plan_sha256 = expected_plan_sha256(&plan);
    plan
}

fn fixture_journal(plan: &FleetEnsurePlan) -> FleetEnsureJournalRecord {
    FleetEnsureJournalRecord {
        funding_reviews: Vec::new(),
        successor_phases: Vec::new(),
        completion: FleetEnsureCompletion::InProgress,
        estate_funding_required: None,
        effects: Vec::new(),
        fleet: plan.fleet.clone(),
        initial_controlled_cycles: 0,
        initial_estate_funding_cycles_by_root: BTreeMap::new(),
        initial_operator_cycles: 0,
        operation_id: plan.operation_id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        schema_version: 1,
        stalled_observations: 0,
    }
}
