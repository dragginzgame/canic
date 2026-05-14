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
    assert_eq!(options.fleets_dir, None);
}

// Ensure fleet listing can target a nonstandard fleet config directory.
#[test]
fn parses_fleet_options_fleets_dir() {
    let options = FleetOptions::parse([
        OsString::from("--fleets-dir"),
        OsString::from("deploy/fleets"),
    ])
    .expect("parse fleet options");

    assert_eq!(options.fleets_dir, Some(PathBuf::from("deploy/fleets")));
}

// Ensure fleet delete options require exactly one fleet name.
#[test]
fn parses_delete_fleet_options() {
    let options =
        DeleteFleetOptions::parse([OsString::from("demo")]).expect("parse delete options");

    assert_eq!(options.fleet, "demo");
    assert_eq!(options.fleets_dir, None);
}

// Ensure fleet sync accepts an optional fleet existence guard.
#[test]
fn parses_sync_fleet_filter() {
    let options = FleetSyncOptions::parse_test([OsString::from("--fleet"), OsString::from("test")])
        .expect("parse sync options");

    assert_eq!(options.fleet.as_deref(), Some("test"));
    assert_eq!(options.fleets_dir, None);
}

// Ensure fleet sync can use a caller-supplied fleet config directory.
#[test]
fn parses_sync_fleets_dir() {
    let options = FleetSyncOptions::parse_test([
        OsString::from("--fleet"),
        OsString::from("test"),
        OsString::from("--fleets-dir"),
        OsString::from("deploy/fleets"),
    ])
    .expect("parse sync options");

    assert_eq!(options.fleet.as_deref(), Some("test"));
    assert_eq!(options.fleets_dir, Some(PathBuf::from("deploy/fleets")));
}

// Ensure unknown fleet sync options fail through usage.
#[test]
fn rejects_unknown_sync_option() {
    let err =
        FleetSyncOptions::parse_test([OsString::from("--unknown")]).expect_err("parse should fail");

    assert!(matches!(err, FleetCommandError::Usage(_)));
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
    assert!(matches!(err, FleetCommandError::DeleteCancelled));
}

// Ensure delete resolves the fleet config parent, not an arbitrary path.
#[test]
fn delete_target_resolves_config_parent() {
    let root = temp_dir("canic-fleet-delete-target");
    let demo = write_fleet_config(&root, "demo");
    let staging = write_fleet_config(&root, "staging");
    let choices = vec![demo.join("canic.toml"), staging.join("canic.toml")];

    let target =
        delete_target_dir_from_choices(&root, &choices, "staging", None).expect("delete target");

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
            canisters: "3 (root, app, user_hub)".to_string(),
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
            "-------   -------   -------------------------   -----------------------",
            "demo      local     fleets/demo/canic.toml      3 (root, app, user_hub)",
            "staging   local     fleets/staging/canic.toml   2 (root, app)",
        ]
        .join("\n")
    );
}

// Ensure fleet command help lists the command family without search.
#[test]
fn fleet_usage_lists_subcommands_and_examples() {
    let text = usage();

    assert!(text.contains("Manage Canic fleets"));
    assert!(text.contains("Usage: canic fleet"));
    assert!(text.contains("create"));
    assert!(text.contains("delete"));
    assert!(text.contains("list"));
    assert!(text.contains("sync"));
    assert!(!text.contains("current"));
    assert!(!text.contains("use"));
    assert!(!text.contains("search"));
    assert!(text.contains("Examples:"));
}

// Ensure fleet sync help explains ICP config reconciliation.
#[test]
fn fleet_sync_usage_lists_options_and_examples() {
    let text = sync_usage();

    assert!(text.contains("Sync fleet configs into icp.yaml"));
    assert!(text.contains("Usage: canic fleet sync"));
    assert!(text.contains("--fleet"));
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

[subnets.prime.canisters.root]
kind = "root"
"#
        ),
    )
    .expect("write canic config");
    dir
}
