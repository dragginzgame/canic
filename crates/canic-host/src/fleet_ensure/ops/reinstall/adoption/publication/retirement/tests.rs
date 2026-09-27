//! Local retirement crash recovery and exact-byte preservation; no IC effects are simulated.

use super::*;

#[test]
fn interrupted_retirement_preserves_history_and_current_operation() {
    for removed in 0..=FILES.len() {
        let workspace = crate::test_support::temp_dir("completed-authority-retirement");
        let paths = EnsurePaths::under(&workspace, "local", "test-fleet");
        let mut files = BTreeMap::new();
        for name in FILES {
            let bytes = format!("opaque exact evidence: {name}\n").into_bytes();
            let digest = sha256_hex(&bytes);
            storage::retain(&paths, &digest, &bytes).unwrap();
            crate::durable_io::write_bytes(&paths.plan.with_file_name(name), &bytes).unwrap();
            files.insert(name.into(), digest);
        }
        let record = CompletedAuthorityRetirementRecord {
            schema_version: 1,
            review_sha256: "ab".repeat(32),
            files,
        };
        write_current(&intent_path(&paths), &record).unwrap();
        for name in FILES.iter().take(removed) {
            remove(&paths.plan.with_file_name(name)).unwrap();
        }
        let current = b"current operation must remain byte-for-byte unchanged";
        crate::durable_io::write_bytes(&paths.plan, current).unwrap();
        crate::durable_io::write_bytes(&paths.journal, current).unwrap();
        recover(&paths).unwrap();
        assert!(!pending(&paths));
        for (name, digest) in &record.files {
            assert!(!paths.plan.with_file_name(name).exists());
            assert_eq!(
                storage::exact_bytes(&storage::object_path(&paths, digest), digest).unwrap(),
                format!("opaque exact evidence: {name}\n").as_bytes()
            );
        }
        assert_eq!(fs::read(&paths.plan).unwrap(), current);
        assert_eq!(fs::read(&paths.journal).unwrap(), current);
        // Replaying retirement cannot roll back or rewrite any later operation.
        recover(&paths).unwrap();
        assert_eq!(fs::read(&paths.journal).unwrap(), current);
        fs::remove_dir_all(workspace).unwrap();
    }
}

#[test]
fn changed_pending_authority_prevents_any_removal() {
    let workspace = crate::test_support::temp_dir("completed-authority-retirement-conflict");
    let paths = EnsurePaths::under(&workspace, "local", "test-fleet");
    let digest = sha256_hex(b"archived");
    storage::retain(&paths, &digest, b"archived").unwrap();
    let record = CompletedAuthorityRetirementRecord {
        schema_version: 1,
        review_sha256: "ab".repeat(32),
        files: BTreeMap::from([(FILES[0].into(), digest)]),
    };
    let path = paths.plan.with_file_name(FILES[0]);
    crate::durable_io::write_bytes(&path, b"different current authority").unwrap();
    write_current(&intent_path(&paths), &record).unwrap();
    assert!(matches!(
        recover(&paths),
        Err(EnsureStateError::CompletedHandoffConflict)
    ));
    assert!(pending(&paths));
    assert_eq!(fs::read(path).unwrap(), b"different current authority");
    fs::remove_dir_all(workspace).unwrap();
}
