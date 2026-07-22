use super::*;

#[test]
fn config_path_defaults_under_fleets_root() {
    let temp = TempWorkspace::new();
    let workspace_root = temp.path();
    let fleets_dir = workspace_root.join("fleets");
    fs::create_dir_all(&fleets_dir).expect("create fleets dir");

    assert_eq!(config_path(workspace_root), fleets_dir.join("canic.toml"));
}

#[test]
fn canisters_root_defaults_to_workspace_fleets_dir() {
    let temp = TempWorkspace::new();
    let workspace_root = temp.path();

    assert_eq!(
        canisters_root(workspace_root),
        workspace_root.join("fleets")
    );
}
