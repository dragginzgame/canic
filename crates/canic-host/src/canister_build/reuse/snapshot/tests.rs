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

#[test]
fn discovered_absence_requires_a_complete_prebuild_directory_scan() {
    let root = temp_dir("reuse-discovered-absence");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/build.rs"), b"fn main() {}\n").unwrap();
    let before = snapshot(&root);
    let missing = root.join("app/src/build.rs");
    let mut after = snapshot(&root);
    add_optional(&missing, &mut after.files).unwrap();
    before.validate_after(&after).unwrap();
    assert_ne!(before.digest(), after.digest());

    // Once the absent input is recorded, its later creation invalidates reuse.
    fs::create_dir_all(missing.parent().unwrap()).unwrap();
    fs::write(&missing, b"new build input").unwrap();
    assert!(
        matches!(after.validate_after(&snapshot(&root)), Err(BuildReuseError::ChangedInput(path)) if path == missing)
    );
    fs::remove_dir_all(root.join("app")).unwrap();

    // Admission of one absent input must not hide a subsequent source addition.
    let added = root.join("z.rs");
    fs::write(&added, b"new source").unwrap();
    add_optional(&added, &mut after.files).unwrap();
    assert!(
        matches!(before.validate_after(&after), Err(BuildReuseError::ChangedInput(path)) if path == added)
    );
    fs::remove_file(added).unwrap();

    for name in ["target", ".git", ".canic", ".icp", ".tmp"] {
        let unscanned = root.join(name).join("input.rs");
        fs::create_dir_all(unscanned.parent().unwrap()).unwrap();
        fs::write(&unscanned, b"existed before compilation").unwrap();
        let before = snapshot(&root);
        fs::remove_file(&unscanned).unwrap();
        let mut after = snapshot(&root);
        add_optional(&unscanned, &mut after.files).unwrap();
        assert!(
            matches!(before.validate_after(&after), Err(BuildReuseError::UnobservedInput(path)) if path == unscanned)
        );
    }
    for unobserved in [
        root.join("missing/../unknown.rs"),
        root.with_extension("external.rs"),
    ] {
        let mut after = snapshot(&root);
        add_optional(&unobserved, &mut after.files).unwrap();
        assert!(
            matches!(before.validate_after(&after), Err(BuildReuseError::UnobservedInput(path)) if path == unobserved)
        );
    }
    fs::remove_dir_all(root).unwrap();
}
