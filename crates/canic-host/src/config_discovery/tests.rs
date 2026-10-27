use super::*;
use crate::test_support::temp_dir;

fn write_config(root: &Path, relative: &str, app: &str) -> PathBuf {
    let path = root.join(relative).join("canic.toml");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, format!("[app]\nname = \"{app}\"\n")).unwrap();
    path
}

#[test]
fn nearest_config_root_ignores_unrelated_ancestor_configs() {
    let root = temp_dir("canic-config-nearest");
    write_config(&root, "apps/duplicate-one", "duplicate");
    write_config(&root, "apps/duplicate-two", "duplicate");
    fs::write(root.join("icp.yaml"), "").unwrap();
    fs::write(root.join("Cargo.toml"), "[workspace\n").unwrap();
    let workspace = root.join("checkout");
    let config = write_config(&workspace, "apps/local", "local");
    assert_eq!(
        discover_canic_workspace_root_from(&config).unwrap(),
        Some(workspace.canonicalize().unwrap())
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cargo_workspace_boundary_prevents_ancestor_app_selection() {
    let root = temp_dir("canic-config-cargo-boundary");
    write_config(&root, "apps/unrelated", "unrelated");
    let workspace = root.join("checkout");
    fs::create_dir_all(workspace.join("src")).unwrap();
    fs::write(workspace.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
    assert_eq!(
        discover_canic_workspace_root_from(&workspace.join("src")).unwrap(),
        None
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn generated_directories_are_pruned_from_app_discovery() {
    let root = temp_dir("canic-config-generated");
    let alpha = write_config(&root, "apps/group/alpha", "alpha");
    let beta = write_config(&root, "apps/beta", "beta");
    for name in EXCLUDED_DIRECTORIES {
        write_config(&root, &format!("apps/group/alpha/{name}/nested"), "alpha");
    }
    let mut expected = vec![alpha, beta];
    expected.sort();
    assert_eq!(
        discover_workspace_canic_config_choices(&root).unwrap(),
        expected
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn duplicate_app_identity_in_selected_workspace_is_rejected() {
    let root = temp_dir("canic-config-duplicate");
    write_config(&root, "apps/one", "duplicate");
    write_config(&root, "apps/two", "duplicate");
    assert!(
        matches!(discover_canic_workspace_root_from(&root), Err(ConfigDiscoveryError::DuplicateApp { app, .. }) if app == "duplicate")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn traversal_depth_is_bounded() {
    let root = temp_dir("canic-config-depth");
    let mut deepest = root.join("apps");
    for _ in 0..=MAX_CONFIG_DEPTH {
        deepest.push("nested");
    }
    fs::create_dir_all(&deepest).unwrap();
    assert!(
        matches!(discover_workspace_canic_config_choices(&root), Err(ConfigDiscoveryError::DepthLimit { path, maximum }) if path == deepest && maximum == MAX_CONFIG_DEPTH)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn traversal_entry_budget_is_shared_across_directories() {
    let root = temp_dir("canic-config-entries");
    write_config(&root, "apps/one", "one");
    write_config(&root, "apps/two", "two");
    let mut visited = MAX_CONFIG_ENTRIES - 3;
    let mut choices = Vec::new();
    assert!(
        matches!(collect_canic_config_choices(&root.join("apps"), &mut choices, 0, &mut visited), Err(ConfigDiscoveryError::EntryLimit { maximum, .. }) if maximum == MAX_CONFIG_ENTRIES)
    );
    assert_eq!(visited, MAX_CONFIG_ENTRIES);
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn symlinked_app_root_is_not_traversed() {
    let root = temp_dir("canic-config-symlink");
    write_config(&root, "outside/app", "outside");
    let workspace = root.join("checkout");
    fs::create_dir_all(&workspace).unwrap();
    std::os::unix::fs::symlink(root.join("outside"), workspace.join("apps")).unwrap();
    assert!(
        discover_workspace_canic_config_choices(&workspace)
            .unwrap()
            .is_empty()
    );
    fs::remove_dir_all(root).unwrap();
}
