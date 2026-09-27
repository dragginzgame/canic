use super::*;
use crate::{
    fleet_ensure::ops::{read_current, write_current},
    test_support::temp_dir,
};
use serde_json::json;
use std::fs;

pub(super) fn retained() -> EnsurePaths {
    let root = temp_dir("completed-operation-selection");
    let paths = EnsurePaths::under(&root, "local", "fleet");
    // Historical executable payloads are deliberately opaque. Only the terminal
    // envelope participates in selection; no omitted field gets a default.
    write_current(
        &paths.plan,
        &json!({
            "environment": "local", "fleet": "fleet", "scope": "full",
            "operation_id": "a1".repeat(32), "plan_sha256": "b2".repeat(32),
            "reviewed_desired": {"release": "0.110.38"},
            "reinstall": {"retired_contract": true}
        }),
    )
    .unwrap();
    write_current(
        &paths.journal,
        &json!({
            "fleet": "fleet", "operation_id": "a1".repeat(32),
            "plan_sha256": "b2".repeat(32), "completion": "converged",
            "effects": [{"state": "applied", "receipt": "opaque completed receipt"}],
            "successor_phases": [{"retired_contract": true}]
        }),
    )
    .unwrap();
    fs::write(&paths.state, b"opaque historical application state").unwrap();
    paths
}

#[test]
fn completed_contracts_are_not_decoded_as_recovery_work() {
    let paths = retained();
    let originals = [&paths.plan, &paths.journal, &paths.state].map(|path| fs::read(path).unwrap());
    let selected = completed(&paths, "local", "fleet").unwrap().unwrap();
    assert_eq!(selected.operation_id, "a1".repeat(32));
    assert_eq!(selected.completion, FleetEnsureCompletion::Converged);
    assert!(!paths.lock.exists());
    assert!(
        crate::fleet_ensure::workflow::retained_in_progress_plan::<std::io::Error>(
            &paths.workspace,
            "local",
            "fleet"
        )
        .unwrap()
        .is_none()
    );
    assert!(
        crate::fleet_ensure::workflow::retained_reinstall_apply_plan::<std::io::Error>(
            &paths.workspace,
            "local",
            "fleet",
            &"c3".repeat(32)
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(
        originals,
        [&paths.plan, &paths.journal, &paths.state].map(|path| fs::read(path).unwrap())
    );
    fs::remove_dir_all(paths.workspace).unwrap();
}

#[test]
fn unfinished_paid_work_is_never_selected_as_history() {
    let paths = retained();
    let mut journal: Value = read_current(&paths.journal).unwrap().unwrap();
    journal["completion"] = json!("in_progress");
    journal["effects"][0]["state"] = json!("issued");
    write_current(&paths.journal, &journal).unwrap();
    assert!(completed(&paths, "local", "fleet").unwrap().is_none());
    assert!(
        crate::fleet_ensure::workflow::retained_in_progress_plan::<std::io::Error>(
            &paths.workspace,
            "local",
            "fleet"
        )
        .is_err()
    );
    journal["completion"] = json!("converged");
    write_current(&paths.journal, &journal).unwrap();
    assert!(matches!(
        completed(&paths, "local", "fleet"),
        Err(EnsureStateError::InvalidTerminalSource)
    ));
    fs::remove_dir_all(paths.workspace).unwrap();
}

#[test]
fn changed_identity_and_new_pending_plan_cannot_be_retired() {
    let paths = retained();
    for (environment, fleet) in [("staging", "fleet"), ("local", "other")] {
        assert!(matches!(
            completed(&paths, environment, fleet),
            Err(EnsureStateError::InvalidTerminalSource)
        ));
    }
    let mut plan: Value = read_current(&paths.plan).unwrap().unwrap();
    plan["plan_sha256"] = json!("c3".repeat(32));
    write_current(&paths.plan, &plan).unwrap();
    assert!(completed(&paths, "local", "fleet").unwrap().is_none());
    fs::remove_dir_all(paths.workspace).unwrap();
}

#[test]
fn completed_import_is_history_without_decoding_its_receipts() {
    let paths = retained();
    let path = paths.plan.with_file_name("capacity-import.json");
    let mut record = json!({
        "approved": true,
        "plan": {"sources": [{"binding": {"canister_id": "opaque recorded ID"}}]},
        "handoffs": [{"canister_id": "opaque recorded ID", "effect": {"state": "applied"}}],
        "operation": {"publication_started": true, "publication_complete": true,
            "settled_status_candid_hex": "aa", "released_status_candid_hex": "bb"}
    });
    write_current(&path, &record).unwrap();
    let original = fs::read(&path).unwrap();
    assert!(!capacity_import_in_progress(&paths).unwrap());
    assert_eq!(fs::read(&path).unwrap(), original);
    record["operation"]["publication_complete"] = json!(false);
    write_current(&path, &record).unwrap();
    assert!(capacity_import_in_progress(&paths).unwrap());
    record["operation"]["publication_complete"] = json!(true);
    record["handoffs"][0]["effect"]["state"] = json!("issued");
    write_current(&path, &record).unwrap();
    assert!(matches!(
        capacity_import_in_progress(&paths),
        Err(EnsureStateError::InvalidTerminalSource)
    ));
    record["approved"] = json!(false);
    record["handoffs"][0]["effect"] = Value::Null;
    write_current(&path, &record).unwrap();
    assert!(matches!(
        capacity_import_in_progress(&paths),
        Err(EnsureStateError::InvalidTerminalSource)
    ));
    record["operation"] = Value::Null;
    write_current(&path, &record).unwrap();
    assert!(!capacity_import_in_progress(&paths).unwrap());
    fs::remove_dir_all(paths.workspace).unwrap();
}

#[test]
fn completed_historical_shape_selects_current_reset_without_desired_decoding() {
    let paths = retained();
    let select = crate::fleet_ensure::workflow::clean_reinstall::selected;
    assert!(select(&paths.workspace, "local", "fleet", true, false).unwrap());
    assert!(!select(&paths.workspace, "local", "fleet", false, false).unwrap());
    crate::fleet_ensure::ops::retained_contract::check(&paths.workspace, "local", "fleet").unwrap();
    fs::remove_dir_all(paths.workspace).unwrap();
}
