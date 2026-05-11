use super::*;
use crate::test_support::temp_dir;

// Ensure scaffold options parse the project name and optional root directory.
#[test]
fn parses_scaffold_options() {
    let options = ScaffoldOptions::parse([
        OsString::from("my_app"),
        OsString::from("--dir"),
        OsString::from("tmp_fleets"),
    ])
    .expect("parse scaffold options");

    assert_eq!(options.name, "my_app");
    assert_eq!(options.fleets_dir, PathBuf::from("tmp_fleets"));
    assert!(!options.yes);
}

// Ensure scaffold accepts explicit noninteractive confirmation.
#[test]
fn parses_scaffold_yes_option() {
    let options = ScaffoldOptions::parse([OsString::from("my_app"), OsString::from("--yes")])
        .expect("parse scaffold yes option");

    assert!(options.yes);
}

// Ensure confirmation accepts an explicit yes response.
#[test]
fn confirm_scaffold_accepts_yes() {
    let root = temp_dir("canic-cli-scaffold-confirm-yes");
    let options = ScaffoldOptions {
        name: "my_app".to_string(),
        fleets_dir: root.join("fleets"),
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
        fleets_dir: root.join("fleets"),
        yes: false,
    };
    let mut output = Vec::new();

    let err = confirm_scaffold(&options, io::Cursor::new(b"\n"), &mut output)
        .expect_err("empty response should cancel");

    assert!(matches!(err, ScaffoldCommandError::Cancelled));
}

// Ensure invalid scaffold names are rejected before filesystem writes.
#[test]
fn rejects_invalid_project_names() {
    for name in ["MyApp", "my-app", "_app", "app_", "app__one", "1app"] {
        assert!(matches!(
            ScaffoldOptions::parse([OsString::from(name)]),
            Err(ScaffoldCommandError::Usage(_))
        ));
    }
}

// Ensure scaffold writes the expected minimal root and app files.
#[test]
fn scaffold_project_writes_root_and_app_files() {
    let root = temp_dir("canic-cli-scaffold");
    let options = ScaffoldOptions {
        name: "my_app".to_string(),
        fleets_dir: root.join("fleets"),
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
    assert!(config.contains("name = \"my_app\""));
    assert!(config.contains("[auth.delegated_tokens]"));
    assert!(config.contains("enabled = false"));
    assert!(config.contains("auto_create = [\"app\"]"));
    assert!(config.contains("subnet_index = [\"app\"]"));
    assert!(config.contains("[subnets.prime.canisters.root]"));
    assert!(config.contains("[subnets.prime.canisters.app]"));
    assert!(root_manifest.contains("version = \"0.1.0\""));
    assert!(root_manifest.contains("canic = \""));
    assert!(!root_manifest.contains("workspace = true"));
    assert!(root_lib.contains("canic::start_root!();"));
    assert!(app_manifest.contains("name = \"canister_my_app_app\""));
    assert!(app_manifest.contains("canic = \""));
    assert!(!app_manifest.contains("workspace = true"));
    assert!(app_lib.contains("CanisterRole::new(\"app\")"));
    assert!(app_lib.contains("canic::start!(APP);"));
}

// Ensure scaffold refuses to overwrite an existing project directory.
#[test]
fn scaffold_project_rejects_existing_target() {
    let root = temp_dir("canic-cli-scaffold-existing");
    let options = ScaffoldOptions {
        name: "my_app".to_string(),
        fleets_dir: root.join("fleets"),
        yes: true,
    };
    fs::create_dir_all(options.fleets_dir.join("my_app")).expect("create existing target");

    let err = scaffold_project(&options).expect_err("existing scaffold should fail");

    fs::remove_dir_all(root).expect("remove scaffold temp root");
    assert!(matches!(err, ScaffoldCommandError::TargetExists(_)));
}
