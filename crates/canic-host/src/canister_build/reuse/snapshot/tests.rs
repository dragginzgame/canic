use super::*;
use crate::{canister_build::reuse::collect_files, test_support::temp_dir};

fn snapshot(root: &Path) -> BuildInputSnapshot {
    let mut files = BTreeMap::new();
    collect_files(root, root, &mut files, true).unwrap();
    BuildInputSnapshot {
        identity: "same authority".into(),
        files,
    }
}

#[test]
fn replaced_dependency_inventory_rechecks_dropped_source_bytes() {
    let root = temp_dir("reuse-replaced-inventory");
    fs::create_dir_all(&root).unwrap();
    let source = root.join("external.rs");
    fs::write(&source, b"first").unwrap();
    let before = snapshot(&root);
    let mut after = snapshot(&root);
    after.files.clear();
    before.validate_after(&after).unwrap();
    assert_ne!(before.digest(), after.digest());

    let added = root.join("new.rs");
    fs::write(&added, b"added").unwrap();
    assert!(
        matches!(before.validate_after(&after), Err(BuildReuseError::ChangedInput(path)) if path == added)
    );
    fs::remove_file(added).unwrap();
    fs::write(&source, b"other").unwrap();
    assert!(
        matches!(before.validate_after(&after), Err(BuildReuseError::ChangedInput(path)) if path == source)
    );
    fs::remove_file(&source).unwrap();
    assert!(
        matches!(before.validate_after(&after), Err(BuildReuseError::ChangedInput(path)) if path == source)
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join("missing-target"), &source).unwrap();
        assert!(
            matches!(before.validate_after(&after), Err(BuildReuseError::Unsupported(path)) if path == source)
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn real_source_edits_and_inventory_growth_are_distinct_failures() {
    let root = temp_dir("reuse-source-growth");
    fs::create_dir_all(&root).unwrap();
    let source = root.join("source.rs");
    fs::write(&source, b"first").unwrap();
    let before = snapshot(&root);
    fs::write(&source, b"other").unwrap();
    assert!(
        matches!(before.validate_after(&snapshot(&root)), Err(BuildReuseError::ChangedInput(path)) if path == source)
    );
    fs::write(&source, b"first").unwrap();
    let added = root.join("added.rs");
    fs::write(&added, b"added").unwrap();
    assert!(
        matches!(before.validate_after(&snapshot(&root)), Err(BuildReuseError::ChangedInput(path)) if path == added)
    );
    fs::remove_file(&added).unwrap();
    let mut after = snapshot(&root);
    after
        .files
        .insert("/previously-unobserved/input.rs".into(), "new bytes".into());
    assert!(matches!(
        before.validate_after(&after),
        Err(BuildReuseError::UnobservedInput(_))
    ));
    after = snapshot(&root);
    after.identity = "different authority".into();
    assert!(matches!(
        before.validate_after(&after),
        Err(BuildReuseError::Changed)
    ));
    fs::remove_dir_all(root).unwrap();
}
