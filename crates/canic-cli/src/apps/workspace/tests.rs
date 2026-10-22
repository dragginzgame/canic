use super::*;
use crate::test_support::TempDir;

#[test]
fn deletion_updates_members_and_defaults_and_preserves_other_apps() {
    let root = TempDir::new("canic-app-membership-delete");
    let target = root.join("apps/demo");
    fs::create_dir_all(target.join("app")).unwrap();
    fs::create_dir_all(target.join("store")).unwrap();
    fs::create_dir_all(root.join("apps/other/app")).unwrap();
    let source = "# retained catalog\n[workspace]\nmembers = ['apps/demo/app', './apps/demo/store', 'apps/demo/*', 'apps/other/app', 'apps/*/app']\ndefault-members = ['apps/demo/app', 'apps/other/app']\n\n[workspace.package]\nversion = '0.1.0' # keep this comment\n";
    fs::write(root.join("Cargo.toml"), source).unwrap();
    let plan = WorkspaceRemoval::plan(&root, &target).unwrap();
    assert_eq!(
        plan.removed,
        ["./apps/demo/store", "apps/demo/*", "apps/demo/app"]
    );
    assert_eq!(fs::read_to_string(root.join("Cargo.toml")).unwrap(), source);
    plan.delete().unwrap();
    assert!(!target.exists());
    assert!(root.join("apps/other/app").is_dir());
    let updated = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(updated.starts_with("# retained catalog\n"));
    assert!(updated.ends_with("version = '0.1.0' # keep this comment\n"));
    let catalog: toml::Value = toml::from_str(&updated).unwrap();
    assert_eq!(
        catalog["workspace"]["members"].as_array().unwrap(),
        &[
            toml::Value::String("apps/other/app".into()),
            toml::Value::String("apps/*/app".into())
        ]
    );
    assert_eq!(
        catalog["workspace"]["default-members"].as_array().unwrap(),
        &[toml::Value::String("apps/other/app".into())]
    );
}

#[test]
fn deletion_refuses_changed_manifest_without_removing_app() {
    let root = TempDir::new("canic-app-membership-changed");
    let target = root.join("apps/demo");
    fs::create_dir_all(&target).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = ['apps/demo']\n",
    )
    .unwrap();
    let plan = WorkspaceRemoval::plan(&root, &target).unwrap();
    let changed = "[workspace]\nmembers = ['apps/demo']\n# concurrent edit\n";
    fs::write(root.join("Cargo.toml"), changed).unwrap();
    let AppCommandError::Io(error) = plan.delete().unwrap_err() else {
        panic!("expected I/O refusal")
    };
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(target.is_dir());
    assert_eq!(
        fs::read_to_string(root.join("Cargo.toml")).unwrap(),
        changed
    );
}

#[test]
fn deletion_refuses_malformed_members_and_handles_missing_manifest() {
    let root = TempDir::new("canic-app-membership-invalid");
    let target = root.join("apps/demo");
    fs::create_dir_all(&target).unwrap();
    fs::write(root.join("Cargo.toml"), "[workspace]\nmembers = 12\n").unwrap();
    assert_eq!(
        WorkspaceRemoval::plan(&root, &target).err().unwrap().kind(),
        io::ErrorKind::InvalidData
    );
    assert!(target.is_dir());
    fs::remove_file(root.join("Cargo.toml")).unwrap();
    WorkspaceRemoval::plan(&root, &target)
        .unwrap()
        .delete()
        .unwrap();
    assert!(!target.exists());
    assert!(!root.join("Cargo.toml").exists());
}

#[cfg(unix)]
#[test]
fn failed_membership_publication_preserves_the_app() {
    let root = TempDir::new("canic-app-membership-publication");
    let target = root.join("apps/demo");
    fs::create_dir_all(&target).unwrap();
    let original = "[workspace]\nmembers = ['apps/demo']\n";
    fs::write(root.join("Cargo.toml"), original).unwrap();
    let alias = root.join("alias");
    std::os::unix::fs::symlink(&*root, &alias).unwrap();
    let plan = WorkspaceRemoval::plan(&alias, &alias.join("apps/demo")).unwrap();
    std::assert_matches!(plan.delete(), Err(AppCommandError::Io(_)));
    assert!(target.is_dir());
    assert_eq!(
        fs::read_to_string(root.join("Cargo.toml")).unwrap(),
        original
    );
}
