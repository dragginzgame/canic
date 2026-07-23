use super::*;

#[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
use super::supported::{FileCommitStep, commit_with_hook};

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn durable_write_creates_parents_and_replaces_complete_contents() {
    let root = temp_root("replace");
    let path = root.join("reports/nested/state.json");

    write_bytes(&path, b"old").expect("write initial contents");
    write_bytes(&path, b"new complete contents").expect("replace contents");

    assert_eq!(
        fs::read(&path).expect("read target"),
        b"new complete contents"
    );
    assert_no_temporary_files(&root);

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn durable_create_new_never_replaces_an_existing_file() {
    let root = temp_root("create-new");
    fs::create_dir_all(&root).expect("create temp root");
    let path = root.join("report.json");

    create_new_bytes(&path, b"first complete contents").expect("create output");
    let error = create_new_bytes(&path, b"replacement").expect_err("existing output must reject");

    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(
        fs::read(&path).expect("read target"),
        b"first complete contents"
    );
    assert_no_temporary_files(&root);

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn durable_create_new_does_not_create_missing_parents() {
    let root = temp_root("create-new-parent");
    fs::create_dir_all(&root).expect("create temp root");
    let missing_parent = root.join("missing");
    let path = missing_parent.join("report.json");

    let error = create_new_bytes(&path, b"contents").expect_err("missing parent must reject");

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(!missing_parent.exists());
    assert_no_temporary_files(&root);

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn durable_create_new_with_parents_never_replaces_an_existing_file() {
    let root = temp_root("create-new-with-parents");
    fs::create_dir_all(&root).expect("create temp root");
    let path = root.join("nested/report.json");

    create_new_bytes_with_parents(&path, b"first complete contents").expect("create output");
    let error = create_new_bytes_with_parents(&path, b"replacement")
        .expect_err("existing output must reject");

    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(
        fs::read(&path).expect("read target"),
        b"first complete contents"
    );
    assert_no_temporary_files(&root);

    fs::remove_dir_all(root).expect("remove temp root");
}

#[cfg(unix)]
#[test]
fn durable_create_new_with_parents_rejects_a_symlinked_parent() {
    use std::os::unix::fs::symlink;

    let root = temp_root("create-new-symlink-parent");
    let outside = temp_root("create-new-symlink-outside");
    fs::create_dir_all(&root).expect("create temp root");
    fs::create_dir_all(&outside).expect("create outside root");
    symlink(&outside, root.join("linked")).expect("create parent symlink");

    let error = create_new_bytes_with_parents(&root.join("linked/report.json"), b"contents")
        .expect_err("symlinked parent must reject");

    assert_eq!(error.kind(), io::ErrorKind::NotADirectory);
    assert!(!outside.join("report.json").exists());
    fs::remove_dir_all(root).expect("remove temp root");
    fs::remove_dir_all(outside).expect("remove outside root");
}

#[test]
fn durable_write_rejects_a_target_without_a_file_name() {
    let error = write_bytes(Path::new("/"), b"value").expect_err("directory target must fail");

    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
}

#[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
#[test]
fn prepublication_failures_preserve_old_complete_bytes_and_remove_staging() {
    let steps = [
        FileCommitStep::TemporaryFileCreate,
        FileCommitStep::TemporaryFileWrite,
        FileCommitStep::TemporaryFileSync,
        FileCommitStep::Publication,
    ];

    for step in steps {
        let root = temp_root(&format!("prepublication-{step:?}"));
        fs::create_dir_all(&root).expect("create temp root");
        let path = root.join("report.json");
        fs::write(&path, b"old complete contents").expect("write old contents");
        let mut failed = false;

        let error = commit_with_hook(
            &path,
            b"new complete contents",
            FileCommitMode::Replace,
            |current, _| {
                if current == step && !failed {
                    failed = true;
                    return Err(io::Error::other("injected file commit failure"));
                }
                Ok(())
            },
        )
        .expect_err("injected step must fail");

        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(
            fs::read(&path).expect("read preserved target"),
            b"old complete contents"
        );
        assert_no_temporary_files(&root);
        fs::remove_dir_all(root).expect("remove temp root");
    }
}

#[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
#[test]
fn postpublication_sync_failure_exposes_only_new_complete_bytes() {
    let root = temp_root("postpublication");
    fs::create_dir_all(&root).expect("create temp root");
    let path = root.join("report.json");
    fs::write(&path, b"old complete contents").expect("write old contents");

    let error = commit_with_hook(
        &path,
        b"new complete contents",
        FileCommitMode::Replace,
        |step, _| {
            if step == FileCommitStep::FinalParentSync {
                return Err(io::Error::other("injected parent sync failure"));
            }
            Ok(())
        },
    )
    .expect_err("postpublication sync must fail");

    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert_eq!(
        fs::read(&path).expect("read published target"),
        b"new complete contents"
    );
    assert_no_temporary_files(&root);

    fs::remove_dir_all(root).expect("remove temp root");
}

#[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
#[test]
fn create_new_publication_race_cannot_replace_the_winner() {
    let root = temp_root("create-new-race");
    fs::create_dir_all(&root).expect("create temp root");
    let path = root.join("report.json");

    let error = commit_with_hook(
        &path,
        b"our complete contents",
        FileCommitMode::CreateNew,
        |step, _| {
            if step == FileCommitStep::Publication {
                fs::write(&path, b"raced complete contents")?;
            }
            Ok(())
        },
    )
    .expect_err("atomic create-new must reject a publication race");

    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(
        fs::read(&path).expect("read winning target"),
        b"raced complete contents"
    );
    assert_no_temporary_files(&root);

    fs::remove_dir_all(root).expect("remove temp root");
}

#[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
#[test]
fn new_parent_failures_never_create_the_final_file_or_staging() {
    let steps = [
        FileCommitStep::ParentDirectoryCreate,
        FileCommitStep::CreatedDirectorySync,
        FileCommitStep::CreatedDirectoryParentSync,
    ];

    for step in steps {
        let root = temp_root(&format!("parent-{step:?}"));
        fs::create_dir_all(&root).expect("create temp root");
        let path = root.join("reports/nested/report.json");
        let mut failed = false;

        let error = commit_with_hook(
            &path,
            b"complete contents",
            FileCommitMode::Replace,
            |current, _| {
                if current == step && !failed {
                    failed = true;
                    return Err(io::Error::other("injected parent persistence failure"));
                }
                Ok(())
            },
        )
        .expect_err("injected parent step must fail");

        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert!(!path.exists());
        assert_no_temporary_files(&root);
        fs::remove_dir_all(root).expect("remove temp root");
    }
}

#[test]
fn failed_replacement_removes_its_staging_file() {
    let root = temp_root("replace-error");
    let path = root.join("occupied");
    fs::create_dir_all(&path).expect("create non-file destination");
    fs::write(path.join("child"), b"preserved").expect("write destination child");

    write_bytes(&path, b"new contents").expect_err("non-empty directory must reject replacement");

    assert_eq!(
        fs::read(path.join("child")).expect("read destination child"),
        b"preserved"
    );
    assert_no_temporary_files(&root);

    fs::remove_dir_all(root).expect("remove temp root");
}

fn assert_no_temporary_files(root: &Path) {
    if !root.exists() {
        return;
    }
    for entry in fs::read_dir(root).expect("read test directory") {
        let entry = entry.expect("read test entry");
        let path = entry.path();
        if path.is_dir() {
            assert_no_temporary_files(&path);
        } else {
            assert!(
                !entry.file_name().to_string_lossy().contains(".canic-tmp-"),
                "temporary file remains at {}",
                path.display()
            );
        }
    }
}

fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "canic-host-durable-io-{label}-{}-{nanos}",
        std::process::id()
    ))
}
