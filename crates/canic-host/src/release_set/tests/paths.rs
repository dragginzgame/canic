use super::*;
use crate::release_set::{ArtifactRootError, resolve_artifact_root, resolve_artifact_root_path};

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

#[test]
fn selected_environment_artifact_root_never_falls_back_to_local() {
    let temp = TempWorkspace::new();
    fs::create_dir_all(temp.path().join(".icp/local/canisters")).expect("create local artifacts");

    assert_eq!(
        resolve_artifact_root(temp.path(), "ic")
            .expect_err("selected environment must not use local artifacts"),
        ArtifactRootError::Missing {
            artifact_root: temp.path().join(".icp/ic/canisters"),
        }
    );
    fs::create_dir_all(temp.path().join(".icp/ic/canisters"))
        .expect("create selected-environment artifacts");
    assert_eq!(
        resolve_artifact_root(temp.path(), "ic").expect("selected root"),
        temp.path().join(".icp/ic/canisters")
    );
}

#[test]
fn exact_release_artifact_root_resolves_inside_the_project() {
    let temp = TempWorkspace::new();
    let artifact_root = temp
        .path()
        .join(".canic/release-builds/0123456789abcdef/artifacts");
    fs::create_dir_all(&artifact_root).expect("create release artifact root");

    assert_eq!(
        resolve_artifact_root_path(temp.path(), &artifact_root).expect("resolve exact root"),
        artifact_root
    );
}

#[cfg(unix)]
#[test]
fn exact_release_artifact_root_rejects_a_symlink_outside_the_project() {
    use std::os::unix::fs::symlink;

    let temp = TempWorkspace::new();
    let outside = TempWorkspace::new();
    let link = temp.path().join(".canic/release-builds/escape/artifacts");
    fs::create_dir_all(link.parent().expect("link parent")).expect("create link parent");
    symlink(outside.path(), &link).expect("link outside root");

    assert_eq!(
        resolve_artifact_root_path(temp.path(), &link)
            .expect_err("outside artifact root must reject"),
        ArtifactRootError::OutsideProject {
            artifact_root: link,
        }
    );
}
