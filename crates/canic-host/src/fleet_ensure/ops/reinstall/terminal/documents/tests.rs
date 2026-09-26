//! Historical completion snapshots preserve missing fields and never authorize execution.

use super::*;
use crate::test_support::temp_dir;
use std::fs;

fn fixture(root: &Path) -> EnsurePaths {
    let paths = EnsurePaths::under(root, "staging", "fleet");
    fs::create_dir_all(paths.plan.with_file_name("phases")).unwrap();
    let plan = serde_json::json!({
        "schema_version": 1, "environment": "staging", "fleet": "fleet",
        "operation_id": "ab".repeat(32), "plan_sha256": "cd".repeat(32),
        "desired_sha256": "ef".repeat(32), "planned_at_time": 17,
        "reviewed_desired": {"desired": {"bootstrap": {"coordinator": "historical"}}},
    });
    let mut phase = plan.clone();
    phase["plan_sha256"] = "12".repeat(32).into();
    let journal = serde_json::json!({
        "schema_version": 1, "fleet": "fleet", "completion": "converged",
        "operation_id": plan["operation_id"], "plan_sha256": plan["plan_sha256"],
        "successor_phases": [{"plan_sha256": phase["plan_sha256"], "execution_burn_before_phase": "7"}],
    });
    let state = serde_json::json!({
        "schema_version": 1, "fleet": "fleet",
        "active_registry": {"authority": {"binding": {"coordinator": "historical"}}},
    });
    for (path, value) in [
        (paths.plan.clone(), plan),
        (paths.journal.clone(), journal),
        (paths.state.clone(), state),
        (phase_path(&paths), phase),
    ] {
        fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    }
    paths
}

fn phase_path(paths: &EnsurePaths) -> std::path::PathBuf {
    paths
        .plan
        .with_file_name("phases")
        .join(format!("{}.json", "12".repeat(32)))
}

#[test]
fn historical_authority_is_captured_without_defaults_or_writes() {
    let root = temp_dir("completed-documents");
    let paths = fixture(&root);
    let before: Vec<_> = [
        &paths.plan,
        &paths.journal,
        &paths.state,
        &phase_path(&paths),
    ]
    .into_iter()
    .map(|path| (path.clone(), fs::read(path).unwrap()))
    .collect();
    let snapshot = read(&paths, "staging", "fleet").unwrap();
    assert!(
        snapshot
            .plan
            .pointer("/reviewed_desired/desired/bootstrap/recovery_controllers")
            .is_none()
    );
    assert!(
        snapshot
            .state
            .pointer("/active_registry/authority/binding/recovery_controllers")
            .is_none()
    );
    assert_eq!(
        snapshot.bindings.plan_document_sha256,
        sha256_hex(&before[0].1)
    );
    assert_eq!(
        snapshot.bindings.phase_document_sha256[&"12".repeat(32)],
        sha256_hex(&before[3].1)
    );
    for (path, bytes) in before {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    assert!(!paths.lock.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn incomplete_and_mixed_source_documents_are_rejected() {
    for (document, pointer, changed) in [
        ("journal", "/completion", Value::from("in_progress")),
        ("journal", "/operation_id", Value::from("00".repeat(32))),
        ("state", "/fleet", Value::from("another-fleet")),
        ("phase", "/desired_sha256", Value::from("00".repeat(32))),
        ("phase", "/planned_at_time", Value::from(18)),
        (
            "phase",
            "/reviewed_desired/desired/bootstrap/coordinator",
            Value::from("changed"),
        ),
    ] {
        let root = temp_dir("completed-document-conflict");
        let paths = fixture(&root);
        let path = match document {
            "journal" => paths.journal.clone(),
            "state" => paths.state.clone(),
            "phase" => phase_path(&paths),
            _ => unreachable!(),
        };
        let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        *value.pointer_mut(pointer).unwrap() = changed;
        fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(matches!(
            read(&paths, "staging", "fleet"),
            Err(EnsureStateError::InvalidTerminalSource)
        ));
        assert!(!paths.lock.exists());
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn phase_references_reject_duplicate_and_non_digest_paths() {
    for duplicate in [false, true] {
        let root = temp_dir("completed-document-references");
        let paths = fixture(&root);
        let mut journal: Value =
            serde_json::from_slice(&fs::read(&paths.journal).unwrap()).unwrap();
        if duplicate {
            let reference = journal["successor_phases"][0].clone();
            journal["successor_phases"]
                .as_array_mut()
                .unwrap()
                .push(reference);
        } else {
            journal["successor_phases"][0]["plan_sha256"] = "../../outside".into();
        }
        fs::write(&paths.journal, serde_json::to_vec(&journal).unwrap()).unwrap();
        assert!(matches!(
            read(&paths, "staging", "fleet"),
            Err(EnsureStateError::InvalidTerminalSource)
        ));
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn byte_budget_and_symlinks_cannot_expand_the_snapshot() {
    let root = temp_dir("completed-document-bounds");
    let paths = fixture(&root);
    let mut remaining = 1;
    assert!(matches!(
        bytes(&paths.plan, &mut remaining),
        Err(EnsureStateError::Io { .. })
    ));
    assert_eq!(remaining, 1);
    #[cfg(unix)]
    {
        let phase = phase_path(&paths);
        fs::remove_file(&phase).unwrap();
        std::os::unix::fs::symlink(&paths.plan, phase).unwrap();
        assert!(matches!(
            read(&paths, "staging", "fleet"),
            Err(EnsureStateError::Io { .. })
        ));
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires the explicit read-only external evidence path"]
fn inspect_supplied_completed_documents_without_external_mutation() {
    let root = std::env::var_os("CANIC_COMPLETED_SOURCE_WORKSPACE").unwrap();
    let environment = std::env::var("CANIC_COMPLETED_SOURCE_ENVIRONMENT").unwrap();
    let fleet = std::env::var("CANIC_COMPLETED_SOURCE_FLEET").unwrap();
    let paths = EnsurePaths::under(Path::new(&root), &environment, &fleet);
    let snapshot = read(&paths, &environment, &fleet).unwrap();
    assert!(
        snapshot
            .plan
            .pointer("/reviewed_desired/desired/bootstrap/recovery_controllers")
            .is_none()
    );
    assert!(!snapshot.bindings.phase_document_sha256.is_empty());
    assert_eq!(
        snapshot.bindings.plan_document_sha256,
        sha256_hex(&fs::read(&paths.plan).unwrap())
    );
    assert_eq!(
        snapshot.bindings.journal_document_sha256,
        sha256_hex(&fs::read(&paths.journal).unwrap())
    );
    assert_eq!(
        snapshot.bindings.state_document_sha256,
        sha256_hex(&fs::read(&paths.state).unwrap())
    );
}
