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
fn root_manifest_path_prefers_canister_manifest_metadata() {
    let temp = TempWorkspace::new();
    let workspace_root = temp.path();
    fs::create_dir_all(workspace_root.join("fleets/test/root")).expect("create root dir");
    fs::create_dir_all(workspace_root.join("fleets/test/root/src")).expect("create root src dir");
    fs::write(
        workspace_root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"fleets/test/root\"]\n",
    )
    .expect("write workspace manifest");
    fs::write(
        workspace_root.join("fleets/test/root/Cargo.toml"),
        r#"[package]
name = "canister_root"
version = "0.1.0"
edition = "2024"

[package.metadata.canic]
role = "root"
"#,
    )
    .expect("write root manifest");
    fs::write(workspace_root.join("fleets/test/root/src/lib.rs"), "").expect("write root lib");

    let result = root_manifest_path(workspace_root).expect("root manifest path");

    assert_eq!(result, workspace_root.join("fleets/test/root/Cargo.toml"));
}

#[test]
fn canister_manifest_path_prefers_canister_manifest_metadata() {
    let temp = TempWorkspace::new();
    let workspace_root = temp.path();
    fs::create_dir_all(workspace_root.join("fleets/test/user_hub")).expect("create user hub dir");
    fs::create_dir_all(workspace_root.join("fleets/test/user_hub/src"))
        .expect("create user hub src dir");
    fs::write(
        workspace_root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"fleets/test/user_hub\"]\n",
    )
    .expect("write workspace manifest");
    fs::write(
        workspace_root.join("fleets/test/user_hub/Cargo.toml"),
        r#"[package]
name = "canister_user_hub"
version = "0.1.0"
edition = "2024"

[package.metadata.canic]
role = "user_hub"
"#,
    )
    .expect("write user hub manifest");
    fs::write(workspace_root.join("fleets/test/user_hub/src/lib.rs"), "")
        .expect("write user hub lib");

    let result =
        canister_manifest_path(workspace_root, "user_hub").expect("user hub manifest path");

    assert_eq!(
        result,
        workspace_root.join("fleets/test/user_hub/Cargo.toml")
    );
}

#[test]
fn canister_manifest_path_uses_declared_canic_role_metadata() {
    let temp = TempWorkspace::new();
    let workspace_root = temp.path();
    fs::create_dir_all(workspace_root.join("fleets/test/scale")).expect("create scale dir");
    fs::create_dir_all(workspace_root.join("fleets/test/scale/src")).expect("create scale src dir");
    fs::write(
        workspace_root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"fleets/test/scale\"]\n",
    )
    .expect("write workspace manifest");
    fs::write(
        workspace_root.join("fleets/test/scale/Cargo.toml"),
        r#"[package]
name = "canister_scale"
version = "0.1.0"
edition = "2024"

[package.metadata.canic]
role = "scale_replica"
"#,
    )
    .expect("write scale manifest");
    fs::write(workspace_root.join("fleets/test/scale/src/lib.rs"), "").expect("write scale lib");

    let result =
        canister_manifest_path(workspace_root, "scale_replica").expect("scale manifest path");

    assert_eq!(result, workspace_root.join("fleets/test/scale/Cargo.toml"));
}

#[test]
fn canister_manifest_path_prefers_scoped_role_metadata() {
    let temp = TempWorkspace::new();
    let workspace_root = temp.path();
    let audit_root = workspace_root.join("canisters/audit/root_probe");
    let fleet_root = workspace_root.join("fleets/test/root");

    fs::create_dir_all(audit_root.join("src")).expect("create audit root dir");
    fs::create_dir_all(fleet_root.join("src")).expect("create fleet root dir");
    fs::write(
        workspace_root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"canisters/audit/root_probe\", \"fleets/test/root\"]\n",
    )
    .expect("write workspace manifest");
    fs::write(
        audit_root.join("Cargo.toml"),
        r#"[package]
name = "root_probe"
version = "0.1.0"
edition = "2024"

[package.metadata.canic]
role = "root"
"#,
    )
    .expect("write audit root manifest");
    fs::write(audit_root.join("src/lib.rs"), "").expect("write audit root lib");
    fs::write(
        fleet_root.join("Cargo.toml"),
        r#"[package]
name = "canister_root"
version = "0.1.0"
edition = "2024"

[package.metadata.canic]
role = "root"
"#,
    )
    .expect("write fleet root manifest");
    fs::write(fleet_root.join("src/lib.rs"), "").expect("write fleet root lib");

    let result = canister_manifest_path(workspace_root, "root").expect("root manifest path");

    assert_eq!(result, fleet_root.join("Cargo.toml"));
}

#[test]
fn canister_manifest_path_requires_declared_role_metadata() {
    let temp = TempWorkspace::new();
    let workspace_root = temp.path();
    fs::create_dir_all(workspace_root.join("fleets")).expect("create fleets dir");
    fs::write(
        workspace_root.join("Cargo.toml"),
        "[workspace]\nmembers = []\n",
    )
    .expect("write workspace manifest");

    let error = canister_manifest_path(workspace_root, "user_hub")
        .expect_err("missing role metadata must fail");

    std::assert_matches!(
        error,
        CanisterManifestError::RoleNotFound { role, canister_root }
            if role == "user_hub"
                && canister_root == workspace_root.join("fleets").canonicalize().expect("root")
    );
}

#[test]
fn canister_manifest_path_preserves_ambiguous_role_manifests() {
    let temp = TempWorkspace::new();
    let workspace_root = temp.path();
    for package in ["root_a", "root_b"] {
        let package_root = workspace_root.join("fleets/test").join(package);
        fs::create_dir_all(package_root.join("src")).expect("create package source");
        fs::write(
            package_root.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[package.metadata.canic]\nrole = \"root\"\n"
            ),
        )
        .expect("write package manifest");
        fs::write(package_root.join("src/lib.rs"), "").expect("write package source");
    }
    fs::write(
        workspace_root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"fleets/test/root_a\", \"fleets/test/root_b\"]\n",
    )
    .expect("write workspace manifest");

    let error = root_manifest_path(workspace_root).expect_err("duplicate roles must fail");
    let CanisterManifestError::RoleAmbiguous {
        role, manifests, ..
    } = error
    else {
        panic!("expected typed ambiguous role error");
    };

    assert_eq!(role, "root");
    assert_eq!(
        manifests,
        ["root_a", "root_b"]
            .into_iter()
            .map(|package| {
                workspace_root
                    .join("fleets/test")
                    .join(package)
                    .join("Cargo.toml")
            })
            .collect::<Vec<_>>()
    );
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
