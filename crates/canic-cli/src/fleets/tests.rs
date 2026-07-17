use super::*;
use crate::test_support::TempDir;
use canic_host::adoption::{
    AdoptionArtifactStateV1, AdoptionObservationStateV1, AdoptionPackageStateV1, AdoptionProfileV1,
};
use canic_host::release_set::{
    AttachedFleetRole, ConfiguredRoleLifecycle, DeclaredFleetRole, RenamedFleetRole,
};
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
    assert!(!options.dry_run);
}

// Ensure fleet delete supports target preview mode.
#[test]
fn parses_delete_fleet_dry_run_option() {
    let options = DeleteFleetOptions::parse([OsString::from("demo"), OsString::from("--dry-run")])
        .expect("parse delete dry-run options");

    assert_eq!(options.fleet, "demo");
    assert!(options.dry_run);
}

// Ensure fleet check requires one fleet name.
#[test]
fn parses_check_fleet() {
    let options =
        FleetCheckOptions::parse_test([OsString::from("test")]).expect("parse check options");

    assert_eq!(options.fleet, "test");
}

#[test]
fn fleet_create_dispatch_preserves_scaffold_error() {
    let error = run_create(std::iter::empty::<OsString>())
        .expect_err("missing fleet create arguments reject");

    std::assert_matches!(
        error,
        FleetCommandError::Create(scaffold::ScaffoldCommandError::Usage(_))
    );
}

#[test]
fn fleet_config_dispatch_preserves_list_error() {
    let error = run_config(std::iter::empty::<OsString>())
        .expect_err("missing fleet config arguments reject");

    std::assert_matches!(
        error,
        FleetCommandError::Config(crate::list::ListCommandError::Usage(_))
    );
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
    assert!(!options.dry_run);
}

// Ensure role declaration supports config-write preview mode.
#[test]
fn parses_role_declare_dry_run_option() {
    let options = RoleDeclareOptions::parse_test([
        OsString::from("demo"),
        OsString::from("store"),
        OsString::from("--package"),
        OsString::from("store"),
        OsString::from("--dry-run"),
    ])
    .expect("parse role declare dry-run options");

    assert!(options.dry_run);
}

// Ensure role attachment requires fleet, role, and subnet and defaults to singleton.
#[test]
fn parses_role_attach_fleet_role_and_subnet() {
    let options = RoleAttachOptions::parse_test([
        OsString::from("demo"),
        OsString::from("store"),
        OsString::from("--subnet"),
        OsString::from("prime"),
    ])
    .expect("parse role attach options");

    assert_eq!(options.fleet, "demo");
    assert_eq!(options.role, "store");
    assert_eq!(options.subnet, "prime");
    assert_eq!(options.kind, "singleton");
    assert!(!options.dry_run);
}

// Ensure role attachment accepts explicit non-singleton kind.
#[test]
fn parses_role_attach_kind() {
    let options = RoleAttachOptions::parse_test([
        OsString::from("demo"),
        OsString::from("worker"),
        OsString::from("--subnet"),
        OsString::from("prime"),
        OsString::from("--kind"),
        OsString::from("replica"),
    ])
    .expect("parse role attach options");

    assert_eq!(options.kind, "replica");
}

// Ensure role attachment supports config-write preview mode.
#[test]
fn parses_role_attach_dry_run_option() {
    let options = RoleAttachOptions::parse_test([
        OsString::from("demo"),
        OsString::from("store"),
        OsString::from("--subnet"),
        OsString::from("prime"),
        OsString::from("--dry-run"),
    ])
    .expect("parse role attach dry-run options");

    assert!(options.dry_run);
}

// Ensure role rename requires fleet, old role, and new role names.
#[test]
fn parses_role_rename_fleet_old_role_and_new_role() {
    let options = RoleRenameOptions::parse_test([
        OsString::from("demo"),
        OsString::from("hub"),
        OsString::from("router"),
    ])
    .expect("parse role rename options");

    assert_eq!(options.fleet, "demo");
    assert_eq!(options.old_role, "hub");
    assert_eq!(options.new_role, "router");
    assert!(!options.dry_run);
}

// Ensure role rename supports config/package metadata preview mode.
#[test]
fn parses_role_rename_dry_run_option() {
    let options = RoleRenameOptions::parse_test([
        OsString::from("demo"),
        OsString::from("hub"),
        OsString::from("router"),
        OsString::from("--dry-run"),
    ])
    .expect("parse role rename dry-run options");

    assert!(options.dry_run);
}

// Ensure adoption report requires explicit fleet and profile, with text output by default.
#[test]
fn parses_adoption_report_fleet_profile_and_default_text() {
    let options = AdoptionReportOptions::parse_test([
        OsString::from("demo"),
        OsString::from("--profile"),
        OsString::from("brownfield"),
    ])
    .expect("parse adoption report options");

    assert_eq!(options.fleet, "demo");
    assert_eq!(options.profile, AdoptionProfileV1::Brownfield);
    assert_eq!(options.format, AdoptionReportFormat::Text);
    assert_eq!(options.deployment_check, None);
    assert_eq!(options.inventory, None);
    assert_eq!(options.artifact_manifest, None);
    assert_eq!(options.cargo_metadata, None);
    assert_eq!(options.package_metadata, None);
    assert_eq!(options.build_provenance, None);
    assert_eq!(options.output, None);
}

// Ensure adoption report can read explicit evidence paths and emit JSON output.
#[test]
fn parses_adoption_report_json_output() {
    let options = AdoptionReportOptions::parse_test([
        OsString::from("demo"),
        OsString::from("--profile"),
        OsString::from("minimal"),
        OsString::from("--json"),
        OsString::from("--deployment-check"),
        OsString::from("check.json"),
        OsString::from("--artifact-manifest"),
        OsString::from("artifacts.json"),
        OsString::from("--package-metadata"),
        OsString::from("packages.json"),
        OsString::from("--output"),
        OsString::from("report.json"),
    ])
    .expect("parse adoption report options");

    assert_eq!(options.profile, AdoptionProfileV1::Minimal);
    assert_eq!(options.format, AdoptionReportFormat::Json);
    assert_eq!(options.deployment_check, Some(PathBuf::from("check.json")));
    assert_eq!(options.inventory, None);
    assert_eq!(
        options.artifact_manifest,
        Some(PathBuf::from("artifacts.json"))
    );
    assert_eq!(options.cargo_metadata, None);
    assert_eq!(
        options.package_metadata,
        Some(PathBuf::from("packages.json"))
    );
    assert_eq!(options.build_provenance, None);
    assert_eq!(options.output, Some(PathBuf::from("report.json")));
}

// Ensure adoption report accepts stable envelope JSON without changing raw JSON.
#[test]
fn parses_adoption_report_envelope_json_output() {
    let options = AdoptionReportOptions::parse_test([
        OsString::from("demo"),
        OsString::from("--profile"),
        OsString::from("minimal"),
        OsString::from("--evidence-envelope"),
    ])
    .expect("parse adoption report options");

    assert_eq!(options.format, AdoptionReportFormat::EnvelopeJson);
}

// Ensure build provenance evidence is accepted only for stable envelope output.
#[test]
fn parses_adoption_report_build_provenance_envelope_input() {
    let options = AdoptionReportOptions::parse_test([
        OsString::from("demo"),
        OsString::from("--profile"),
        OsString::from("minimal"),
        OsString::from("--evidence-envelope"),
        OsString::from("--build-provenance"),
        OsString::from("build-provenance.json"),
    ])
    .expect("parse adoption report options");

    assert_eq!(
        options.build_provenance,
        Some(PathBuf::from("build-provenance.json"))
    );
}

// Ensure adoption report can read cargo metadata evidence from an explicit path.
#[test]
fn parses_adoption_report_cargo_metadata_path() {
    let options = AdoptionReportOptions::parse_test([
        OsString::from("demo"),
        OsString::from("--profile"),
        OsString::from("partial"),
        OsString::from("--cargo-metadata"),
        OsString::from("cargo-metadata.json"),
    ])
    .expect("parse adoption report options");

    assert_eq!(
        options.cargo_metadata,
        Some(PathBuf::from("cargo-metadata.json"))
    );
    assert_eq!(options.package_metadata, None);
}

// Ensure adoption report rejects ambiguous inventory evidence sources at parse time.
#[test]
fn rejects_adoption_report_inventory_and_deployment_check_together() {
    let err = AdoptionReportOptions::parse_test([
        OsString::from("demo"),
        OsString::from("--profile"),
        OsString::from("partial"),
        OsString::from("--deployment-check"),
        OsString::from("check.json"),
        OsString::from("--inventory"),
        OsString::from("inventory.json"),
    ])
    .expect_err("ambiguous inventory evidence should fail");

    std::assert_matches!(err, FleetCommandError::Usage(_));
}

// Ensure adoption report rejects ambiguous package metadata sources at parse time.
#[test]
fn rejects_adoption_report_package_metadata_and_cargo_metadata_together() {
    let err = AdoptionReportOptions::parse_test([
        OsString::from("demo"),
        OsString::from("--profile"),
        OsString::from("partial"),
        OsString::from("--package-metadata"),
        OsString::from("packages.json"),
        OsString::from("--cargo-metadata"),
        OsString::from("cargo-metadata.json"),
    ])
    .expect_err("ambiguous package metadata evidence should fail");

    std::assert_matches!(err, FleetCommandError::Usage(_));
}

// Ensure unsupported adoption profiles fail before any report generation.
#[test]
fn rejects_unknown_adoption_profile() {
    let err = AdoptionReportOptions::parse_test([
        OsString::from("demo"),
        OsString::from("--profile"),
        OsString::from("import"),
    ])
    .expect_err("unknown profile should fail");

    std::assert_matches!(err, FleetCommandError::Usage(_));
}

// Ensure raw JSON and envelope artifact modes are mutually exclusive.
#[test]
fn rejects_ambiguous_adoption_report_output_modes() {
    let err = AdoptionReportOptions::parse_test([
        OsString::from("demo"),
        OsString::from("--profile"),
        OsString::from("brownfield"),
        OsString::from("--json"),
        OsString::from("--evidence-envelope"),
    ])
    .expect_err("ambiguous output mode should fail");

    std::assert_matches!(err, FleetCommandError::Usage(_));
}

// Ensure provenance evidence cannot silently no-op on raw adoption report output.
#[test]
fn rejects_adoption_report_build_provenance_without_envelope_output() {
    let err = AdoptionReportOptions::parse_test([
        OsString::from("demo"),
        OsString::from("--profile"),
        OsString::from("minimal"),
        OsString::from("--build-provenance"),
        OsString::from("build-provenance.json"),
    ])
    .expect_err("build provenance requires envelope output");

    std::assert_matches!(err, FleetCommandError::Usage(_));
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
    let root = TempDir::new("canic-fleet-delete-target");
    let demo = write_fleet_config(&root, "demo");
    let staging = write_fleet_config(&root, "staging");
    let choices = vec![demo.join("canic.toml"), staging.join("canic.toml")];

    let target = delete_target_dir_from_choices(&root, &choices, "staging").expect("delete target");

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
            package: "canisters/root".to_string(),
            attached: true,
            state: "attached".to_string(),
            topology: Some("prime/root".to_string()),
        },
        ConfiguredRoleLifecycle {
            fleet: "demo".to_string(),
            role: "store".to_string(),
            display: "demo.store".to_string(),
            declaration_kind: "canister".to_string(),
            package: "canisters/store".to_string(),
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
        package: "canisters/store".to_string(),
        attached: false,
        state: "declared".to_string(),
        topology: None,
    });

    assert!(output.contains("role: demo.store"));
    assert!(output.contains("cargo check: allowed"));
    assert!(output.contains("deploy artifact: blocked: role is declared-only"));
    assert!(output.contains("canic fleet role attach demo store --subnet <subnet>"));
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
    assert!(output.contains("canic fleet role attach demo store --subnet <subnet>"));
}

// Ensure declaration dry-run output is explicit about no writes.
#[test]
fn renders_planned_declared_role_output() {
    let root = Path::new("/workspace");
    let config = root.join("fleets/demo/canic.toml");
    let output = render_planned_declared_role(
        &DeclaredFleetRole {
            fleet: "demo".to_string(),
            role: "store".to_string(),
            display: "demo.store".to_string(),
            package: "store".to_string(),
        },
        root,
        &config,
    );

    assert!(output.contains("Planned fleet role declaration:"));
    assert!(output.contains("role: demo.store"));
    assert!(output.contains("would_write: fleets/demo/canic.toml"));
    assert!(output.contains("dry_run: true"));
    assert!(output.contains("files_changed: 0"));
}

// Ensure attachment output points at artifact build as the next step.
#[test]
fn renders_attached_role_output() {
    let root = Path::new("/workspace");
    let config = root.join("fleets/demo/canic.toml");
    let output = render_attached_role(
        &AttachedFleetRole {
            fleet: "demo".to_string(),
            role: "store".to_string(),
            display: "demo.store".to_string(),
            subnet: "prime".to_string(),
            kind: "singleton".to_string(),
            topology: "prime/store".to_string(),
        },
        root,
        &config,
    );

    assert!(output.contains("Attached fleet role:"));
    assert!(output.contains("role: demo.store"));
    assert!(output.contains("kind: singleton"));
    assert!(output.contains("topology: prime/store"));
    assert!(output.contains("config: fleets/demo/canic.toml"));
    assert!(output.contains("state: attached"));
    assert!(output.contains("canic build demo store"));
}

// Ensure attachment dry-run output names the topology and config target.
#[test]
fn renders_planned_attached_role_output() {
    let root = Path::new("/workspace");
    let config = root.join("fleets/demo/canic.toml");
    let output = render_planned_attached_role(
        &AttachedFleetRole {
            fleet: "demo".to_string(),
            role: "store".to_string(),
            display: "demo.store".to_string(),
            subnet: "prime".to_string(),
            kind: "singleton".to_string(),
            topology: "prime/store".to_string(),
        },
        root,
        &config,
    );

    assert!(output.contains("Planned fleet role attachment:"));
    assert!(output.contains("topology: prime/store"));
    assert!(output.contains("would_write: fleets/demo/canic.toml"));
    assert!(output.contains("files_changed: 0"));
}

// Ensure rename output reports config and package metadata updates.
#[test]
fn renders_renamed_role_output() {
    let root = Path::new("/workspace");
    let config = root.join("fleets/demo/canic.toml");
    let manifest = root.join("fleets/demo/router/Cargo.toml");
    let output = render_renamed_role(
        &RenamedFleetRole {
            fleet: "demo".to_string(),
            old_role: "hub".to_string(),
            new_role: "router".to_string(),
            old_display: "demo.hub".to_string(),
            new_display: "demo.router".to_string(),
            package_manifest: Some(manifest),
            package_manifest_note: None,
        },
        root,
        &config,
    );

    assert!(output.contains("Renamed fleet role:"));
    assert!(output.contains("old: demo.hub"));
    assert!(output.contains("new: demo.router"));
    assert!(output.contains("config: fleets/demo/canic.toml"));
    assert!(output.contains("package_manifest: fleets/demo/router/Cargo.toml"));
    assert!(output.contains("canic fleet role inspect demo router"));
}

// Ensure rename dry-run output names both possible write targets.
#[test]
fn renders_planned_renamed_role_output() {
    let root = Path::new("/workspace");
    let config = root.join("fleets/demo/canic.toml");
    let manifest = root.join("fleets/demo/router/Cargo.toml");
    let output = render_planned_renamed_role(
        &RenamedFleetRole {
            fleet: "demo".to_string(),
            old_role: "hub".to_string(),
            new_role: "router".to_string(),
            old_display: "demo.hub".to_string(),
            new_display: "demo.router".to_string(),
            package_manifest: Some(manifest),
            package_manifest_note: None,
        },
        root,
        &config,
    );

    assert!(output.contains("Planned fleet role rename:"));
    assert!(output.contains("old: demo.hub"));
    assert!(output.contains("new: demo.router"));
    assert!(output.contains("would_write: fleets/demo/canic.toml"));
    assert!(output.contains("would_write_package_manifest: fleets/demo/router/Cargo.toml"));
    assert!(output.contains("files_changed: 0"));
}

// Ensure delete dry-run output names the safe target without deleting.
#[test]
fn renders_planned_delete_output() {
    let root = Path::new("/workspace");
    let target = root.join("fleets/demo");
    let output = render_planned_delete(root, "demo", &target);

    assert!(output.contains("Planned fleet delete:"));
    assert!(output.contains("fleet: demo"));
    assert!(output.contains("would_remove: fleets/demo"));
    assert!(output.contains("files_changed: 0"));
}

// Ensure text adoption reports summarize lifecycle state without mutating config.
#[test]
fn renders_adoption_report_text_for_declared_only_roles() {
    let root = TempDir::new("canic-fleet-adoption-report");
    let demo = write_fleet_config(&root, "demo");
    let config_path = demo.join("canic.toml");
    let before = fs::read_to_string(&config_path).expect("read config before report");
    let options = AdoptionReportOptions {
        fleet: "demo".to_string(),
        profile: AdoptionProfileV1::Brownfield,
        format: AdoptionReportFormat::Text,
        deployment_check: None,
        inventory: None,
        artifact_manifest: None,
        cargo_metadata: None,
        package_metadata: None,
        build_provenance: None,
        output: None,
    };

    let report =
        build_adoption_report_from_config_path(&config_path, &options, "unix:1").expect("report");
    let text = render_adoption_report(&report);
    let after = fs::read_to_string(&config_path).expect("read config after report");

    assert_eq!(after, before);
    assert!(text.contains("Adoption report:"));
    assert!(text.contains("profile: brownfield"));
    assert!(text.contains("read_only: true"));
    assert!(text.contains("demo.store: declared-only"));
    assert!(text.contains("deployment inventory was not supplied"));
    assert!(text.contains("mutating_actions_performed: 0"));
    assert!(text.contains("Recommendations (report-only; not executed):"));
    assert!(text.contains(
        "suggested_action_preview: canic fleet role attach demo store --subnet <subnet>"
    ));
    assert!(text.contains("status: not executed by adoption report"));
    assert!(!text.contains("suggested_action:"));
    assert!(text.contains("Blocked adoption actions (not executed by report):"));
    assert!(text.contains("topology attachment"));
}

// Ensure adoption report --output writes only the requested JSON report artifact.
#[test]
fn writes_adoption_report_json_output_file() {
    let root = TempDir::new("canic-fleet-adoption-json");
    let demo = write_fleet_config(&root, "demo");
    let config_path = demo.join("canic.toml");
    let out = root.join("reports/adoption.json");
    let options = AdoptionReportOptions {
        fleet: "demo".to_string(),
        profile: AdoptionProfileV1::Minimal,
        format: AdoptionReportFormat::Json,
        deployment_check: None,
        inventory: None,
        artifact_manifest: None,
        cargo_metadata: None,
        package_metadata: None,
        build_provenance: None,
        output: Some(out.clone()),
    };

    let report =
        build_adoption_report_from_config_path(&config_path, &options, "unix:2").expect("report");
    write_adoption_report(&config_path, &options, &report).expect("write report");
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(&out).expect("read report")).expect("parse report");

    assert_eq!(value["fleet"], "demo");
    assert_eq!(value["profile"], "Minimal");
    assert_eq!(value["summary"]["mutating_actions_performed"], 0);
    assert!(value.get("envelope_schema").is_none());
}

// Ensure envelope JSON wraps the raw adoption report with stable provenance fields.
#[test]
fn writes_adoption_report_envelope_json_output_file() {
    let root = TempDir::new("canic-fleet-adoption-envelope-json");
    let demo = write_fleet_config(&root, "demo");
    let config_path = demo.join("canic.toml");
    let evidence = write_adoption_evidence_files(&root);
    let out = root.join("reports/adoption-envelope.json");
    let before = fs::read_to_string(&config_path).expect("read config before envelope");
    let options = AdoptionReportOptions {
        fleet: "demo".to_string(),
        profile: AdoptionProfileV1::Partial,
        format: AdoptionReportFormat::EnvelopeJson,
        deployment_check: None,
        inventory: Some(evidence.inventory),
        artifact_manifest: Some(evidence.artifact_manifest),
        cargo_metadata: None,
        package_metadata: Some(evidence.package_metadata),
        build_provenance: Some(evidence.build_provenance),
        output: Some(out.clone()),
    };

    let report =
        build_adoption_report_from_config_path(&config_path, &options, "unix:51").expect("report");
    write_adoption_report(&config_path, &options, &report).expect("write report");
    let after = fs::read_to_string(&config_path).expect("read config after envelope");
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(&out).expect("read report")).expect("parse envelope");

    assert_eq!(after, before);
    assert_eq!(value["envelope_schema"]["id"], "canic.evidence_envelope.v1");
    assert_eq!(value["envelope_schema"]["stability"], "stable");
    assert_eq!(value["command"]["name"], "canic fleet adoption report");
    assert_eq!(value["command"]["format"], "envelope-json");
    assert_eq!(value["target"]["kind"], "fleet_adoption");
    assert_eq!(value["target"]["fleet"], "demo");
    assert_eq!(value["target"]["profile"], "partial");
    assert_eq!(value["payload_schema"]["id"], "canic.adoption_report.v1");
    assert_eq!(value["payload_schema"]["stability"], "experimental");
    assert_eq!(value["payload"]["fleet"], "demo");
    assert_eq!(value["payload"]["profile"], "Partial");
    assert!(
        value["payload_sha256"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64)
    );
    assert_eq!(value["source_config"]["kind"], "canic_config");
    assert_eq!(value["source_config"]["path"], "canic.toml");
    assert_eq!(value["source_config"]["path_display"], "relative");
    assert!(
        value["inputs"]
            .as_array()
            .expect("inputs array")
            .iter()
            .any(|input| input["kind"] == "deployment_inventory")
    );
    assert!(
        value["inputs"]
            .as_array()
            .expect("inputs array")
            .iter()
            .any(|input| input["kind"] == "build_provenance"
                && input["schema"]["id"] == "canic.build_provenance.v1"
                && input["schema"]["stability"] == "stable")
    );
    assert!(
        value["command"]["argv_normalized"]
            .as_array()
            .expect("argv")
            .iter()
            .any(|arg| arg == "--build-provenance")
    );
    assert!(
        value["summary"]["missing_or_stale_evidence"]
            .as_array()
            .expect("missing evidence array")
            .is_empty()
    );
}

// Ensure explicit evidence files are read and passed to the host adoption builder.
#[test]
fn adoption_report_reads_explicit_evidence_files() {
    let root = TempDir::new("canic-fleet-adoption-evidence");
    let demo = write_fleet_config(&root, "demo");
    let config_path = demo.join("canic.toml");
    let evidence = write_adoption_evidence_files(&root);

    let options = AdoptionReportOptions {
        fleet: "demo".to_string(),
        profile: AdoptionProfileV1::Partial,
        format: AdoptionReportFormat::Text,
        deployment_check: None,
        inventory: Some(evidence.inventory),
        artifact_manifest: Some(evidence.artifact_manifest),
        cargo_metadata: None,
        package_metadata: Some(evidence.package_metadata),
        build_provenance: None,
        output: None,
    };

    let report =
        build_adoption_report_from_config_path(&config_path, &options, "unix:3").expect("report");
    let store = report
        .role_findings
        .iter()
        .find(|finding| finding.role == "store")
        .expect("store finding");

    assert_eq!(report.inputs.inventory_id.as_deref(), Some("inventory-1"));
    assert_eq!(
        report.inputs.artifact_manifest_id.as_deref(),
        Some("manifest-1")
    );
    assert_eq!(report.inputs.package_metadata_count, 1);
    assert_eq!(store.package_state, AdoptionPackageStateV1::Matches);
    assert_eq!(
        store.observation_state,
        AdoptionObservationStateV1::Observed
    );
    assert_eq!(store.artifact_state, AdoptionArtifactStateV1::CanicBuilt);
}

// Ensure deployment-check evidence can supply inventory without live discovery.
#[test]
fn adoption_report_reads_inventory_from_deployment_check_file() {
    let root = TempDir::new("canic-fleet-adoption-check-evidence");
    let demo = write_fleet_config(&root, "demo");
    let config_path = demo.join("canic.toml");
    let evidence = write_adoption_evidence_files(&root);

    let options = AdoptionReportOptions {
        fleet: "demo".to_string(),
        profile: AdoptionProfileV1::Partial,
        format: AdoptionReportFormat::Text,
        deployment_check: Some(evidence.deployment_check),
        inventory: None,
        artifact_manifest: None,
        cargo_metadata: None,
        package_metadata: None,
        build_provenance: None,
        output: None,
    };

    let report =
        build_adoption_report_from_config_path(&config_path, &options, "unix:4").expect("report");
    let store = report
        .role_findings
        .iter()
        .find(|finding| finding.role == "store")
        .expect("store finding");

    assert_eq!(report.inputs.inventory_id.as_deref(), Some("inventory-1"));
    assert_eq!(
        report.inputs.artifact_manifest_id.as_deref(),
        Some("deployment-check:check-1:role-artifacts")
    );
    assert_eq!(
        store.observation_state,
        AdoptionObservationStateV1::Observed
    );
    assert_eq!(store.artifact_state, AdoptionArtifactStateV1::CanicBuilt);
}

// Ensure explicit artifact-manifest evidence wins over deployment-check plan artifacts.
#[test]
fn adoption_report_artifact_manifest_overrides_deployment_check_artifacts() {
    let root = TempDir::new("canic-fleet-adoption-artifact-precedence");
    let demo = write_fleet_config(&root, "demo");
    let config_path = demo.join("canic.toml");
    let evidence = write_adoption_evidence_files(&root);
    let explicit_artifact_manifest = root.join("explicit-artifact-manifest.json");
    let mut manifest = adoption_artifact_manifest_fixture();
    manifest["manifest_id"] = serde_json::Value::String("explicit-manifest".to_string());
    manifest["role_artifacts"][0]["source"] = serde_json::Value::String("External".to_string());
    write_json_fixture(&explicit_artifact_manifest, manifest);

    let options = AdoptionReportOptions {
        fleet: "demo".to_string(),
        profile: AdoptionProfileV1::Partial,
        format: AdoptionReportFormat::Text,
        deployment_check: Some(evidence.deployment_check),
        inventory: None,
        artifact_manifest: Some(explicit_artifact_manifest),
        cargo_metadata: None,
        package_metadata: None,
        build_provenance: None,
        output: None,
    };

    let report =
        build_adoption_report_from_config_path(&config_path, &options, "unix:9").expect("report");
    let store = report
        .role_findings
        .iter()
        .find(|finding| finding.role == "store")
        .expect("store finding");

    assert_eq!(report.inputs.inventory_id.as_deref(), Some("inventory-1"));
    assert_eq!(
        report.inputs.artifact_manifest_id.as_deref(),
        Some("explicit-manifest")
    );
    assert_eq!(store.artifact_state, AdoptionArtifactStateV1::ExternalWasm);
    assert!(
        store
            .evidence
            .iter()
            .any(|evidence| evidence == "artifact manifest source=external")
    );
}

// Ensure text adoption reports expose observed canister evidence details.
#[test]
fn renders_adoption_report_text_with_observed_canister_evidence() {
    let root = TempDir::new("canic-fleet-adoption-observed-text");
    let demo = write_fleet_config(&root, "demo");
    let config_path = demo.join("canic.toml");
    let evidence = write_adoption_evidence_files(&root);

    let options = AdoptionReportOptions {
        fleet: "demo".to_string(),
        profile: AdoptionProfileV1::Partial,
        format: AdoptionReportFormat::Text,
        deployment_check: Some(evidence.deployment_check),
        inventory: None,
        artifact_manifest: None,
        cargo_metadata: None,
        package_metadata: None,
        build_provenance: None,
        output: None,
    };

    let report =
        build_adoption_report_from_config_path(&config_path, &options, "unix:8").expect("report");
    let text = render_adoption_report(&report);

    assert!(text.contains("Observed canisters:"));
    assert!(text.contains("aaaaa-aa: role=store, confidence=candidate"));
    assert!(text.contains("controllers: controller-a"));
    assert!(text.contains("wasm_evidence: module_hash=hash-a"));
    assert!(text.contains("deployment_target_evidence: inventory-1"));
}

// Ensure cargo metadata evidence can supply package role metadata without live Cargo.
#[test]
fn adoption_report_reads_package_metadata_from_cargo_metadata_file() {
    let root = TempDir::new("canic-fleet-adoption-cargo-metadata");
    let demo = write_fleet_config(&root, "demo");
    let config_path = demo.join("canic.toml");
    let evidence = write_adoption_evidence_files(&root);

    let options = AdoptionReportOptions {
        fleet: "demo".to_string(),
        profile: AdoptionProfileV1::Partial,
        format: AdoptionReportFormat::Text,
        deployment_check: None,
        inventory: None,
        artifact_manifest: None,
        cargo_metadata: Some(evidence.cargo_metadata),
        package_metadata: None,
        build_provenance: None,
        output: None,
    };

    let report =
        build_adoption_report_from_config_path(&config_path, &options, "unix:5").expect("report");
    let store = report
        .role_findings
        .iter()
        .find(|finding| finding.role == "store")
        .expect("store finding");

    assert_eq!(report.inputs.package_metadata_count, 1);
    assert_eq!(store.package_state, AdoptionPackageStateV1::Matches);
}

// Ensure cargo metadata package roots match package = "." declarations.
#[test]
fn cargo_metadata_package_path_preserves_current_directory_package() {
    let root = Path::new("/workspace/fleets/demo");
    let package = serde_json::json!({
        "manifest_path": "/workspace/fleets/demo/Cargo.toml"
    });

    assert_eq!(
        cargo_metadata_package_path(root, &package).as_deref(),
        Some(".")
    );
}

// Ensure cargo metadata package roots can match sibling relative declarations.
#[test]
fn cargo_metadata_package_path_normalizes_sibling_package() {
    let config_dir = Path::new("/workspace/fleets/test/test-configs");
    let package = serde_json::json!({
        "manifest_path": "/workspace/fleets/test/test/Cargo.toml"
    });

    assert_eq!(
        cargo_metadata_package_path(config_dir, &package).as_deref(),
        Some("../test")
    );
}

struct AdoptionEvidenceFiles {
    deployment_check: PathBuf,
    inventory: PathBuf,
    artifact_manifest: PathBuf,
    cargo_metadata: PathBuf,
    package_metadata: PathBuf,
    build_provenance: PathBuf,
}

fn write_adoption_evidence_files(root: &Path) -> AdoptionEvidenceFiles {
    let files = AdoptionEvidenceFiles {
        deployment_check: root.join("deployment-check.json"),
        inventory: root.join("inventory.json"),
        artifact_manifest: root.join("artifact-manifest.json"),
        cargo_metadata: root.join("cargo-metadata.json"),
        package_metadata: root.join("package-metadata.json"),
        build_provenance: root.join("build-provenance.json"),
    };

    write_json_fixture(&files.deployment_check, adoption_deployment_check_fixture());
    write_json_fixture(&files.inventory, adoption_inventory_fixture());
    write_json_fixture(
        &files.artifact_manifest,
        adoption_artifact_manifest_fixture(),
    );
    write_json_fixture(&files.cargo_metadata, adoption_cargo_metadata_fixture(root));
    write_json_fixture(&files.package_metadata, adoption_package_metadata_fixture());
    write_json_fixture(&files.build_provenance, build_provenance_fixture());
    files
}

fn write_json_fixture(path: &Path, value: serde_json::Value) {
    fs::write(path, serde_json::to_vec(&value).expect("encode fixture")).expect("write fixture");
}

fn build_provenance_fixture() -> serde_json::Value {
    serde_json::json!({
        "envelope_schema": {
            "id": "canic.evidence_envelope.v1",
            "version": "1",
            "stability": "stable"
        },
        "payload_schema": {
            "id": "canic.build_provenance.v1",
            "version": "1",
            "stability": "stable"
        },
        "payload": {
            "schema_version": 1,
            "build_status": "success"
        }
    })
}

fn adoption_deployment_check_fixture() -> serde_json::Value {
    serde_json::json!({
        "check_id": "check-1",
        "plan": {
            "deployment_identity": {
                "network": "local"
            },
            "role_artifacts": [adoption_role_artifact_fixture()]
        },
        "inventory": adoption_inventory_fixture()
    })
}

fn adoption_inventory_fixture() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "inventory_id": "inventory-1",
        "observed_at": "2026-05-30T00:00:00Z",
        "observed_identity": null,
        "observed_root": null,
        "local_config": {
            "config_path": "fleets/demo/canic.toml",
            "raw_config_sha256": null,
            "canonical_embedded_config_sha256": null
        },
        "observed_canisters": [{
            "canister_id": "aaaaa-aa",
            "role": "store",
            "control_class": "DeploymentControlled",
            "controllers": ["controller-a"],
            "module_hash": "hash-a",
            "status": "running",
            "root_trust_anchor": null,
            "canonical_embedded_config_digest": null,
            "role_assignment_source": "fixture"
        }],
        "observed_pool": [],
        "observed_artifacts": [],
        "observed_verifier_readiness": {
            "status": "NotObserved",
            "role_epochs": []
        },
        "unresolved_observations": []
    })
}

fn adoption_artifact_manifest_fixture() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "manifest_id": "manifest-1",
        "network": "local",
        "artifact_root": null,
        "role_artifacts": [adoption_role_artifact_fixture()],
        "unresolved_artifacts": []
    })
}

fn adoption_role_artifact_fixture() -> serde_json::Value {
    serde_json::json!({
        "role": "store",
        "source": "LocalBuild",
        "build_profile": "fast",
        "wasm_path": null,
        "wasm_gz_path": null,
        "wasm_gz_size_bytes": null,
        "wasm_sha256": null,
        "wasm_gz_sha256": null,
        "wasm_gz_sha256_source": null,
        "observed_wasm_gz_file_sha256": null,
        "observed_wasm_gz_file_sha256_source": null,
        "installed_module_hash": null,
        "candid_path": null,
        "candid_sha256": null,
        "raw_config_sha256": null,
        "canonical_embedded_config_sha256": null,
        "embedded_topology_sha256": null,
        "builder_version": null,
        "rust_toolchain": null,
        "package_version": null
    })
}

fn adoption_package_metadata_fixture() -> serde_json::Value {
    serde_json::json!([{
        "package": "store",
        "fleet": "demo",
        "role": "store"
    }])
}

fn adoption_cargo_metadata_fixture(root: &Path) -> serde_json::Value {
    serde_json::json!({
        "packages": [{
            "name": "store",
            "manifest_path": root
                .join("fleets/demo/store/Cargo.toml")
                .to_string_lossy()
                .to_string(),
            "metadata": {
                "canic": {
                    "fleet": "demo",
                    "role": "store"
                }
            }
        }, {
            "name": "without-canic-metadata",
            "manifest_path": root
                .join("fleets/demo/ignored/Cargo.toml")
                .to_string_lossy()
                .to_string(),
            "metadata": {}
        }]
    })
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
    assert!(text.contains("adoption"));
    assert!(text.contains("role"));
    assert!(!text.contains("sync"));
    assert!(!text.contains("current"));
    assert!(!text.contains("use"));
    assert!(!text.contains("search"));
    assert!(text.contains("Examples:"));
    assert!(text.contains("Mutation notes:"));
    assert!(
        text.contains(
            "canic fleet check/list/config/adoption/role list/role inspect are read-only"
        )
    );
    assert!(text.contains("Mutating fleet commands that can be previewed expose --dry-run"));
}

// Ensure fleet adoption help lists the read-only report command.
#[test]
fn fleet_adoption_usage_lists_subcommands_and_examples() {
    let text = adoption_usage();

    assert!(text.contains("Report safe onboarding recommendations"));
    assert!(text.contains("Usage: canic fleet adoption"));
    assert!(text.contains("report"));
    assert!(text.contains("read-only"));
    assert!(text.contains("Examples:"));
}

// Ensure fleet role help lists read-only lifecycle commands.
#[test]
fn fleet_role_usage_lists_subcommands_and_examples() {
    let text = role_usage();

    assert!(text.contains("Manage fleet role lifecycle"));
    assert!(text.contains("Usage: canic fleet role"));
    assert!(text.contains("declare"));
    assert!(text.contains("attach"));
    assert!(text.contains("rename"));
    assert!(text.contains("list"));
    assert!(text.contains("inspect"));
    assert!(text.contains("Examples:"));
    assert!(text.contains("Mutation notes:"));
    assert!(text.contains("declare and attach update canic.toml"));
    assert!(text.contains("rename updates canic.toml"));
}

// Ensure fleet check help explains read-only ICP config checks.
#[test]
fn fleet_check_usage_lists_options_and_examples() {
    let text = check_usage();

    assert!(text.contains("Check icp.yaml for one Canic fleet"));
    assert!(text.contains("Usage: canic fleet check <name>"));
    assert!(text.contains("Examples:"));
}

// Ensure fleet create help explains creation.
#[test]
fn fleet_create_usage_lists_options_and_examples() {
    let text = create_usage();

    assert!(text.contains("Create a minimal Canic fleet"));
    assert!(text.contains("Usage: canic fleet create"));
    assert!(text.contains("--yes"));
    assert!(text.contains("Examples:"));
}

// Ensure fleet list help explains network selection.
#[test]
fn fleet_list_usage_lists_options_and_examples() {
    let text = list_usage();

    assert!(text.contains("List config-defined Canic fleets"));
    assert!(text.contains("Usage: canic fleet list"));
    assert!(text.contains("Examples:"));
}

// Ensure fleet delete help explains the destructive confirmation.
#[test]
fn delete_usage_lists_confirmation() {
    let text = delete_usage();

    assert!(text.contains("Delete a config-defined Canic fleet directory"));
    assert!(text.contains("Usage: canic fleet delete"));
    assert!(text.contains("<name>"));
    assert!(text.contains("--dry-run"));
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
    assert!(text.contains("--dry-run"));
    assert!(text.contains("Examples:"));
}

// Ensure role attach help takes explicit fleet, role, and subnet.
#[test]
fn role_attach_usage_lists_required_subnet() {
    let text = role_attach_usage();

    assert!(text.contains("Usage: canic fleet role attach"));
    assert!(text.contains("<fleet>"));
    assert!(text.contains("<role>"));
    assert!(text.contains("--subnet <subnet>"));
    assert!(text.contains("--kind <kind>"));
    assert!(text.contains("--dry-run"));
    assert!(text.contains("Examples:"));
}

// Ensure role rename help takes explicit fleet, old role, and new role identity.
#[test]
fn role_rename_usage_lists_fleet_old_role_and_new_role_arguments() {
    let text = role_rename_usage();

    assert!(text.contains("Usage: canic fleet role rename"));
    assert!(text.contains("<fleet>"));
    assert!(text.contains("<old-role>"));
    assert!(text.contains("<new-role>"));
    assert!(text.contains("--dry-run"));
    assert!(text.contains("Examples:"));
}

// Ensure adoption report help takes explicit fleet and profile identity.
#[test]
fn adoption_report_usage_lists_profile_and_output_options() {
    let text = adoption_report_usage();

    assert!(text.contains("Usage: canic fleet adoption report"));
    assert!(text.contains("--profile <profile>"));
    assert!(text.contains("<fleet>"));
    assert!(text.contains("--json"));
    assert!(text.contains("--evidence-envelope"));
    assert!(text.contains("--deployment-check <path>"));
    assert!(text.contains("--inventory <path>"));
    assert!(text.contains("--artifact-manifest <path>"));
    assert!(text.contains("--cargo-metadata <path>"));
    assert!(text.contains("--package-metadata <path>"));
    assert!(text.contains("--build-provenance <path>"));
    assert!(text.contains("--output <path>"));
    assert!(text.contains("brownfield"));
    assert!(text.contains("read-only"));
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
package = "root"

[roles.store]
kind = "canister"
package = "store"

[auth.delegated_tokens]
enabled = false

[subnets.prime.canisters.root]
kind = "root"
"#
        ),
    )
    .expect("write canic config");
    dir
}
