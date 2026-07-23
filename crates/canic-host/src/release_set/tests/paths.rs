use super::*;

#[test]
fn config_path_defaults_under_apps_root() {
    let temp = TempWorkspace::new();
    let workspace_root = temp.path();
    let apps_dir = workspace_root.join("apps");
    fs::create_dir_all(&apps_dir).expect("create apps dir");

    assert_eq!(config_path(workspace_root), apps_dir.join("canic.toml"));
}

#[test]
fn app_sources_root_defaults_to_workspace_apps_dir() {
    let temp = TempWorkspace::new();
    let workspace_root = temp.path();

    assert_eq!(
        app_sources_root(workspace_root),
        workspace_root.join("apps")
    );
}
