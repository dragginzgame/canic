use super::*;
use crate::test_support::TempDir;

// Ensure scaffold options parse the App name.
#[test]
fn parses_scaffold_options() {
    let options =
        ScaffoldOptions::parse([OsString::from("my_app")]).expect("parse scaffold options");

    assert_eq!(options.name, "my_app");
    assert!(!options.yes);
    assert!(!options.dry_run);
}

// Ensure scaffold accepts explicit noninteractive confirmation.
#[test]
fn parses_scaffold_yes_option() {
    let options = ScaffoldOptions::parse([OsString::from("my_app"), OsString::from("--yes")])
        .expect("parse scaffold yes option");

    assert!(options.yes);
}

// Ensure scaffold accepts source-write preview mode.
#[test]
fn parses_scaffold_dry_run_option() {
    let options = ScaffoldOptions::parse([OsString::from("my_app"), OsString::from("--dry-run")])
        .expect("parse scaffold dry-run option");

    assert!(options.dry_run);
}

// Ensure canister scaffold options parse app and role identity.
#[test]
fn parses_canister_scaffold_options() {
    let options = CanisterScaffoldOptions::parse([OsString::from("demo"), OsString::from("store")])
        .expect("parse canister scaffold options");

    assert_eq!(options.app, "demo");
    assert_eq!(options.role, "store");
    assert!(!options.dry_run);
}

// Ensure canister scaffold accepts source/config preview mode.
#[test]
fn parses_canister_scaffold_dry_run_option() {
    let options = CanisterScaffoldOptions::parse([
        OsString::from("demo"),
        OsString::from("store"),
        OsString::from("--dry-run"),
    ])
    .expect("parse canister scaffold dry-run options");

    assert!(options.dry_run);
}

// Ensure Fleet-input scaffold options accept exact node-count evidence per role.
#[test]
fn parses_fleet_input_scaffold_options() {
    let options = FleetInputScaffoldOptions::parse([
        OsString::from("preview_multi_subnet"),
        OsString::from("--coordinator-node-count"),
        OsString::from("34"),
        OsString::from("--root-node-count"),
        OsString::from("13"),
        OsString::from("--root-node-count"),
        OsString::from("28"),
    ])
    .expect("parse Fleet-input scaffold options");

    assert_eq!(options.profile, FleetFundingProfile::PreviewMultiSubnet);
    assert_eq!(
        options.node_counts,
        FleetInputScaffoldNodeCounts::Explicit {
            coordinator: 34,
            roots: vec![13, 28],
        }
    );
}

// Ensure Registry-backed authoring captures exact Subnet IDs and refresh policy.
#[test]
fn parses_registry_fleet_input_scaffold_options() {
    let coordinator = "pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae";
    let root = "bkfrj-6k62g-dycql-7h53p-atvkj-zg4to-gaogh-netha-ptybj-ntsgw-rqe";
    let options = FleetInputScaffoldOptions::parse([
        OsString::from("preview_multi_subnet"),
        OsString::from("--coordinator-subnet"),
        OsString::from(coordinator),
        OsString::from("--root-subnet"),
        OsString::from(root),
        OsString::from("--refresh-catalog"),
        OsString::from("--__canic-environment"),
        OsString::from("ic"),
    ])
    .expect("parse Registry Fleet-input scaffold options");

    assert_eq!(options.profile, FleetFundingProfile::PreviewMultiSubnet);
    assert_eq!(
        options.node_counts,
        FleetInputScaffoldNodeCounts::Registry {
            environment: "ic".to_string(),
            coordinator_subnet: parse_subnet_id(coordinator).expect("Coordinator Subnet ID"),
            root_subnets: vec![parse_subnet_id(root).expect("Root Subnet ID")],
            refresh_catalog: true,
        }
    );
}

// Ensure the two node-count authority modes cannot be mixed.
#[test]
fn rejects_mixed_fleet_input_scaffold_node_count_authority() {
    let result = FleetInputScaffoldOptions::parse([
        OsString::from("preview_multi_subnet"),
        OsString::from("--coordinator-node-count"),
        OsString::from("34"),
        OsString::from("--root-subnet"),
        OsString::from("bkfrj-6k62g-dycql-7h53p-atvkj-zg4to-gaogh-netha-ptybj-ntsgw-rqe"),
    ]);

    std::assert_matches!(result, Err(ScaffoldCommandError::Usage(_)));
}

// Ensure the offline scaffold explains Toko's real values and emits exact parseable TOML.
#[test]
fn renders_fee_complete_toko_funding_profile_scaffold() {
    let scaffold =
        scaffold_fleet_funding_profile(FleetFundingProfile::PreviewMultiSubnet, 34, &[13])
            .expect("materialize Toko funding scaffold");
    let text = render_fleet_input_scaffold(&scaffold, "explicit_node_counts");

    assert!(text.contains("funded_identity_required: false"));
    assert!(text.contains("node_count_source: explicit_node_counts"));
    assert!(text.contains(
        "coordinator.minimum_reserve_cycles = ceil_to_10000000000000(80000000000000 * max(34, 13) / 13) = 210000000000000 cycles"
    ));
    assert!(text.contains("operator_creation_amount_cycles: 310000000000000"));
    assert!(text.contains("operator_creation_fee_cycles: 300000000"));
    assert!(text.contains("maximum_operator_debit_cycles: 310000300000000"));
    assert!(text.contains("cycles = \"270000000000000\""));
    assert!(!text.contains("kind = \"icp\""));

    let toml = text
        .split_once("Exact funding TOML:\n")
        .expect("TOML heading")
        .1
        .split_once("\n\nNext:\n")
        .expect("TOML footer")
        .0;
    toml::from_str::<TomlValue>(toml).expect("exact funding TOML parses");
}

// Ensure confirmation accepts an explicit yes response.
#[test]
fn confirm_scaffold_accepts_yes() {
    let root = TempDir::new("canic-cli-scaffold-confirm-yes");
    let options = ScaffoldOptions {
        name: "my_app".to_string(),
        yes: false,
        dry_run: false,
    };
    let mut output = Vec::new();

    confirm_scaffold(&options, &root, io::Cursor::new(b"y\n"), &mut output)
        .expect("confirm scaffold");

    let output = String::from_utf8(output).expect("utf8 prompt");
    assert!(output.contains("target:"));
    assert!(output.contains("apps/my_app"));
    assert!(output.contains("install: canic install my_app <fleet>"));
}

// Ensure confirmation defaults to no on empty input.
#[test]
fn confirm_scaffold_rejects_empty_response() {
    let root = TempDir::new("canic-cli-scaffold-confirm-no");
    let options = ScaffoldOptions {
        name: "my_app".to_string(),
        yes: false,
        dry_run: false,
    };
    let mut output = Vec::new();

    let err = confirm_scaffold(&options, &root, io::Cursor::new(b"\n"), &mut output)
        .expect_err("empty response should cancel");

    std::assert_matches!(err, ScaffoldCommandError::Cancelled);
}

// Ensure invalid scaffold names are rejected before filesystem writes.
#[test]
fn rejects_invalid_app_names() {
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
fn scaffold_app_writes_root_and_app_files() {
    let root = TempDir::new("canic-cli-scaffold");
    let options = ScaffoldOptions {
        name: "my_app".to_string(),
        yes: true,
        dry_run: false,
    };

    let result = scaffold_app_at(&root, &options).expect("scaffold app");
    let config = fs::read_to_string(&result.config_path).expect("read config");
    let root_lib = fs::read_to_string(result.root_dir.join("src/lib.rs")).expect("read root lib");
    let root_manifest =
        fs::read_to_string(result.root_dir.join("Cargo.toml")).expect("read root manifest");
    let app_lib = fs::read_to_string(result.app_dir.join("src/lib.rs")).expect("read app lib");
    let app_manifest =
        fs::read_to_string(result.app_dir.join("Cargo.toml")).expect("read app manifest");

    assert!(config.contains("[app]"));
    assert!(config.contains("name = \"my_app\""));
    assert!(config.contains("[auth.delegated_tokens]"));
    assert!(config.contains("enabled = false"));
    assert!(config.contains("[roles.root]"));
    assert!(config.contains("[roles.app]"));
    assert!(!config.contains("auto_create"));
    assert!(config.contains("[component_specs.app]"));
    assert!(config.contains("component_role = \"app\""));
    assert!(config.contains("maximum_instances = 1"));
    assert!(!config.contains("topup_policy"));
    assert!(!config.contains("[[canisters]]"));
    assert!(root_manifest.contains("version = \"0.1.0\""));
    assert!(root_manifest.contains("app = \"my_app\""));
    assert!(root_manifest.contains("role = \"root\""));
    assert!(root_manifest.contains("Add runtime Canic features here"));
    assert!(root_manifest.contains("canic = \""));
    assert!(root_manifest.contains("ic-cdk = \"0.20\""));
    assert!(!root_manifest.contains("workspace = true"));
    assert!(root_lib.contains("canic::start!();"));
    assert!(root_lib.contains("canic::finish!();"));
    assert!(app_manifest.contains("name = \"canister_my_app_app\""));
    assert!(app_manifest.contains("app = \"my_app\""));
    assert!(app_manifest.contains("role = \"app\""));
    assert!(app_manifest.contains("Add runtime Canic features here"));
    assert!(app_manifest.contains("canic = \""));
    assert!(app_manifest.contains("ic-cdk = \"0.20\""));
    assert!(!app_manifest.contains("workspace = true"));
    assert!(!app_lib.contains("CanisterRole::new"));
    assert!(app_lib.contains("canic::start!();"));
    assert!(app_lib.contains("canic::finish!();"));
}

// Ensure app scaffold dry-run plans files without creating them.
#[test]
fn scaffold_app_plan_does_not_write_files() {
    let root = TempDir::new("canic-cli-scaffold-plan");
    let options = ScaffoldOptions {
        name: "my_app".to_string(),
        yes: false,
        dry_run: true,
    };

    let plan = plan_scaffold_app_at(&root, &options).expect("plan scaffold");
    let text = render_scaffold_app_plan(&plan);

    assert!(text.contains("Planned Canic app scaffold:"));
    assert!(text.contains("dry_run: true"));
    assert!(text.contains("files_changed: 0"));
    assert!(text.contains("canic.toml"));
    assert!(!plan.result.app_root.exists());
}

// Ensure canister scaffold writes a declared-only canister role under one app.
#[test]
fn scaffold_canister_writes_declared_only_role_files() {
    let root = TempDir::new("canic-cli-scaffold-canister");
    let app_dir = root.join("apps/demo");
    fs::create_dir_all(&app_dir).expect("create app dir");
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\n    \"apps/demo/root\",\n]\n",
    )
    .expect("write workspace manifest");
    fs::write(app_dir.join("canic.toml"), canic_toml("demo")).expect("write config");
    let options = CanisterScaffoldOptions {
        app: "demo".to_string(),
        role: "store".to_string(),
        dry_run: false,
    };

    let result = scaffold_canister_at(&root, &options).expect("scaffold canister");
    let config = fs::read_to_string(app_dir.join("canic.toml")).expect("read config");
    let workspace_manifest = fs::read_to_string(root.join("Cargo.toml")).expect("read workspace");
    let manifest = fs::read_to_string(app_dir.join("store/Cargo.toml")).expect("read manifest");
    let build_rs = fs::read_to_string(app_dir.join("store/build.rs")).expect("read build");
    let lib = fs::read_to_string(app_dir.join("store/src/lib.rs")).expect("read lib");

    assert_eq!(result.app, "demo");
    assert_eq!(result.role, "store");
    assert_eq!(result.package, "store");
    assert_eq!(result.package_name, "canister_demo_store");
    assert_eq!(result.canister_dir, PathBuf::from("apps/demo/store"));
    assert_eq!(result.config_path, PathBuf::from("apps/demo/canic.toml"));
    assert!(config.contains("[roles.\"store\"]"));
    assert!(config.contains("package = \"store\""));
    assert!(!config.contains("[component_specs.store]"));
    assert!(workspace_manifest.contains("\"apps/demo/store\""));
    assert!(manifest.contains("name = \"canister_demo_store\""));
    assert!(manifest.contains("app = \"demo\""));
    assert!(manifest.contains("role = \"store\""));
    assert!(manifest.contains("crate-type = [\"cdylib\"]"));
    assert!(manifest.contains("Add runtime Canic features here"));
    assert!(build_rs.contains("canic::build!(\"../canic.toml\")"));
    assert!(lib.contains("canic::start!();"));
    assert!(lib.contains("pub async fn canic_install(_: Option<Vec<u8>>) {}"));
    assert!(lib.contains("canic::finish!();"));
}

// Ensure canister scaffold dry-run validates all intended writes without changing files.
#[test]
fn scaffold_canister_plan_does_not_write_files() {
    let root = TempDir::new("canic-cli-scaffold-canister-plan");
    let app_dir = root.join("apps/demo");
    fs::create_dir_all(&app_dir).expect("create app dir");
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\n    \"apps/demo/root\",\n]\n",
    )
    .expect("write workspace manifest");
    fs::write(app_dir.join("canic.toml"), canic_toml("demo")).expect("write config");
    let before_config = fs::read_to_string(app_dir.join("canic.toml")).expect("read config");
    let before_workspace = fs::read_to_string(root.join("Cargo.toml")).expect("read workspace");
    let options = CanisterScaffoldOptions {
        app: "demo".to_string(),
        role: "store".to_string(),
        dry_run: true,
    };

    let plan = plan_scaffold_canister_at(&root, &options).expect("plan canister scaffold");
    let text = render_canister_scaffold_plan(&plan);
    let after_config = fs::read_to_string(app_dir.join("canic.toml")).expect("read config");
    let after_workspace = fs::read_to_string(root.join("Cargo.toml")).expect("read workspace");

    assert_eq!(after_config, before_config);
    assert_eq!(after_workspace, before_workspace);
    assert!(!app_dir.join("store").exists());
    assert_eq!(plan.canister_dir, app_dir.join("store"));
    assert_eq!(plan.config_path, app_dir.join("canic.toml"));
    assert_eq!(plan.result.canister_dir, PathBuf::from("apps/demo/store"));
    assert_eq!(
        plan.result.config_path,
        PathBuf::from("apps/demo/canic.toml")
    );
    assert!(text.contains("Planned Canic canister role scaffold:"));
    assert!(text.contains("role: demo.store"));
    assert!(text.contains("workspace_member: apps/demo/store"));
    assert!(text.contains("files_changed: 0"));
}

// Ensure workspace member insertion handles compact workspace arrays.
#[test]
fn append_workspace_member_source_updates_compact_members_array() {
    let updated = append_workspace_member_source(
        "[workspace]\nmembers = [\"apps/demo/root\"]\n",
        "apps/demo/store",
    )
    .expect("append member");

    assert!(updated.contains("\"apps/demo/root\""));
    assert!(updated.contains("\"apps/demo/store\""));
}

// Ensure workspace appends only treat real [workspace].members entries as present.
#[test]
fn append_workspace_member_source_does_not_skip_unrelated_string_matches() {
    let updated = append_workspace_member_source(
        "[package]\ndescription = \"apps/demo/store\"\n\n[workspace]\nmembers = [\"apps/demo/root\"]\n",
        "apps/demo/store",
    )
    .expect("append member");

    let manifest = toml::from_str::<TomlValue>(&updated).expect("parse updated manifest");
    let members = manifest
        .get("workspace")
        .and_then(TomlValue::as_table)
        .and_then(|workspace| workspace.get("members"))
        .and_then(TomlValue::as_array)
        .expect("workspace members");

    assert_eq!(
        members
            .iter()
            .filter_map(TomlValue::as_str)
            .filter(|member| *member == "apps/demo/store")
            .count(),
        1
    );
}

// Ensure workspace appends fail closed when members is not a string array.
#[test]
fn append_workspace_member_source_rejects_non_array_members() {
    let err = append_workspace_member_source(
        "[workspace]\nmembers = \"apps/demo/root\"\n",
        "apps/demo/store",
    )
    .expect_err("non-array members should fail");

    std::assert_matches!(err, ScaffoldCommandError::Usage(_));
}

// Ensure canister scaffold refuses to overwrite an existing role crate.
#[test]
fn scaffold_canister_rejects_existing_target() {
    let root = TempDir::new("canic-cli-scaffold-canister-existing");
    let app_dir = root.join("apps/demo");
    fs::create_dir_all(app_dir.join("store")).expect("create existing canister dir");
    fs::write(app_dir.join("canic.toml"), canic_toml("demo")).expect("write config");
    let options = CanisterScaffoldOptions {
        app: "demo".to_string(),
        role: "store".to_string(),
        dry_run: false,
    };

    let err = scaffold_canister_at(&root, &options).expect_err("existing scaffold should fail");

    std::assert_matches!(err, ScaffoldCommandError::TargetExists(_));
}

// Ensure canister scaffold rejects an existing declaration before writing files.
#[test]
fn scaffold_canister_rejects_existing_declaration_without_writing_files() {
    let root = TempDir::new("canic-cli-scaffold-canister-declared");
    let app_dir = root.join("apps/demo");
    fs::create_dir_all(&app_dir).expect("create app dir");
    fs::write(app_dir.join("canic.toml"), canic_toml("demo")).expect("write config");
    let options = CanisterScaffoldOptions {
        app: "demo".to_string(),
        role: "app".to_string(),
        dry_run: false,
    };

    let err = scaffold_canister_at(&root, &options).expect_err("declared role should fail");

    std::assert_matches!(err, ScaffoldCommandError::Usage(_));
    assert!(!app_dir.join("app").exists());
}

// Ensure rollback restores every existing document and removes only the new scaffold root.
#[test]
fn scaffold_rollback_restores_documents_and_removes_new_directory() {
    let root = TempDir::new("canic-cli-scaffold-rollback");
    let workspace = root.join("Cargo.toml");
    let config = root.join("apps/demo/canic.toml");
    let created_dir = root.join("apps/demo/store");
    let workspace_before = b"[workspace]\nmembers = []\n".to_vec();
    let config_before = b"[app]\nname = \"demo\"\n".to_vec();
    fs::create_dir_all(&created_dir).expect("create partial scaffold");
    fs::write(created_dir.join("Cargo.toml"), "partial").expect("write partial scaffold");
    fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
    fs::write(&workspace, "changed workspace").expect("write changed workspace");
    fs::write(&config, "changed config").expect("write changed config");

    rollback_scaffold(
        &created_dir,
        &[
            (workspace.clone(), workspace_before.clone()),
            (config.clone(), config_before.clone()),
        ],
    )
    .expect("rollback scaffold");

    assert!(!created_dir.exists());
    assert_eq!(
        fs::read(workspace).expect("read workspace"),
        workspace_before
    );
    assert_eq!(fs::read(config).expect("read config"), config_before);
}

// Ensure rollback still removes the new directory when restoring a document fails.
#[test]
fn scaffold_rollback_attempts_cleanup_after_restore_failure() {
    let root = TempDir::new("canic-cli-scaffold-rollback-failure");
    let created_dir = root.join("apps/demo/store");
    let invalid_restore_target = root.join("existing-directory");
    fs::create_dir_all(&created_dir).expect("create partial scaffold");
    fs::create_dir_all(&invalid_restore_target).expect("create invalid restore target");

    let error = rollback_scaffold(
        &created_dir,
        &[(invalid_restore_target.clone(), b"original".to_vec())],
    )
    .expect_err("restore over directory should fail");

    assert!(matches!(
        error.kind(),
        io::ErrorKind::IsADirectory
            | io::ErrorKind::PermissionDenied
            | io::ErrorKind::AlreadyExists
    ));
    assert!(invalid_restore_target.is_dir());
    assert!(!created_dir.exists());
}

// Ensure canister scaffold help exposes the declared-only workflow.
#[test]
fn scaffold_canister_usage_lists_app_and_role() {
    let text = scaffold_canister_usage();

    assert!(text.contains("Create a declared-only canister role"));
    assert!(text.contains("Usage: canic scaffold canister"));
    assert!(text.contains("<app>"));
    assert!(text.contains("<role>"));
    assert!(text.contains("--dry-run"));
    assert!(text.contains("Examples:"));
}

// Ensure scaffold family help explains local write boundaries.
#[test]
fn scaffold_usage_lists_mutation_notes() {
    let text = usage();

    assert!(text.contains("Mutation notes:"));
    assert!(text.contains("writes a new local role crate"));
    assert!(text.contains("appends the workspace"));
    assert!(text.contains("Use --dry-run"));
}

// Ensure scaffold refuses to overwrite an existing App directory.
#[test]
fn scaffold_app_rejects_existing_target() {
    let root = TempDir::new("canic-cli-scaffold-existing");
    let options = ScaffoldOptions {
        name: "my_app".to_string(),
        yes: true,
        dry_run: false,
    };
    fs::create_dir_all(root.join("apps/my_app")).expect("create existing target");

    let err = scaffold_app_at(&root, &options).expect_err("existing scaffold should fail");

    std::assert_matches!(err, ScaffoldCommandError::TargetExists(_));
}
