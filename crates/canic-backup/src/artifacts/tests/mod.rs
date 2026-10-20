use super::*;
use crate::test_support::temp_path;
use ic_backup::model::artifacts::ArtifactChecksumRecord;
use std::{fs, io::Read};

const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

// Ensure empty-byte checksums match the standard SHA-256 vector.
#[test]
fn byte_checksum_matches_sha256_vector() {
    let checksum = ArtifactChecksumRecord::from_bytes(&[]);

    assert_eq!(checksum.algorithm().to_owned(), "sha256");
    assert_eq!(checksum.hash().to_owned(), EMPTY_SHA256);
}

// Ensure file checksums use the same implementation as byte checksums.
#[test]
fn file_checksum_matches_byte_checksum() {
    let path = temp_path("canic-backup-checksum");
    fs::write(&path, b"canic backup artifact").expect("write temp artifact");

    let from_file = crate::artifacts::checksum_file(&path).expect("checksum file");
    let from_bytes = ArtifactChecksumRecord::from_bytes(b"canic backup artifact");

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
        crate::artifacts::checksum_reader(&mut unreadable).unwrap_err()
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

    let first_checksum = crate::artifacts::checksum_directory(&first).expect("checksum first");
    let second_checksum = crate::artifacts::checksum_directory(&second).expect("checksum second");

    fs::remove_dir_all(first).expect("remove first");
    fs::remove_dir_all(second).expect("remove second");
    assert_eq!(first_checksum, second_checksum);
    assert_eq!(
        first_checksum.hash().to_owned(),
        "e4d330f138b8f1b3044e84b5dcbe4fd1cb7e043d0c20810c083d791b6de01266"
    );
}

#[test]
fn checksum_record_decode_validates_and_normalizes_manifest_fields() {
    let valid = serde_json::json!({"algorithm":"sha256", "hash":EMPTY_SHA256.to_ascii_uppercase()});
    let checksum: ArtifactChecksumRecord = serde_json::from_value(valid).unwrap();
    assert_eq!(checksum.hash(), EMPTY_SHA256);
    for value in [
        serde_json::json!({"algorithm":"sha256", "hash":"invalid"}),
        serde_json::json!({"algorithm":"unsupported", "hash":EMPTY_SHA256}),
    ] {
        assert!(serde_json::from_value::<ArtifactChecksumRecord>(value).is_err());
    }
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

    let file_error = crate::artifacts::checksum_file(&root.join("linked-file"))
        .expect_err("symlinked file must reject");
    let tree_error = crate::artifacts::checksum_directory(&root.join("tree"))
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
        crate::artifacts::stage_relative_path(&root, Path::new("source"), &destination).unwrap();
    assert_eq!(
        checksum.hash().to_owned(),
        "e4d330f138b8f1b3044e84b5dcbe4fd1cb7e043d0c20810c083d791b6de01266"
    );
    assert_eq!(
        crate::artifacts::checksum_directory(&destination).unwrap(),
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
    let checksum =
        crate::artifacts::stage_relative_path(&root, Path::new("source/a.txt"), &file_destination)
            .unwrap();
    assert_eq!(checksum, ArtifactChecksumRecord::from_bytes(b"a"));
    fs::write(root.join("source/a.txt"), b"changed").unwrap();
    let ArtifactChecksumError::Io(error) =
        crate::artifacts::stage_relative_path(&root, Path::new("source/a.txt"), &file_destination)
            .unwrap_err()
    else {
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
        assert!(crate::artifacts::checksum_relative_path(&source, relative).is_err());
        assert!(crate::artifacts::stage_relative_path(&source, relative, &destination).is_err());
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
        crate::artifacts::checksum_directory(&root.join("source")),
        crate::artifacts::stage_relative_path(&root, Path::new("source"), &root.join("staged")),
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

#[test]
fn directory_declaration_errors_preserve_backup_categories() {
    use ic_host_artifacts::artifact::tree::DirectoryChecksumError;
    let path = std::path::PathBuf::from("entry");
    assert!(matches!(
        artifact_error(directory_artifact_error(DirectoryChecksumError::NonUtf8Path { path: path.clone() })),
        ArtifactChecksumError::Artifact(ArtifactError::NonUtf8Path { path: observed }) if observed == path
    ));
    for error in [
        DirectoryChecksumError::InvalidRelativePath { path: path.clone() },
        DirectoryChecksumError::DuplicatePath { path },
    ] {
        let ArtifactChecksumError::Io(error) = artifact_error(directory_artifact_error(error))
        else {
            panic!("expected invalid-data IO error");
        };
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }
}
