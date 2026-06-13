use super::*;

#[test]
fn canisters_root_follows_config_parent_when_manifest_metadata_is_unavailable() {
    with_guarded_env(|| {
        let temp = TempWorkspace::new();
        let workspace_root = temp.path();
        let config_dir = workspace_root.join("custom");
        fs::create_dir_all(&config_dir).expect("create config dir");
        let config_file = config_dir.join("override.toml");
        fs::write(&config_file, "").expect("write config");

        let previous = std::env::var_os("CANIC_CONFIG_PATH");
        unsafe {
            std::env::set_var("CANIC_CONFIG_PATH", &config_file);
        }
        let result = canisters_root(workspace_root);
        unsafe {
            if let Some(value) = previous {
                std::env::set_var("CANIC_CONFIG_PATH", value);
            } else {
                std::env::remove_var("CANIC_CONFIG_PATH");
            }
        }

        assert_eq!(result, config_dir);
    });
}

#[test]
fn config_path_defaults_under_fleets_root() {
    with_guarded_env(|| {
        let temp = TempWorkspace::new();
        let workspace_root = temp.path();
        let fleets_dir = workspace_root.join("fleets");
        fs::create_dir_all(&fleets_dir).expect("create fleets dir");
        let expected = fleets_dir.join("canic.toml");

        let previous = std::env::var_os("CANIC_CONFIG_PATH");
        unsafe {
            std::env::remove_var("CANIC_CONFIG_PATH");
        }
        let result = config_path(workspace_root);
        unsafe {
            if let Some(value) = previous {
                std::env::set_var("CANIC_CONFIG_PATH", value);
            }
        }

        assert_eq!(result, expected);
    });
}

#[test]
fn root_manifest_path_prefers_canister_manifest_metadata() {
    with_guarded_env(|| {
        let temp = TempWorkspace::new();
        let workspace_root = temp.path();
        fs::create_dir_all(workspace_root.join("fleets/test/root")).expect("create root dir");
        fs::create_dir_all(workspace_root.join("fleets/test/root/src"))
            .expect("create root src dir");
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

        let previous_config = std::env::var_os("CANIC_CONFIG_PATH");
        let previous_root = std::env::var_os("CANIC_CANISTERS_ROOT");
        unsafe {
            std::env::remove_var("CANIC_CONFIG_PATH");
            std::env::remove_var("CANIC_CANISTERS_ROOT");
        }
        let result = root_manifest_path(workspace_root).expect("root manifest path");
        restore_env("CANIC_CONFIG_PATH", previous_config);
        restore_env("CANIC_CANISTERS_ROOT", previous_root);

        assert_eq!(result, workspace_root.join("fleets/test/root/Cargo.toml"));
    });
}

#[test]
fn canister_manifest_path_prefers_canister_manifest_metadata() {
    with_guarded_env(|| {
        let temp = TempWorkspace::new();
        let workspace_root = temp.path();
        fs::create_dir_all(workspace_root.join("fleets/test/user_hub"))
            .expect("create user hub dir");
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

        let previous_config = std::env::var_os("CANIC_CONFIG_PATH");
        let previous_root = std::env::var_os("CANIC_CANISTERS_ROOT");
        unsafe {
            std::env::remove_var("CANIC_CONFIG_PATH");
            std::env::remove_var("CANIC_CANISTERS_ROOT");
        }
        let result =
            canister_manifest_path(workspace_root, "user_hub").expect("user hub manifest path");
        restore_env("CANIC_CONFIG_PATH", previous_config);
        restore_env("CANIC_CANISTERS_ROOT", previous_root);

        assert_eq!(
            result,
            workspace_root.join("fleets/test/user_hub/Cargo.toml")
        );
    });
}

#[test]
fn canister_manifest_path_uses_declared_canic_role_metadata() {
    with_guarded_env(|| {
        let temp = TempWorkspace::new();
        let workspace_root = temp.path();
        fs::create_dir_all(workspace_root.join("fleets/test/scale")).expect("create scale dir");
        fs::create_dir_all(workspace_root.join("fleets/test/scale/src"))
            .expect("create scale src dir");
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
        fs::write(workspace_root.join("fleets/test/scale/src/lib.rs"), "")
            .expect("write scale lib");

        let previous_config = std::env::var_os("CANIC_CONFIG_PATH");
        let previous_root = std::env::var_os("CANIC_CANISTERS_ROOT");
        unsafe {
            std::env::remove_var("CANIC_CONFIG_PATH");
            std::env::remove_var("CANIC_CANISTERS_ROOT");
        }
        let result =
            canister_manifest_path(workspace_root, "scale_replica").expect("scale manifest path");
        restore_env("CANIC_CONFIG_PATH", previous_config);
        restore_env("CANIC_CANISTERS_ROOT", previous_root);

        assert_eq!(result, workspace_root.join("fleets/test/scale/Cargo.toml"));
    });
}

#[test]
fn canister_manifest_path_prefers_scoped_role_metadata() {
    with_guarded_env(|| {
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

        let previous_config = std::env::var_os("CANIC_CONFIG_PATH");
        let previous_root = std::env::var_os("CANIC_CANISTERS_ROOT");
        unsafe {
            std::env::remove_var("CANIC_CONFIG_PATH");
            std::env::set_var("CANIC_CANISTERS_ROOT", workspace_root.join("fleets/test"));
        }
        let result =
            canister_manifest_path(workspace_root, "root").expect("scoped root manifest path");
        restore_env("CANIC_CONFIG_PATH", previous_config);
        restore_env("CANIC_CANISTERS_ROOT", previous_root);

        assert_eq!(result, fleet_root.join("Cargo.toml"));
    });
}

#[test]
fn canister_manifest_path_requires_declared_role_metadata() {
    with_guarded_env(|| {
        let temp = TempWorkspace::new();
        let workspace_root = temp.path();
        fs::create_dir_all(workspace_root.join("fleets")).expect("create fleets dir");
        fs::write(
            workspace_root.join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .expect("write workspace manifest");

        let previous_config = std::env::var_os("CANIC_CONFIG_PATH");
        let previous_root = std::env::var_os("CANIC_CANISTERS_ROOT");
        unsafe {
            std::env::remove_var("CANIC_CONFIG_PATH");
            std::env::remove_var("CANIC_CANISTERS_ROOT");
        }
        let err = canister_manifest_path(workspace_root, "user_hub")
            .expect_err("missing role metadata must fail");
        restore_env("CANIC_CONFIG_PATH", previous_config);
        restore_env("CANIC_CANISTERS_ROOT", previous_root);

        assert!(
            err.to_string()
                .contains("[package.metadata.canic] role = \"user_hub\""),
            "unexpected error: {err}"
        );
    });
}

#[test]
fn canisters_root_defaults_to_workspace_fleets_dir() {
    with_guarded_env(|| {
        let temp = TempWorkspace::new();
        let workspace_root = temp.path();
        let previous_config = std::env::var_os("CANIC_CONFIG_PATH");
        let previous_root = std::env::var_os("CANIC_CANISTERS_ROOT");
        unsafe {
            std::env::remove_var("CANIC_CONFIG_PATH");
            std::env::remove_var("CANIC_CANISTERS_ROOT");
        }
        let result = canisters_root(workspace_root);
        restore_env("CANIC_CONFIG_PATH", previous_config);
        restore_env("CANIC_CANISTERS_ROOT", previous_root);

        assert_eq!(result, workspace_root.join("fleets"));
    });
}

#[test]
fn config_path_override_is_normalized_against_workspace_root() {
    with_guarded_env(|| {
        let temp = TempWorkspace::new();
        let workspace_root = temp.path();
        let relative = Path::new("configs/canic.toml");
        let previous = std::env::var_os("CANIC_CONFIG_PATH");
        unsafe {
            std::env::set_var("CANIC_CONFIG_PATH", relative);
        }
        let result = config_path(workspace_root);
        unsafe {
            if let Some(value) = previous {
                std::env::set_var("CANIC_CONFIG_PATH", value);
            } else {
                std::env::remove_var("CANIC_CONFIG_PATH");
            }
        }

        assert_eq!(result, workspace_root.join(relative));
    });
}
