use super::*;
use crate::fleet_ensure::ops::operation_selection::retirement::history;
use crate::fleet_ensure::ops::operation_selection::tests::retained;
use serde_json::json;

#[test]
fn partial_import_and_unreadable_application_state_do_not_require_old_completion() {
    let paths = retained();
    fs::write(&paths.plan, b"obsolete plan bytes").unwrap();
    fs::write(&paths.state, b"partly written application state").unwrap();
    write_current(
        &paths.journal,
        &json!({"completion": "in_progress", "effects": [
            {"state": "applied"}
        ]}),
    )
    .unwrap();
    write_current(
        &paths.plan.with_file_name("capacity-import.json"),
        &json!({
            "approved": true, "handoffs": [{"effect": null}],
            "operation": {"submissions": {"advance": 2}}, "reservation": {"retained": true}
        }),
    )
    .unwrap();
    require_reconciled_effects(&paths).unwrap();
    let original = fs::read(&paths.plan).unwrap();
    let desired = desired();
    let lock = ops::lock_fleet_file_without_recovery(&paths).unwrap();
    ResetRetirement {
        _lock: lock,
        paths: &paths,
        environment: "local".into(),
        fleet: "fleet".into(),
        replacement_sha256: sha256_hex(&serde_json::to_vec(&desired).unwrap()),
        has_predecessor: true,
    }
    .finish(&desired, Path::new("policy.toml"), Path::new("seed.toml"))
    .unwrap();
    assert!(!paths.plan.exists());
    assert!(!paths.journal.exists());
    assert!(!paths.state.exists());
    assert_eq!(
        fs::read(
            history(&paths)
                .unwrap()
                .join("objects")
                .join(sha256_hex(&original))
        )
        .unwrap(),
        original
    );
    assert!(paths.lock.is_file());
    assert_eq!(
        ops::clean_reinstall::read(&paths)
            .unwrap()
            .unwrap()
            .desired
            .desired(),
        &desired
    );
    fs::remove_dir_all(paths.workspace).unwrap();
}

#[test]
fn current_retry_keeps_allowances_and_unpaid_successor_but_not_a_later_ordinary_plan() {
    let (paths, desired) = current_fixture();
    let mut plan = operation_selection::read(&paths.plan).unwrap().unwrap();
    let mut setup = plan.clone();
    setup["scope"] = json!("infrastructure_bootstrap");
    let setup_digest = setup["plan_sha256"].as_str().unwrap();
    write_current(
        &paths
            .plan
            .with_file_name("infrastructure-bootstrap-completed")
            .join(setup_digest)
            .join("plan.json"),
        &setup,
    )
    .unwrap();
    assert!(!current_selection(&paths, &desired).unwrap());
    plan["plan_sha256"] = json!("c3".repeat(32));
    write_current(&paths.plan, &plan).unwrap();
    // The still-converged predecessor journal is not completion of this unpaid successor.
    assert!(current_selection(&paths, &desired).unwrap());
    let mut journal = operation_selection::read(&paths.journal).unwrap().unwrap();
    journal["plan_sha256"] = plan["plan_sha256"].clone();
    journal["completion"] = json!("in_progress");
    journal["effects"] = json!([{
        "state": "intent", "maintenance_attempts": 2, "publication_attempts": 0,
        "action_sha256": "e5".repeat(32), "created_principal": null,
        "destination_post_cycles": null, "destination_pre_cycles": null,
        "post_cycles": null, "pre_cycles": null, "pre_canister_version": null,
        "progress_identity": null, "receipt": null
    }]);
    write_current(&paths.journal, &journal).unwrap();
    let before = fs::read(&paths.journal).unwrap();
    assert!(
        ResetRetirement::prepare(&paths, &desired)
            .unwrap()
            .is_none()
    );
    assert_eq!(fs::read(&paths.journal).unwrap(), before);
    let mut changed = desired.clone();
    changed.maximum_update_burn_cycles = "2".into();
    assert!(!current_selection(&paths, &changed).unwrap());
    assert!(matches!(
        ResetRetirement::prepare(&paths, &changed),
        Err(EnsureStateError::ResetUncertainEffect { .. })
    ));
    plan["operation_id"] = json!("d4".repeat(32));
    write_current(&paths.plan, &plan).unwrap();
    assert!(!current_selection(&paths, &desired).unwrap());
    fs::remove_dir_all(paths.workspace).unwrap();
}

#[test]
fn matching_selection_does_not_require_damaged_current_documents_to_be_repaired() {
    let (paths, desired) = current_fixture();
    let mut plan = operation_selection::read(&paths.plan).unwrap().unwrap();
    plan["scope"] = json!("infrastructure_bootstrap");
    plan["plan_sha256"] = json!("c3".repeat(32));
    write_current(&paths.plan, &plan).unwrap();
    assert!(current_selection(&paths, &desired).unwrap());
    for path in [&paths.plan, &paths.state, &paths.journal] {
        let original = fs::read(path).unwrap();
        let mut damaged: serde_json::Value = serde_json::from_slice(&original).unwrap();
        damaged["schema_version"] = json!(999);
        write_current(path, &damaged).unwrap();
        assert!(!current_selection(&paths, &desired).unwrap());
        assert!(
            ResetRetirement::prepare(&paths, &desired)
                .unwrap()
                .is_some()
        );
        fs::write(path, original).unwrap();
    }
    fs::remove_dir_all(paths.workspace).unwrap();
}

fn current_fixture() -> (EnsurePaths, DesiredFleet) {
    use crate::fleet_ensure::{tests::protocol_tranche_fixture, workflow};
    let mut fixture = protocol_tranche_fixture(Vec::new());
    fixture.desired.protocol = None;
    let desired = fixture.desired;
    let digest = "12".repeat(32);
    let plan = workflow::plan(
        &fixture.root,
        &desired,
        &digest,
        &desired.fleet,
        1_800_000_000_000_000_000,
        &mut fixture.platform,
    )
    .unwrap()
    .plan;
    workflow::apply(
        &fixture.root,
        &desired,
        &digest,
        &desired.fleet,
        &plan.plan_sha256,
        &mut fixture.platform,
    )
    .unwrap();
    let paths = EnsurePaths::under(&fixture.root, &desired.environment, &desired.fleet);
    ops::clean_reinstall::bind(
        &paths,
        &desired,
        Path::new("policy.toml"),
        Path::new("seed.toml"),
    )
    .unwrap();
    (paths, desired)
}

#[test]
fn replacement_artifacts_are_checked_before_unreadable_paid_evidence() {
    let paths = retained();
    fs::write(&paths.journal, b"unreadable paid evidence").unwrap();
    let mut desired = desired();
    desired.canisters.push(
        serde_json::from_value(json!({
            "controllers": [], "drain": null, "initial_cycles": "0", "minimum_cycles": "0",
            "init_arg": null, "init_candid": null, "kind": "auxiliary", "name": "artifact",
            "parent": null, "presence": "present", "principal": null, "replace": false,
            "subnet": "aaaaa-aa", "wasm": "missing-current.wasm"
        }))
        .unwrap(),
    );
    assert!(matches!(
        ResetRetirement::prepare(&paths, &desired),
        Err(EnsureStateError::ArtifactUnavailable { .. })
    ));
    assert_eq!(
        fs::read(&paths.journal).unwrap(),
        b"unreadable paid evidence"
    );
    assert!(!paths.lock.exists());
    fs::remove_dir_all(paths.workspace).unwrap();
}

fn desired() -> DesiredFleet {
    serde_json::from_value(json!({
        "canisters": [], "cycles_ledger": "aaaaa-aa", "environment": "local", "fleet": "fleet",
        "ledger_fee_cycles": "0", "management_creation_fee_cycles": "0", "material_cycle_threshold": "0",
        "maximum_observation_burn_cycles": "1", "maximum_stalled_observations": 2,
        "maximum_update_burn_cycles": "1", "operator": "aaaaa-aa", "schema_version": 1, "treasury": "treasury"
    })).unwrap()
}

#[test]
fn release_read_reservations_allow_reset_but_unresolved_effects_do_not() {
    use crate::fleet_ensure::{
        ops::release::{
            prepare_review,
            reservation::{ReleaseObservationJournal, tests::seed},
        },
        policy::release::tests::fixture,
    };

    let paths = retained();
    let (review, observed) = fixture();
    let review = prepare_review(review, &observed).unwrap();
    seed(&paths, &review);
    let mut owner = ReleaseObservationJournal::attach(&paths, &review, &observed).unwrap();
    // Losing a read response retains its allowance without an uncertain transfer.
    let _ = owner
        .reserve(review.sources[0].binding.canister_id)
        .unwrap();
    drop(owner);
    let bytes = fs::read(&paths.journal).unwrap();
    let mut journal = operation_selection::read(&paths.journal).unwrap().unwrap();
    assert!(
        journal["release"]["reserved_paid_calls"]
            .as_object()
            .unwrap()
            .values()
            .any(|calls| calls.as_u64() == Some(4))
    );
    require_reconciled_effects(&paths).unwrap();
    assert_eq!(fs::read(&paths.journal).unwrap(), bytes);

    for state in ["intent", "issued"] {
        journal["effects"] = json!([{"state": state}]);
        write_current(&paths.journal, &journal).unwrap();
        let bytes = fs::read(&paths.journal).unwrap();
        assert!(matches!(
            require_reconciled_effects(&paths),
            Err(EnsureStateError::ResetUncertainEffect { .. })
        ));
        assert_eq!(fs::read(&paths.journal).unwrap(), bytes);
    }
    journal["effects"] = json!([{"state": "applied"}]);
    write_current(&paths.journal, &journal).unwrap();
    require_reconciled_effects(&paths).unwrap();
    fs::remove_dir_all(paths.workspace).unwrap();
}

#[test]
fn uncertain_paid_effects_remain_reconcilable_and_unchanged() {
    for journal in [
        json!({"effects": [{"state": "issued"}]}),
        json!({"effects": [{"state": "intent"}]}),
        json!({"effects": [null]}),
        json!({"effects": [], "funding_reviews": {"unreadable": true}}),
        json!({"effects": [], "funding_reviews": [{"effect": {"state": "issued"}}]}),
        json!({"effects": [], "funding_reviews": [{"effect": null, "operator_mint": {
            "transfer_argument": [1,2], "receipt": null
        }}]}),
    ] {
        let paths = retained();
        write_current(&paths.journal, &journal).unwrap();
        let bytes = fs::read(&paths.journal).unwrap();
        assert!(matches!(
            require_reconciled_effects(&paths),
            Err(EnsureStateError::ResetUncertainEffect { .. })
        ));
        assert_eq!(fs::read(&paths.journal).unwrap(), bytes);
        fs::remove_dir_all(paths.workspace).unwrap();
    }
    let paths = retained();
    write_current(
        &paths.plan.with_file_name("capacity-import.json"),
        &json!({
            "approved": true, "handoffs": [{"effect": {"state": "issued"}}]
        }),
    )
    .unwrap();
    assert!(matches!(
        require_reconciled_effects(&paths),
        Err(EnsureStateError::ResetUncertainEffect { .. })
    ));
    fs::write(&paths.journal, b"unreadable paid-effect record").unwrap();
    assert!(matches!(
        require_reconciled_effects(&paths),
        Err(EnsureStateError::ResetUncertainEffect { .. })
    ));
    fs::remove_dir_all(paths.workspace).unwrap();
}

#[test]
fn current_reset_keeps_nondefault_paths_and_rejects_changed_publication_destinations() {
    let (paths, desired) = current_fixture();
    let policy = paths.workspace.join(".tools/selected policy.toml");
    let seed = paths.workspace.join(".tools/selected estate.toml");
    let record = CleanReinstallRecord {
        schema_version: 1,
        desired: crate::fleet_ensure::model::ReviewedDesiredFleetRecord::capture(&desired),
        policy: policy.clone(),
        seed: seed.clone(),
    };
    write_current(&paths.plan.with_file_name("clean-reinstall.json"), &record).unwrap();
    let retained = ops::clean_reinstall::read(&paths).unwrap().unwrap();
    ops::clean_reinstall::validate_inputs(&retained, &policy, &seed).unwrap();
    fs::create_dir_all(policy.parent().unwrap().join("nested")).unwrap();
    fs::write(&policy, []).unwrap();
    fs::write(&seed, []).unwrap();
    ops::clean_reinstall::validate_inputs(
        &retained,
        &policy
            .parent()
            .unwrap()
            .join("nested/../selected policy.toml"),
        &seed,
    )
    .unwrap();
    for (policy, seed) in [
        (paths.workspace.join("wrong.toml"), seed),
        (policy, paths.workspace.join("wrong.toml")),
    ] {
        assert!(matches!(
            ops::clean_reinstall::validate_inputs(&retained, &policy, &seed),
            Err(EnsureStateError::ResetInputConflict { .. })
        ));
        assert_eq!(ops::clean_reinstall::read(&paths).unwrap().unwrap(), record);
    }
    fs::remove_dir_all(paths.workspace).unwrap();
}
