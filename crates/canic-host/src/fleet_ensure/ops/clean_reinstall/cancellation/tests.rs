use super::*;
use crate::{fleet_ensure::ops::lock_operation, test_support::temp_dir};
use serde_json::{Value, json};

fn unpaid() -> EnsurePaths {
    let paths = EnsurePaths::under(&temp_dir("unpaid-reset-cancellation"), "local", "fleet");
    let plan = json!({"environment": "local", "fleet": "fleet", "scope": "infrastructure_bootstrap",
        "operation_id": "a1".repeat(32), "plan_sha256": "b2".repeat(32),
        "reviewed_desired": {"opaque_application_contract": true}});
    write_current(&paths.plan, &plan).unwrap();
    write_current(
        &paths.plan.with_file_name("clean-reinstall.json"),
        &json!({"opaque_reviewed_application": true}),
    )
    .unwrap();
    write_current(
        &inspection(&paths),
        &json!({
            "schema_version": 1, "plan_sha256": "b2".repeat(32), "review_attempts": 1,
            "apply_attempts": 0, "terminal_attempts": 0, "registration_attempts": 0,
            "registration_plan_sha256": null, "effect_observations": { "effect": 0 }
        }),
    )
    .unwrap();
    paths
}

fn inspection(paths: &EnsurePaths) -> PathBuf {
    paths
        .plan
        .with_file_name("infrastructure-bootstrap-inspections")
        .join(format!("{}.json", "e5".repeat(32)))
}

#[test]
fn cancellation_archives_exact_unpaid_review_and_replays_without_a_new_owner() {
    let paths = unpaid();
    let original = fs::read(&paths.plan).unwrap();
    let record = cancel(&paths, "local", "fleet", &"b2".repeat(32)).unwrap();
    assert!(!paths.plan.exists());
    assert!(!paths.journal.exists());
    assert!(ready_for_review(&paths).unwrap());
    let history = retirement::history(&paths).unwrap();
    assert_eq!(
        fs::read(
            history
                .join("objects")
                .join(canic_core::cdk::utils::hash::sha256_hex(&original))
        )
        .unwrap(),
        original
    );
    assert_eq!(
        cancel(&paths, "local", "fleet", &record.plan_sha256).unwrap(),
        record
    );
    let selected = paths.plan.with_file_name("clean-reinstall.json");
    write_current(&selected, &json!({"new_review_selected": true})).unwrap();
    assert!(!ready_for_review(&paths).unwrap());
    assert!(matches!(
        cancel(&paths, "local", "fleet", &record.plan_sha256),
        Err(EnsureStateError::ResetReviewConflict)
    ));
    fs::remove_file(selected).unwrap();
    assert!(matches!(
        cancel(&paths, "local", "fleet", &"c3".repeat(32)),
        Err(EnsureStateError::ResetReviewConflict)
    ));
    fs::write(&paths.plan, &original).unwrap();
    let mut next: Value = serde_json::from_slice(&original).unwrap();
    next["plan_sha256"] = json!("c3".repeat(32));
    write_current(&paths.plan, &next).unwrap();
    let before = fs::read(&paths.plan).unwrap();
    assert!(matches!(
        cancel(&paths, "local", "fleet", &record.plan_sha256),
        Err(EnsureStateError::ResetReviewConflict)
    ));
    assert_eq!(fs::read(&paths.plan).unwrap(), before);
    assert!(!ready_for_review(&paths).unwrap());
    fs::remove_dir_all(paths.workspace).unwrap();
}

#[test]
fn cancellation_rejects_stale_digests_and_all_execution_or_side_operation_files() {
    let paths = unpaid();
    let before = fs::read(&paths.plan).unwrap();
    assert!(matches!(
        cancel(&paths, "local", "fleet", &"c3".repeat(32)),
        Err(EnsureStateError::ResetReviewConflict)
    ));
    for name in [
        "journal.json",
        "state.json",
        "capacity-import.json",
        "operator-mint.json",
        "completed-estate-publication.json",
        "completed-preparation-journal.json",
        "activation-reset-adoption.json",
        "unknown-owner.json",
    ] {
        let path = paths.plan.with_file_name(name);
        fs::write(&path, b"issued evidence must remain opaque").unwrap();
        assert!(
            matches!(cancel(&paths, "local", "fleet", &"b2".repeat(32)), Err(EnsureStateError::ResetReviewEffectEvidence { path: rejected }) if rejected == path)
        );
        assert_eq!(fs::read(&paths.plan).unwrap(), before);
        assert!(!intent_path(&paths).unwrap().exists());
        fs::remove_file(path).unwrap();
    }
    for field in [
        "apply_attempts",
        "terminal_attempts",
        "registration_attempts",
    ] {
        let path = inspection(&paths);
        let mut record: Value = read_current(&path).unwrap().unwrap();
        record[field] = json!(1);
        write_current(&path, &record).unwrap();
        assert!(matches!(
            cancel(&paths, "local", "fleet", &"b2".repeat(32)),
            Err(EnsureStateError::ResetReviewEffectEvidence { .. })
        ));
        record[field] = json!(0);
        write_current(&path, &record).unwrap();
    }
    let path = inspection(&paths);
    let mut record: Value = read_current(&path).unwrap().unwrap();
    record["effect_observations"]["effect"] = json!(1);
    write_current(&path, &record).unwrap();
    assert!(matches!(
        cancel(&paths, "local", "fleet", &"b2".repeat(32)),
        Err(EnsureStateError::ResetReviewEffectEvidence { .. })
    ));
    assert_eq!(fs::read(&paths.plan).unwrap(), before);
    fs::remove_dir_all(paths.workspace).unwrap();
}

#[test]
fn cancellation_recovers_each_removal_boundary_and_fences_other_writers() {
    let paths = unpaid();
    let files = [
        &paths.plan,
        &paths.plan.with_file_name("clean-reinstall.json"),
        &inspection(&paths),
    ]
    .map(Clone::clone);
    fs::remove_dir_all(paths.workspace).unwrap();
    for removed in 0..=files.len() {
        let paths = unpaid();
        let archive_sha256 =
            archive::capture_review(&paths, "local", "fleet", &"a1".repeat(32), &"b2".repeat(32))
                .unwrap();
        let record = CleanReinstallCancellationRecord {
            schema_version: 1,
            environment: "local".into(),
            fleet: "fleet".into(),
            operation_id: "a1".repeat(32),
            plan_sha256: "b2".repeat(32),
            archive_sha256,
        };
        write_current(&intent_path(&paths).unwrap(), &record).unwrap();
        let extra = paths
            .plan
            .with_file_name("infrastructure-bootstrap-surveys")
            .join(format!("{}.json", "f6".repeat(32)));
        write_current(&extra, &json!({"new_observation": true})).unwrap();
        let original = fs::read(&paths.plan).unwrap();
        assert!(matches!(
            cancel(&paths, "local", "fleet", &record.plan_sha256),
            Err(EnsureStateError::InvalidTerminalSource)
        ));
        assert_eq!(fs::read(&paths.plan).unwrap(), original);
        fs::remove_file(&extra).unwrap();
        fs::remove_dir(extra.parent().unwrap()).unwrap();
        let selection = paths.plan.with_file_name("clean-reinstall.json");
        let original_selection = fs::read(&selection).unwrap();
        fs::write(&selection, b"changed selection").unwrap();
        assert!(matches!(
            cancel(&paths, "local", "fleet", &record.plan_sha256),
            Err(EnsureStateError::InvalidTerminalSource)
        ));
        assert_eq!(fs::read(&paths.plan).unwrap(), original);
        fs::write(selection, original_selection).unwrap();
        let files = [
            paths.plan.clone(),
            paths.plan.with_file_name("clean-reinstall.json"),
            inspection(&paths),
        ];
        for file in files.iter().take(removed) {
            fs::remove_file(file).unwrap();
        }
        assert!(matches!(
            lock_operation(&paths),
            Err(EnsureStateError::ResetReviewCancellationPending { .. })
        ));
        assert!(matches!(
            cancel(&paths, "local", "fleet", &"c3".repeat(32)),
            Err(EnsureStateError::ResetReviewConflict)
        ));
        let unexpected = paths.plan.with_file_name("journal.json");
        fs::write(&unexpected, b"new issued intent").unwrap();
        assert!(matches!(
            cancel(&paths, "local", "fleet", &record.plan_sha256),
            Err(EnsureStateError::ResetReviewEffectEvidence { .. })
        ));
        fs::remove_file(unexpected).unwrap();
        assert_eq!(
            cancel(&paths, "local", "fleet", &record.plan_sha256).unwrap(),
            record
        );
        assert!(ready_for_review(&paths).unwrap());
        assert!(!intent_path(&paths).unwrap().exists());
        fs::remove_dir_all(paths.workspace).unwrap();
    }
}
