use super::*;
use crate::fleet_ensure::ops::{read_current, write_current};
use serde_json::json;

fn fixture() -> EnsurePaths {
    let paths = crate::fleet_ensure::ops::operation_selection::tests::retained();
    let phase = paths
        .plan
        .with_file_name("phases")
        .join("opaque-phase.json");
    fs::create_dir_all(phase.parent().unwrap()).unwrap();
    let blob = b"archived publication payload";
    let digest = sha256_hex(blob);
    fs::create_dir_all(&paths.content).unwrap();
    fs::write(paths.content.join(&digest), blob).unwrap();
    fs::write(
        phase,
        serde_json::to_vec(&json!({
            "uninterpreted_execution": {"bytes_sha256": digest, "bytes_size": blob.len()}
        }))
        .unwrap(),
    )
    .unwrap();
    paths
}

fn archive(paths: &EnsurePaths) -> PathBuf {
    paths
        .workspace
        .join(".canic/fleet-ensure/history/local/fleet")
}

#[test]
fn completed_archive_retains_exact_opaque_evidence_and_shared_objects() {
    let paths = fixture();
    let before = [&paths.plan, &paths.journal, &paths.state].map(|p| fs::read(p).unwrap());
    let digest = capture(&paths, "local", "fleet").unwrap().unwrap();
    let manifest = archive(&paths)
        .join("operations")
        .join(format!("{digest}.json"));
    let record: OperationArchiveRecord = read_current(&manifest).unwrap().unwrap();
    for (name, hash) in &record.files {
        let source = name.strip_prefix("estate/").map_or_else(
            || paths.content.join(name.strip_prefix("objects/").unwrap()),
            |name| paths.plan.parent().unwrap().join(name),
        );
        assert_eq!(
            fs::read(source).unwrap(),
            fs::read(archive(&paths).join("objects").join(hash)).unwrap()
        );
    }
    assert!(record.files.contains_key("estate/state.json"));
    assert!(record.files.keys().any(|name| name.starts_with("objects/")));
    assert_eq!(capture(&paths, "local", "fleet").unwrap(), Some(digest));
    assert_eq!(
        fs::read_dir(archive(&paths).join("operations"))
            .unwrap()
            .count(),
        1
    );
    assert_eq!(
        before,
        [&paths.plan, &paths.journal, &paths.state].map(|p| fs::read(p).unwrap())
    );
    fs::remove_dir_all(paths.workspace).unwrap();
}

#[test]
fn incomplete_archive_resumes_and_corrupt_objects_cannot_be_published() {
    let paths = fixture();
    let bytes = fs::read(&paths.plan).unwrap();
    let object = archive(&paths).join("objects").join(sha256_hex(&bytes));
    // Interruption after an object but before the final manifest is harmless.
    retain(&object, &bytes).unwrap();
    assert!(capture(&paths, "local", "fleet").unwrap().is_some());
    fs::write(&object, b"changed bytes").unwrap();
    assert!(matches!(
        capture(&paths, "local", "fleet"),
        Err(EnsureStateError::InvalidTerminalSource)
    ));
    assert_eq!(fs::read(&paths.plan).unwrap(), bytes);
    fs::remove_dir_all(paths.workspace).unwrap();
}

#[test]
fn unfinished_operation_creates_no_history() {
    let paths = fixture();
    let mut journal: Value = read_current(&paths.journal).unwrap().unwrap();
    journal["completion"] = json!("in_progress");
    write_current(&paths.journal, &journal).unwrap();
    assert!(capture(&paths, "local", "fleet").unwrap().is_none());
    assert!(!archive(&paths).exists());
    fs::remove_dir_all(paths.workspace).unwrap();
}

#[test]
fn unavailable_historical_publication_content_does_not_require_old_artifacts() {
    let paths = fixture();
    fs::remove_dir_all(&paths.content).unwrap();
    let digest = capture(&paths, "local", "fleet").unwrap().unwrap();
    let manifest = archive(&paths)
        .join("operations")
        .join(format!("{digest}.json"));
    let record: OperationArchiveRecord = read_current(&manifest).unwrap().unwrap();
    assert!(
        record
            .unavailable_objects
            .contains(&sha256_hex(b"archived publication payload"))
    );
    assert!(record.files.contains_key("estate/phases/opaque-phase.json"));
    fs::remove_dir_all(paths.workspace).unwrap();
}

#[cfg(unix)]
#[test]
fn archive_rejects_symlinks_without_following_them() {
    let paths = fixture();
    std::os::unix::fs::symlink(&paths.state, paths.plan.with_file_name("linked-evidence")).unwrap();
    assert!(matches!(
        capture(&paths, "local", "fleet"),
        Err(EnsureStateError::Unsafe { .. })
    ));
    assert!(!archive(&paths).join("operations").exists());
    fs::remove_dir_all(paths.workspace).unwrap();
}

#[test]
#[ignore = "requires an explicitly selected read-only completed estate"]
fn inspect_supplied_completed_operation_without_source_mutation() {
    let source = PathBuf::from(std::env::var("CANIC_COMPLETED_SOURCE_WORKSPACE").unwrap());
    let environment = std::env::var("CANIC_COMPLETED_SOURCE_ENVIRONMENT").unwrap();
    let fleet = std::env::var("CANIC_COMPLETED_SOURCE_FLEET").unwrap();
    let mut paths = EnsurePaths::under(&source, &environment, &fleet);
    // All source paths stay read-only. Only the archive destination points into
    // invocation-owned Canic scratch; no external operation lock is opened.
    paths.workspace = crate::test_support::temp_dir("supplied-completed-archive");
    let before = [&paths.plan, &paths.journal, &paths.state].map(|p| fs::read(p).unwrap());
    let completed = operation_selection::completed(&paths, &environment, &fleet)
        .unwrap()
        .unwrap();
    let digest = capture(&paths, &environment, &fleet).unwrap().unwrap();
    let manifest = paths
        .workspace
        .join(".canic/fleet-ensure/history")
        .join(&environment)
        .join(&fleet)
        .join("operations")
        .join(format!("{digest}.json"));
    let record: OperationArchiveRecord = read_current(&manifest).unwrap().unwrap();
    assert_eq!(record.operation_id.as_ref(), Some(&completed.operation_id));
    assert_eq!(record.plan_sha256.as_ref(), Some(&completed.plan_sha256));
    assert_eq!(
        before,
        [&paths.plan, &paths.journal, &paths.state].map(|p| fs::read(p).unwrap())
    );
    println!(
        "completed operation {}; archived {} evidence files; source unchanged",
        completed.operation_id,
        record.files.len()
    );
    fs::remove_dir_all(paths.workspace).unwrap();
}
