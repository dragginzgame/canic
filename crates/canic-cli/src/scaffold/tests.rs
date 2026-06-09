use super::*;
use crate::test_support::temp_dir;

// Ensure scaffold options parse the project name.
#[test]
fn parses_scaffold_options() {
    let options =
        ScaffoldOptions::parse([OsString::from("my_app")]).expect("parse scaffold options");

    assert_eq!(options.name, "my_app");
    assert_eq!(options.project_root, None);
    assert!(!options.yes);
}

// Ensure scaffold accepts explicit noninteractive confirmation.
#[test]
fn parses_scaffold_yes_option() {
    let options = ScaffoldOptions::parse([OsString::from("my_app"), OsString::from("--yes")])
        .expect("parse scaffold yes option");

    assert!(options.yes);
}

// Ensure canister scaffold options parse fleet and role identity.
#[test]
fn parses_canister_scaffold_options() {
    let options = CanisterScaffoldOptions::parse([OsString::from("demo"), OsString::from("store")])
        .expect("parse canister scaffold options");

    assert_eq!(options.fleet, "demo");
    assert_eq!(options.role, "store");
    assert_eq!(options.project_root, None);
}

// Ensure confirmation accepts an explicit yes response.
#[test]
fn confirm_scaffold_accepts_yes() {
    let root = temp_dir("canic-cli-scaffold-confirm-yes");
    let options = ScaffoldOptions {
        name: "my_app".to_string(),
        project_root: Some(root),
        yes: false,
    };
    let mut output = Vec::new();

    confirm_scaffold(&options, io::Cursor::new(b"y\n"), &mut output).expect("confirm scaffold");

    let output = String::from_utf8(output).expect("utf8 prompt");
    assert!(output.contains("target:"));
    assert!(output.contains("fleets/my_app"));
    assert!(output.contains("install: canic install my_app"));
}

// Ensure confirmation defaults to no on empty input.
#[test]
fn confirm_scaffold_rejects_empty_response() {
    let root = temp_dir("canic-cli-scaffold-confirm-no");
    let options = ScaffoldOptions {
        name: "my_app".to_string(),
        project_root: Some(root),
        yes: false,
    };
    let mut output = Vec::new();

    let err = confirm_scaffold(&options, io::Cursor::new(b"\n"), &mut output)
        .expect_err("empty response should cancel");

    std::assert_matches!(err, ScaffoldCommandError::Cancelled);
}

// Ensure invalid scaffold names are rejected before filesystem writes.
#[test]
fn rejects_invalid_project_names() {
    for name in ["MyApp", "my-app", "_app", "app_", "app__one", "1app"] {
        std::assert_matches!(
            ScaffoldOptions::parse([OsString::from(name)]),
            Err(ScaffoldCommandError::Usage(_))
        );
    }
}

// Ensure invalid canister scaffold role names are rejected before filesystem writes.
#[test]
fn rejects_invalid_canister_scaffold_role_names() {
    for name in [
        "Store",
        "store-api",
        "_store",
        "store_",
        "store__one",
        "1store",
    ] {
        std::assert_matches!(
            CanisterScaffoldOptions::parse([OsString::from("demo"), OsString::from(name)]),
            Err(ScaffoldCommandError::Usage(_))
        );
    }
}

// Ensure scaffold writes the expected minimal root and app files.
#[test]
fn scaffold_project_writes_root_and_app_files() {
    let root = temp_dir("canic-cli-scaffold");
    let options = ScaffoldOptions {
        name: "my_app".to_string(),
        project_root: Some(root.clone()),
        yes: true,
    };

    let result = scaffold_project(&options).expect("scaffold project");
    let config = fs::read_to_string(&result.config_path).expect("read config");
    let root_lib = fs::read_to_string(result.root_dir.join("src/lib.rs")).expect("read root lib");
    let root_manifest =
        fs::read_to_string(result.root_dir.join("Cargo.toml")).expect("read root manifest");
    let app_lib = fs::read_to_string(result.app_dir.join("src/lib.rs")).expect("read app lib");
    let app_manifest =
        fs::read_to_string(result.app_dir.join("Cargo.toml")).expect("read app manifest");

    fs::remove_dir_all(root).expect("remove scaffold temp root");
    assert!(config.contains("controllers = []"));
    assert!(config.contains("app_index = []"));
    assert!(config.contains("[fleet]"));
    assert!(config.contains("name = \"my_app\""));
    assert!(config.contains("[auth.delegated_tokens]"));
    assert!(config.contains("enabled = false"));
    assert!(config.contains("[roles.root]"));
    assert!(config.contains("[roles.app]"));
    assert!(!config.contains("auto_create"));
    assert!(!config.contains("subnet_index"));
    assert!(config.contains("[subnets.prime.canisters.root]"));
    assert!(config.contains("[subnets.prime.canisters.app]"));
    assert!(!config.contains("app_directory"));
    assert!(!config.contains("topup_policy"));
    assert!(!config.contains("[[canisters]]"));
    assert!(root_manifest.contains("version = \"0.1.0\""));
    assert!(root_manifest.contains("fleet = \"my_app\""));
    assert!(root_manifest.contains("role = \"root\""));
    assert!(root_manifest.contains("canic = \""));
    assert!(root_manifest.contains("ic-cdk = \"0.20\""));
    assert!(!root_manifest.contains("workspace = true"));
    assert!(root_lib.contains("canic::start!();"));
    assert!(root_lib.contains("canic::finish!();"));
    assert!(app_manifest.contains("name = \"canister_my_app_app\""));
    assert!(app_manifest.contains("fleet = \"my_app\""));
    assert!(app_manifest.contains("role = \"app\""));
    assert!(app_manifest.contains("canic = \""));
    assert!(app_manifest.contains("ic-cdk = \"0.20\""));
    assert!(!app_manifest.contains("workspace = true"));
    assert!(!app_lib.contains("CanisterRole::new"));
    assert!(app_lib.contains("canic::start!();"));
    assert!(app_lib.contains("canic::finish!();"));
}

// Ensure canister scaffold writes a declared-only canister role under one fleet.
#[test]
fn scaffold_canister_writes_declared_only_role_files() {
    let root = temp_dir("canic-cli-scaffold-canister");
    let fleet_dir = root.join("fleets/demo");
    fs::create_dir_all(&fleet_dir).expect("create fleet dir");
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\n    \"fleets/demo/root\",\n]\n",
    )
    .expect("write workspace manifest");
    fs::write(fleet_dir.join("canic.toml"), canic_toml("demo")).expect("write config");
    let options = CanisterScaffoldOptions {
        fleet: "demo".to_string(),
        role: "store".to_string(),
        project_root: Some(root.clone()),
    };

    let result = scaffold_canister(&options).expect("scaffold canister");
    let config = fs::read_to_string(fleet_dir.join("canic.toml")).expect("read config");
    let workspace_manifest = fs::read_to_string(root.join("Cargo.toml")).expect("read workspace");
    let manifest = fs::read_to_string(fleet_dir.join("store/Cargo.toml")).expect("read manifest");
    let build_rs = fs::read_to_string(fleet_dir.join("store/build.rs")).expect("read build");
    let lib = fs::read_to_string(fleet_dir.join("store/src/lib.rs")).expect("read lib");

    fs::remove_dir_all(root).expect("remove scaffold temp root");
    assert_eq!(result.fleet, "demo");
    assert_eq!(result.role, "store");
    assert_eq!(result.package, "store");
    assert_eq!(result.package_name, "canister_demo_store");
    assert_eq!(result.canister_dir, PathBuf::from("fleets/demo/store"));
    assert_eq!(result.config_path, PathBuf::from("fleets/demo/canic.toml"));
    assert!(config.contains("[roles.\"store\"]"));
    assert!(config.contains("package = \"store\""));
    assert!(!config.contains("[subnets.\"prime\".canisters.\"store\"]"));
    assert!(!config.contains("[subnets.prime.canisters.store]"));
    assert!(workspace_manifest.contains("\"fleets/demo/store\""));
    assert!(manifest.contains("name = \"canister_demo_store\""));
    assert!(manifest.contains("fleet = \"demo\""));
    assert!(manifest.contains("role = \"store\""));
    assert!(manifest.contains("crate-type = [\"cdylib\"]"));
    assert!(build_rs.contains("canic::build!(\"../canic.toml\")"));
    assert!(lib.contains("canic::start!();"));
    assert!(lib.contains("pub async fn canic_install(_: Option<Vec<u8>>) {}"));
    assert!(lib.contains("canic::finish!();"));
}

// Ensure workspace member insertion handles compact workspace arrays.
#[test]
fn append_workspace_member_source_updates_compact_members_array() {
    let updated = append_workspace_member_source(
        "[workspace]\nmembers = [\"fleets/demo/root\"]\n",
        "fleets/demo/store",
    )
    .expect("append member");

    assert!(updated.contains("\"fleets/demo/root\""));
    assert!(updated.contains("\"fleets/demo/store\""));
}

// Ensure canister scaffold refuses to overwrite an existing role crate.
#[test]
fn scaffold_canister_rejects_existing_target() {
    let root = temp_dir("canic-cli-scaffold-canister-existing");
    let fleet_dir = root.join("fleets/demo");
    fs::create_dir_all(fleet_dir.join("store")).expect("create existing canister dir");
    fs::write(fleet_dir.join("canic.toml"), canic_toml("demo")).expect("write config");
    let options = CanisterScaffoldOptions {
        fleet: "demo".to_string(),
        role: "store".to_string(),
        project_root: Some(root.clone()),
    };

    let err = scaffold_canister(&options).expect_err("existing scaffold should fail");

    fs::remove_dir_all(root).expect("remove scaffold temp root");
    std::assert_matches!(err, ScaffoldCommandError::TargetExists(_));
}

// Ensure canister scaffold rejects an existing declaration before writing files.
#[test]
fn scaffold_canister_rejects_existing_declaration_without_writing_files() {
    let root = temp_dir("canic-cli-scaffold-canister-declared");
    let fleet_dir = root.join("fleets/demo");
    fs::create_dir_all(&fleet_dir).expect("create fleet dir");
    fs::write(fleet_dir.join("canic.toml"), canic_toml("demo")).expect("write config");
    let options = CanisterScaffoldOptions {
        fleet: "demo".to_string(),
        role: "app".to_string(),
        project_root: Some(root.clone()),
    };

    let err = scaffold_canister(&options).expect_err("declared role should fail");

    std::assert_matches!(err, ScaffoldCommandError::Usage(_));
    assert!(!fleet_dir.join("app").exists());
    fs::remove_dir_all(root).expect("remove scaffold temp root");
}

// Ensure canister scaffold help exposes the declared-only workflow.
#[test]
fn scaffold_canister_usage_lists_fleet_and_role() {
    let text = scaffold_canister_usage();

    assert!(text.contains("Create a declared-only canister role"));
    assert!(text.contains("Usage: canic scaffold canister <fleet> <role>"));
    assert!(text.contains("Examples:"));
}

// Ensure scaffold refuses to overwrite an existing project directory.
#[test]
fn scaffold_project_rejects_existing_target() {
    let root = temp_dir("canic-cli-scaffold-existing");
    let options = ScaffoldOptions {
        name: "my_app".to_string(),
        project_root: Some(root.clone()),
        yes: true,
    };
    fs::create_dir_all(root.join("fleets/my_app")).expect("create existing target");

    let err = scaffold_project(&options).expect_err("existing scaffold should fail");

    fs::remove_dir_all(root).expect("remove scaffold temp root");
    std::assert_matches!(err, ScaffoldCommandError::TargetExists(_));
}
