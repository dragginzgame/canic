use super::*;
use crate::test_support::temp_path;
use std::{fs, io::Read};

const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[test]
fn artifact_path_segment_preserves_safe_characters_and_replaces_others() {
    assert_eq!(
        artifact_path_segment("abc-XYZ_123:canister"),
        "abc-XYZ_123_canister"
    );
}

// Ensure empty-byte checksums match the standard SHA-256 vector.
#[test]
fn byte_checksum_matches_sha256_vector() {
    let checksum = ArtifactChecksum::from_bytes(&[]);

    assert_eq!(checksum.algorithm, "sha256");
    assert_eq!(checksum.hash, EMPTY_SHA256);
}

#[test]
fn checked_directory_projection_preserves_identity_and_typed_refusals() {
    use ic_backup::ops::artifacts::DirectoryChecksumError;
    let checksum = ArtifactChecksum::from_relative_file_checksums(vec![
        ("nested/b.txt".into(), ArtifactChecksum::from_bytes(b"b")),
        ("a.txt".into(), ArtifactChecksum::from_bytes(b"a")),
    ])
    .unwrap();
    assert_eq!(
        checksum.hash,
        "e4d330f138b8f1b3044e84b5dcbe4fd1cb7e043d0c20810c083d791b6de01266"
    );
    assert_eq!(
        ArtifactChecksum::from_relative_file_checksums(vec![])
            .unwrap()
            .hash,
        EMPTY_SHA256
    );
    for paths in [["a", "a"], ["../a", "b"]] {
        let error = ArtifactChecksum::from_relative_file_checksums(
            paths
                .into_iter()
                .map(|path| (path.into(), ArtifactChecksum::from_bytes(b"same")))
                .collect(),
        )
        .unwrap_err();
        let ArtifactChecksumError::Io(source) = error else {
            panic!("expected typed path cause")
        };
        let cause = source
            .get_ref()
            .unwrap()
            .downcast_ref::<DirectoryChecksumError>()
            .unwrap();
        if paths[0] == "a" {
            std::assert_matches!(cause, DirectoryChecksumError::DuplicatePath { .. });
        } else {
            std::assert_matches!(cause, DirectoryChecksumError::InvalidRelativePath { .. });
        }
    }
    let mut invalid = ArtifactChecksum::from_bytes(b"a");
    invalid.algorithm = "unknown".into();
    std::assert_matches!(
        ArtifactChecksum::from_relative_file_checksums(vec![("a".into(), invalid)]),
        Err(ArtifactChecksumError::UnsupportedAlgorithm(_))
    );
}

#[test]
fn checked_directory_projection_preserves_unicode_framing_and_hex_equivalence() {
    let mut accent = ArtifactChecksum::from_bytes(b"accent");
    accent.hash.make_ascii_uppercase();
    let checksum = ArtifactChecksum::from_relative_file_checksums(vec![
        ("é.txt".into(), accent),
        (
            "nested/雪.txt".into(),
            ArtifactChecksum::from_bytes(b"snow"),
        ),
    ])
    .unwrap();
    assert_eq!(
        checksum.hash,
        "a7bd4da90f6e41313fd28db721c61188b9cb7cdfe11c30d455335e2a19e0f776"
    );
}

#[cfg(unix)]
#[test]
fn checked_directory_projection_preserves_non_utf8_identity_refusal() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let path = PathBuf::from(OsString::from_vec(vec![0x80]));
    let result = ArtifactChecksum::from_relative_file_checksums(vec![(
        path.clone(),
        ArtifactChecksum::from_bytes(b"source"),
    )]);
    std::assert_matches!(
        result,
        Err(ArtifactChecksumError::Artifact(ArtifactError::NonUtf8Path { path: rejected })) if rejected == path
    );
}

// Ensure file checksums use the same implementation as byte checksums.
#[test]
fn file_checksum_matches_byte_checksum() {
    let path = temp_path("canic-backup-checksum");
    fs::write(&path, b"canic backup artifact").expect("write temp artifact");

    let from_file = ArtifactChecksum::from_file(&path).expect("checksum file");
    let from_bytes = ArtifactChecksum::from_bytes(b"canic backup artifact");

    fs::remove_file(&path).expect("remove temp artifact");
    assert_eq!(from_file, from_bytes);
}

#[test]
fn stream_checksum_projection_preserves_source_io_errors() {
    let path = temp_path("canic-backup-checksum-io");
    fs::write(&path, b"source").unwrap();
    let mut unreadable = fs::OpenOptions::new().write(true).open(&path).unwrap();
    let expected_read = unreadable.read(&mut [0; 1]).unwrap_err().raw_os_error();
    let ArtifactChecksumError::Io(read_error) =
        ArtifactChecksum::from_reader(&mut unreadable).unwrap_err()
    else {
        panic!("expected source I/O error");
    };
    assert_eq!(read_error.raw_os_error(), expected_read);

    assert_eq!(fs::read(&path).unwrap(), b"source");
    fs::remove_file(path).unwrap();
}

// Ensure directory checksums are stable regardless of file creation order.
#[test]
fn directory_checksum_is_order_independent() {
    let first = temp_path("canic-backup-dir-a");
    let second = temp_path("canic-backup-dir-b");
    fs::create_dir_all(first.join("nested")).expect("create first");
    fs::create_dir_all(second.join("nested")).expect("create second");

    fs::write(first.join("a.txt"), b"a").expect("write first a");
    fs::write(first.join("nested/b.txt"), b"b").expect("write first b");
    fs::write(second.join("nested/b.txt"), b"b").expect("write second b");
    fs::write(second.join("a.txt"), b"a").expect("write second a");

    let first_checksum = ArtifactChecksum::from_directory(&first).expect("checksum first");
    let second_checksum = ArtifactChecksum::from_directory(&second).expect("checksum second");

    fs::remove_dir_all(first).expect("remove first");
    fs::remove_dir_all(second).expect("remove second");
    assert_eq!(first_checksum, second_checksum);
    assert_eq!(
        first_checksum.hash,
        "e4d330f138b8f1b3044e84b5dcbe4fd1cb7e043d0c20810c083d791b6de01266"
    );
}

// Ensure checksum verification reports mismatches.
#[test]
fn checksum_verify_rejects_mismatch() {
    let checksum = ArtifactChecksum::from_bytes(b"actual");

    let err = checksum
        .verify(EMPTY_SHA256)
        .expect_err("different hash should fail");

    std::assert_matches!(err, ArtifactChecksumError::ChecksumMismatch { .. });
}

#[test]
fn checksum_verification_accepts_equivalent_hex_case() {
    let lower = ArtifactChecksum::from_bytes(&[]);
    let upper = ArtifactChecksum {
        algorithm: lower.algorithm.clone(),
        hash: lower.hash.to_ascii_uppercase(),
    };
    lower.verify(&upper.hash).expect("uppercase expected hash");
    upper.verify(&lower.hash).expect("uppercase observed hash");
    std::assert_matches!(
        lower.verify(
            &ArtifactChecksum::from_bytes(b"different")
                .hash
                .to_ascii_uppercase()
        ),
        Err(ArtifactChecksumError::ChecksumMismatch { .. })
    );
    std::assert_matches!(
        lower.verify("invalid"),
        Err(ArtifactChecksumError::InvalidHash(_))
    );
}

#[cfg(unix)]
#[test]
fn filesystem_checksums_reject_symlinked_files_and_entries() {
    let root = temp_path("canic-backup-checksum-symlink");
    fs::create_dir_all(root.join("tree")).expect("create checksum tree");
    fs::write(root.join("source"), b"source").expect("write source");
    std::os::unix::fs::symlink(root.join("source"), root.join("linked-file"))
        .expect("create file symlink");
    std::os::unix::fs::symlink(root.join("source"), root.join("tree/linked-entry"))
        .expect("create tree symlink");

    let file_error = ArtifactChecksum::from_file(&root.join("linked-file"))
        .expect_err("symlinked file must reject");
    let tree_error = ArtifactChecksum::from_directory(&root.join("tree"))
        .expect_err("symlinked directory entry must reject");

    std::assert_matches!(file_error, ArtifactChecksumError::Io(_));
    std::assert_matches!(
        tree_error,
        ArtifactChecksumError::Artifact(ArtifactError::UnsupportedEntry { .. })
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(unix)]
#[test]
fn private_staging_preserves_bytes_digest_modes_and_existing_output() {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_path("canic-backup-shared-stage");
    fs::create_dir_all(root.join("source/nested")).unwrap();
    fs::write(root.join("source/a.txt"), b"a").unwrap();
    fs::write(root.join("source/nested/b.txt"), b"b").unwrap();
    let destination = root.join("staged tree");
    let checksum =
        ArtifactChecksum::stage_relative_path_no_follow(&root, Path::new("source"), &destination)
            .unwrap();
    assert_eq!(
        checksum.hash,
        "e4d330f138b8f1b3044e84b5dcbe4fd1cb7e043d0c20810c083d791b6de01266"
    );
    assert_eq!(
        ArtifactChecksum::from_directory(&destination).unwrap(),
        checksum
    );
    assert_eq!(fs::read(destination.join("nested/b.txt")).unwrap(), b"b");
    for directory in [&destination, &destination.join("nested")] {
        assert_eq!(
            fs::metadata(directory).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
    assert_eq!(
        fs::metadata(destination.join("a.txt"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );

    let file_destination = root.join("staged file");
    let checksum = ArtifactChecksum::stage_relative_path_no_follow(
        &root,
        Path::new("source/a.txt"),
        &file_destination,
    )
    .unwrap();
    assert_eq!(checksum, ArtifactChecksum::from_bytes(b"a"));
    fs::write(root.join("source/a.txt"), b"changed").unwrap();
    let ArtifactChecksumError::Io(error) = ArtifactChecksum::stage_relative_path_no_follow(
        &root,
        Path::new("source/a.txt"),
        &file_destination,
    )
    .unwrap_err() else {
        panic!("expected create-new destination refusal");
    };
    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(fs::read(file_destination).unwrap(), b"a");
    assert_eq!(fs::read(root.join("source/a.txt")).unwrap(), b"changed");
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn relative_artifacts_refuse_traversal_and_symlinked_parents_before_staging() {
    let root = temp_path("canic-backup-relative-artifacts");
    fs::create_dir_all(root.join("source")).unwrap();
    fs::create_dir_all(root.join("outside")).unwrap();
    fs::write(root.join("outside/sentinel"), b"must survive").unwrap();
    std::os::unix::fs::symlink(root.join("outside"), root.join("source/link")).unwrap();
    let source = root.join("source");
    let destination = root.join("destination");
    for relative in [Path::new("../outside/sentinel"), Path::new("link/sentinel")] {
        assert!(ArtifactChecksum::from_relative_path_no_follow(&source, relative).is_err());
        assert!(
            ArtifactChecksum::stage_relative_path_no_follow(&source, relative, &destination)
                .is_err()
        );
        assert!(!destination.exists());
        assert_eq!(
            fs::read(root.join("outside/sentinel")).unwrap(),
            b"must survive"
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn non_utf8_tree_names_retain_shared_typed_refusal() {
    use std::os::unix::ffi::OsStringExt;

    let root = temp_path("canic-backup-utf8-artifacts");
    fs::create_dir_all(root.join("source")).unwrap();
    let invalid = root
        .join("source")
        .join(std::ffi::OsString::from_vec(vec![0xff]));
    fs::write(&invalid, b"source").unwrap();
    for result in [
        ArtifactChecksum::from_directory(&root.join("source")),
        ArtifactChecksum::stage_relative_path_no_follow(
            &root,
            Path::new("source"),
            &root.join("staged"),
        ),
    ] {
        std::assert_matches!(
            result,
            Err(ArtifactChecksumError::Artifact(
                ArtifactError::NonUtf8Path { .. }
            ))
        );
    }
    assert_eq!(fs::read(invalid).unwrap(), b"source");
    fs::remove_dir_all(root).unwrap();
}
