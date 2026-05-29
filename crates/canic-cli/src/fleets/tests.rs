use super::*;
use crate::test_support::temp_dir;
use std::fs;

// Ensure fleet listing options accept network selection.
#[test]
fn parses_fleet_options() {
    let options = FleetOptions::parse([
        OsString::from(crate::cli::globals::INTERNAL_NETWORK_OPTION),
        OsString::from("ic"),
    ])
    .expect("parse fleet options");

    assert_eq!(options.network, "ic");
}

// Ensure fleet delete options require exactly one fleet name.
#[test]
fn parses_delete_fleet_options() {
    let options =
        DeleteFleetOptions::parse([OsString::from("demo")]).expect("parse delete options");

    assert_eq!(options.fleet, "demo");
}

// Ensure fleet check requires one fleet name.
#[test]
fn parses_check_fleet() {
    let options =
        FleetCheckOptions::parse_test([OsString::from("test")]).expect("parse check options");

    assert_eq!(options.fleet, "test");
}

// Ensure role list requires one fleet name.
#[test]
fn parses_role_list_fleet() {
    let options =
        RoleListOptions::parse_test([OsString::from("demo")]).expect("parse role list options");

    assert_eq!(options.fleet, "demo");
}

// Ensure role inspect requires fleet and role names.
#[test]
fn parses_role_inspect_fleet_and_role() {
    let options = RoleInspectOptions::parse_test([OsString::from("demo"), OsString::from("app")])
        .expect("parse role inspect options");

    assert_eq!(options.fleet, "demo");
    assert_eq!(options.role, "app");
}

// Ensure role declaration requires fleet, role, and package path.
#[test]
fn parses_role_declare_fleet_role_and_package() {
    let options = RoleDeclareOptions::parse_test([
        OsString::from("demo"),
        OsString::from("store"),
        OsString::from("--package"),
        OsString::from("store"),
    ])
    .expect("parse role declare options");

    assert_eq!(options.fleet, "demo");
    assert_eq!(options.role, "store");
    assert_eq!(options.package, "store");
}

// Ensure unknown fleet check options fail through usage.
#[test]
fn rejects_unknown_check_option() {
    let err = FleetCheckOptions::parse_test([OsString::from("--unknown")])
        .expect_err("parse should fail");

    std::assert_matches!(err, FleetCommandError::Usage(_));
}

// Ensure fleet deletion requires the exact fleet name as confirmation.
#[test]
fn confirm_delete_fleet_requires_exact_name() {
    let target = Path::new("/tmp/canic/fleets/demo");
    let mut output = Vec::new();

    confirm_delete_fleet("demo", target, io::Cursor::new(b"demo\n"), &mut output)
        .expect("confirm delete");

    let output = String::from_utf8(output).expect("utf8 prompt");
    assert!(output.contains("Delete Canic fleet?"));
    assert!(output.contains("fleet: demo"));
    assert!(output.contains("Type the fleet name to confirm"));

    let err = confirm_delete_fleet("demo", target, io::Cursor::new(b"yes\n"), Vec::new())
        .expect_err("wrong confirmation should cancel");
    std::assert_matches!(err, FleetCommandError::DeleteCancelled);
}

// Ensure delete resolves the fleet config parent, not an arbitrary path.
#[test]
fn delete_target_resolves_config_parent() {
    let root = temp_dir("canic-fleet-delete-target");
    let demo = write_fleet_config(&root, "demo");
    let staging = write_fleet_config(&root, "staging");
    let choices = vec![demo.join("canic.toml"), staging.join("canic.toml")];

    let target = delete_target_dir_from_choices(&root, &choices, "staging").expect("delete target");

    fs::remove_dir_all(&root).expect("remove temp root");
    assert_eq!(target, staging);
}

// Ensure fleet listing renders deterministic config-defined rows.
#[test]
fn renders_fleet_list_table() {
    let table = render_fleet_list_from_rows(vec![
        FleetListRow {
            fleet: "demo".to_string(),
            network: "local".to_string(),
            config: "fleets/demo/canic.toml".to_string(),
            canisters: "4 (root, app, user_hub, user_shard)".to_string(),
        },
        FleetListRow {
            fleet: "staging".to_string(),
            network: "local".to_string(),
            config: "fleets/staging/canic.toml".to_string(),
            canisters: "2 (root, app)".to_string(),
        },
    ]);

    assert_eq!(
        table,
        [
            "FLEET     NETWORK   CONFIG                      CANISTERS",
            "-------   -------   -------------------------   -----------------------------------",
            "demo      local     fleets/demo/canic.toml      4 (root, app, user_hub, user_shard)",
            "staging   local     fleets/staging/canic.toml   2 (root, app)",
        ]
        .join("\n")
    );
}

// Ensure role lifecycle list renders declared-only and attached state.
#[test]
fn renders_role_lifecycle_table() {
    let table = render_role_lifecycle_rows(&[
        ConfiguredRoleLifecycle {
            fleet: "demo".to_string(),
            role: "root".to_string(),
            display: "demo.root".to_string(),
            declaration_kind: "root".to_string(),
            package: Some("canisters/root".to_string()),
            attached: true,
            state: "attached".to_string(),
            topology: Some("prime/root".to_string()),
        },
        ConfiguredRoleLifecycle {
            fleet: "demo".to_string(),
            role: "store".to_string(),
            display: "demo.store".to_string(),
            declaration_kind: "canister".to_string(),
            package: Some("canisters/store".to_string()),
            attached: false,
            state: "declared".to_string(),
            topology: None,
        },
    ]);

    assert_eq!(
        table,
        [
            "ROLE         PACKAGE           STATE      TOPOLOGY",
            "----------   ---------------   --------   ----------",
            "demo.root    canisters/root    attached   prime/root",
            "demo.store   canisters/store   declared   -",
        ]
        .join("\n")
    );
}

// Ensure role inspection explains build and deploy eligibility.
#[test]
fn renders_declared_only_role_inspection() {
    let output = render_role_inspection(&ConfiguredRoleLifecycle {
        fleet: "demo".to_string(),
        role: "store".to_string(),
        display: "demo.store".to_string(),
        declaration_kind: "canister".to_string(),
        package: Some("canisters/store".to_string()),
        attached: false,
        state: "declared".to_string(),
        topology: None,
    });

    assert!(output.contains("role: demo.store"));
    assert!(output.contains("cargo check: allowed"));
    assert!(output.contains("deploy artifact: blocked: role is declared-only"));
    assert!(output.contains("canic fleet role attach demo store"));
}

// Ensure declaration output stays explicit about config-only state.
#[test]
fn renders_declared_role_output() {
    let root = Path::new("/workspace");
    let config = root.join("fleets/demo/canic.toml");
    let output = render_declared_role(
        &DeclaredFleetRole {
            fleet: "demo".to_string(),
            role: "store".to_string(),
            display: "demo.store".to_string(),
            package: "store".to_string(),
        },
        root,
        &config,
    );

    assert!(output.contains("Declared fleet role:"));
    assert!(output.contains("role: demo.store"));
    assert!(output.contains("package: store"));
    assert!(output.contains("config: fleets/demo/canic.toml"));
    assert!(output.contains("state: declared"));
    assert!(output.contains("canic fleet role attach demo store"));
}

// Ensure fleet command help lists the command family without search.
#[test]
fn fleet_usage_lists_subcommands_and_examples() {
    let text = usage();

    assert!(text.contains("Manage Canic fleets"));
    assert!(text.contains("Usage: canic fleet"));
    assert!(text.contains("check"));
    assert!(text.contains("create"));
    assert!(text.contains("delete"));
    assert!(text.contains("list"));
    assert!(text.contains("role"));
    assert!(!text.contains("sync"));
    assert!(!text.contains("current"));
    assert!(!text.contains("use"));
    assert!(!text.contains("search"));
    assert!(text.contains("Examples:"));
}

// Ensure fleet role help lists read-only lifecycle commands.
#[test]
fn fleet_role_usage_lists_subcommands_and_examples() {
    let text = role_usage();

    assert!(text.contains("Manage fleet role lifecycle"));
    assert!(text.contains("Usage: canic fleet role"));
    assert!(text.contains("declare"));
    assert!(text.contains("list"));
    assert!(text.contains("inspect"));
    assert!(text.contains("Examples:"));
}

// Ensure fleet check help explains read-only ICP config checks.
#[test]
fn fleet_check_usage_lists_options_and_examples() {
    let text = check_usage();

    assert!(text.contains("Check icp.yaml for one Canic fleet"));
    assert!(text.contains("Usage: canic fleet check <name>"));
    assert!(!text.contains("--fleet"));
    assert!(text.contains("Examples:"));
}

// Ensure fleet create help explains creation.
#[test]
fn fleet_create_usage_lists_options_and_examples() {
    let text = create_usage();

    assert!(text.contains("Create a minimal Canic fleet"));
    assert!(text.contains("Usage: canic fleet create"));
    assert!(!text.contains("--network"));
    assert!(text.contains("--yes"));
    assert!(text.contains("Examples:"));
}

// Ensure fleet list help explains network selection.
#[test]
fn fleet_list_usage_lists_options_and_examples() {
    let text = list_usage();

    assert!(text.contains("List config-defined Canic fleets"));
    assert!(text.contains("Usage: canic fleet list"));
    assert!(!text.contains("--network"));
    assert!(text.contains("Examples:"));
}

// Ensure fleet delete help explains the destructive confirmation.
#[test]
fn delete_usage_lists_confirmation() {
    let text = delete_usage();

    assert!(text.contains("Delete a config-defined Canic fleet directory"));
    assert!(text.contains("Usage: canic fleet delete"));
    assert!(text.contains("<name>"));
    assert!(text.contains("type the"));
}

// Ensure role list help takes explicit fleet identity.
#[test]
fn role_list_usage_lists_fleet_argument() {
    let text = role_list_usage();

    assert!(text.contains("Usage: canic fleet role list <fleet>"));
    assert!(text.contains("Examples:"));
}

// Ensure role inspect help takes explicit fleet and role identity.
#[test]
fn role_inspect_usage_lists_fleet_and_role_arguments() {
    let text = role_inspect_usage();

    assert!(text.contains("Usage: canic fleet role inspect <fleet> <role>"));
    assert!(text.contains("Examples:"));
}

// Ensure role declare help takes explicit fleet, role, and package path.
#[test]
fn role_declare_usage_lists_required_package() {
    let text = role_declare_usage();

    assert!(text.contains("Usage: canic fleet role declare"));
    assert!(text.contains("<fleet>"));
    assert!(text.contains("<role>"));
    assert!(text.contains("--package <path>"));
    assert!(text.contains("Examples:"));
}

// Render precomputed config rows for focused table tests.
fn render_fleet_list_from_rows(rows: Vec<FleetListRow>) -> String {
    render_fleet_rows(rows)
}

fn write_fleet_config(root: &Path, name: &str) -> PathBuf {
    let dir = root.join("fleets").join(name);
    fs::create_dir_all(dir.join("root")).expect("create root dir");
    fs::write(dir.join("root/Cargo.toml"), "").expect("write root manifest");
    fs::write(
        dir.join("canic.toml"),
        format!(
            r#"
[fleet]
name = "{name}"

[roles.root]
kind = "root"

[subnets.prime.canisters.root]
kind = "root"
"#
        ),
    )
    .expect("write canic config");
    dir
}
