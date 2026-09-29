use super::*;

#[test]
fn rejected_stage_preserves_destination_and_removes_temporary_copy() {
    let temp = TempDirectory::create("publication-test").unwrap();
    let candidate = temp.path.join("candidate");
    let destination = temp.path.join("installed");
    fs::write(&candidate, b"new executable").unwrap();
    fs::write(&destination, b"previous executable").unwrap();
    let result = publish_executable("test", &candidate, &destination, |_stage| {
        Err(InstallError::ExecutableHashMismatch {
            path: destination.clone(),
            actual: "actual".into(),
            expected: "required".into(),
        })
    });
    std::assert_matches!(result, Err(InstallError::ExecutableHashMismatch { .. }));
    assert_eq!(fs::read(&destination).unwrap(), b"previous executable");
    assert_eq!(fs::read_dir(&temp.path).unwrap().count(), 2);
}

#[test]
fn archive_hash_mismatch_is_rejected_before_extraction() {
    let temp = TempDirectory::create("hash-test").unwrap();
    let archive = temp.path.join("archive");
    fs::write(&archive, b"wrong archive").unwrap();
    std::assert_matches!(
        verify_archive(&archive, "required"),
        Err(InstallError::ArchiveHashMismatch { .. })
    );
}

#[test]
fn publication_failure_cleans_stage_and_preserves_destination_directory() {
    let temp = TempDirectory::create("rename-test").unwrap();
    let candidate = temp.path.join("candidate");
    let destination = temp.path.join("installed");
    fs::write(&candidate, b"candidate").unwrap();
    fs::create_dir(&destination).unwrap();
    let result = publish_executable("test", &candidate, &destination, |_| {
        Ok::<_, InstallError>(())
    });
    std::assert_matches!(result, Err(InstallError::Io { .. }));
    assert!(destination.is_dir());
    assert_eq!(fs::read_dir(&temp.path).unwrap().count(), 2);
}
