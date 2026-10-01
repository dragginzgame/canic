use super::*;
use crate::fleet_ensure::ops::operation_selection::tests::retained;
use serde_json::json;

#[test]
fn completed_retirement_archives_original_bytes_and_preserves_lock() {
    let paths = retained();
    fs::write(&paths.lock, b"same lock inode").unwrap();
    let original = fs::read(&paths.journal).unwrap();
    begin(&paths, "local", "fleet", &"c3".repeat(32)).unwrap();
    assert!(!paths.plan.exists());
    assert!(!paths.journal.exists());
    assert!(!paths.state.exists());
    assert_eq!(fs::read(&paths.lock).unwrap(), b"same lock inode");
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
    recover(&paths).unwrap();
    assert!(!paths.plan.exists());
    fs::remove_dir_all(paths.workspace).unwrap();
}

#[test]
fn retirement_resumes_each_removal_boundary_and_refuses_changed_bytes() {
    for removed in 0..=3 {
        let paths = retained();
        let archive_sha256 = operation_selection::archive::capture(&paths, "local", "fleet")
            .unwrap()
            .unwrap();
        let record = CompletedOperationRetirementRecord {
            schema_version: 1,
            environment: "local".into(),
            fleet: "fleet".into(),
            archive_sha256,
            replacement_sha256: "c3".repeat(32),
        };
        write_current(&intent_path(&paths).unwrap(), &record).unwrap();
        for path in [&paths.plan, &paths.journal, &paths.state]
            .into_iter()
            .take(removed)
        {
            fs::remove_file(path).unwrap();
        }
        if removed == 0 {
            let original = fs::read(&paths.state).unwrap();
            fs::write(&paths.state, b"new work must survive").unwrap();
            assert!(recover(&paths).is_err());
            assert!(paths.plan.exists());
            fs::write(&paths.state, original).unwrap();
        }
        recover(&paths).unwrap();
        recover(&paths).unwrap();
        assert!(!paths.plan.exists() && !paths.journal.exists() && !paths.state.exists());
        assert!(!intent_path(&paths).unwrap().exists());
        fs::remove_dir_all(paths.workspace).unwrap();
    }
}

#[test]
fn unfinished_import_or_activation_handoff_cannot_be_retired() {
    let paths = retained();
    let import = paths.plan.with_file_name("capacity-import.json");
    write_current(
        &import,
        &json!({"approved": true, "handoffs": [], "operation": null}),
    )
    .unwrap();
    assert!(begin(&paths, "local", "fleet", &"c3".repeat(32)).is_err());
    assert!(paths.plan.exists());
    fs::remove_file(import).unwrap();
    write_current(
        &paths.plan.with_file_name("activation-reset-adoption.json"),
        &json!({"complete": false}),
    )
    .unwrap();
    assert!(begin(&paths, "local", "fleet", &"c3".repeat(32)).is_err());
    assert!(paths.plan.exists());
    fs::remove_dir_all(paths.workspace).unwrap();
}
