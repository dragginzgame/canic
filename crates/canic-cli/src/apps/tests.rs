use super::{
    render::{AppListRow, render_app_rows},
    *,
};
use crate::test_support::TempDir;
use canic_host::release_set::{
    AttachedAppRole, ConfiguredRoleLifecycle, DeclaredAppRole, RenamedAppRole,
};
use std::fs;

#[test]
fn parses_app_options() {
    let options = AppOptions::parse([
        OsString::from(crate::cli::globals::INTERNAL_ENVIRONMENT_OPTION),
        OsString::from("ic"),
    ])
    .expect("parse app options");

    assert_eq!(options.environment, "ic");
}

#[test]
fn parses_delete_app_options() {
    let options = DeleteAppOptions::parse([OsString::from("demo")]).expect("parse delete options");

    assert_eq!(options.app, "demo");
    assert!(!options.dry_run);
}

#[test]
fn parses_delete_app_dry_run_option() {
    let options = DeleteAppOptions::parse([OsString::from("demo"), OsString::from("--dry-run")])
        .expect("parse delete dry-run options");

    assert_eq!(options.app, "demo");
    assert!(options.dry_run);
}

#[test]
fn parses_check_app() {
    let options = AppCheckOptions::parse([OsString::from("test")]).expect("parse check options");

    assert_eq!(options.app, "test");
}

#[test]
fn app_create_dispatch_preserves_scaffold_error() {
    let error = run_create(std::iter::empty::<OsString>())
        .expect_err("missing app create arguments reject");

    std::assert_matches!(
        error,
        AppCommandError::Create(scaffold::ScaffoldCommandError::Usage(_))
    );
}

#[test]
fn app_config_dispatch_preserves_list_error() {
    let error = run_config(std::iter::empty::<OsString>())
        .expect_err("missing app config arguments reject");

    let AppCommandError::Config(error) = error else {
        panic!("expected config error");
    };
    std::assert_matches!(*error, crate::list::ListCommandError::Usage(_));
}

#[test]
fn parses_role_list_app() {
    let options =
        RoleListOptions::parse([OsString::from("demo")]).expect("parse role list options");

    assert_eq!(options.app, "demo");
}

#[test]
fn parses_role_inspect_app_and_role() {
    let options = RoleInspectOptions::parse([OsString::from("demo"), OsString::from("app")])
        .expect("parse role inspect options");

    assert_eq!(options.app, "demo");
    assert_eq!(options.role, "app");
}

#[test]
fn parses_role_declare_app_role_and_package() {
    let options = RoleDeclareOptions::parse([
        OsString::from("demo"),
        OsString::from("store"),
        OsString::from("--package"),
        OsString::from("store"),
    ])
    .expect("parse role declare options");

    assert_eq!(options.app, "demo");
    assert_eq!(options.role, "store");
    assert_eq!(options.package, "store");
    assert!(!options.dry_run);
}

#[test]
fn parses_role_declare_dry_run_option() {
    let options = RoleDeclareOptions::parse([
        OsString::from("demo"),
        OsString::from("store"),
        OsString::from("--package"),
        OsString::from("store"),
        OsString::from("--dry-run"),
    ])
    .expect("parse role declare dry-run options");

    assert!(options.dry_run);
}

#[test]
fn parses_role_attach_app_role_and_component_spec() {
    let options = RoleAttachOptions::parse([
        OsString::from("demo"),
        OsString::from("store"),
        OsString::from("--component-spec"),
        OsString::from("default"),
    ])
    .expect("parse role attach options");

    assert_eq!(options.app, "demo");
    assert_eq!(options.role, "store");
    assert_eq!(options.component_spec, "default");
    assert_eq!(options.kind, "singleton");
    assert!(!options.dry_run);
}

#[test]
fn parses_role_attach_kind() {
    let options = RoleAttachOptions::parse([
        OsString::from("demo"),
        OsString::from("worker"),
        OsString::from("--component-spec"),
        OsString::from("default"),
        OsString::from("--kind"),
        OsString::from("replica"),
    ])
    .expect("parse role attach options");

    assert_eq!(options.kind, "replica");
}

#[test]
fn parses_role_attach_dry_run_option() {
    let options = RoleAttachOptions::parse([
        OsString::from("demo"),
        OsString::from("store"),
        OsString::from("--component-spec"),
        OsString::from("default"),
        OsString::from("--dry-run"),
    ])
    .expect("parse role attach dry-run options");

    assert!(options.dry_run);
}

#[test]
fn parses_role_rename_app_old_role_and_new_role() {
    let options = RoleRenameOptions::parse([
        OsString::from("demo"),
        OsString::from("hub"),
        OsString::from("router"),
    ])
    .expect("parse role rename options");

    assert_eq!(options.app, "demo");
    assert_eq!(options.old_role, "hub");
    assert_eq!(options.new_role, "router");
    assert!(!options.dry_run);
}

#[test]
fn parses_role_rename_dry_run_option() {
    let options = RoleRenameOptions::parse([
        OsString::from("demo"),
        OsString::from("hub"),
        OsString::from("router"),
        OsString::from("--dry-run"),
    ])
    .expect("parse role rename dry-run options");

    assert!(options.dry_run);
}

#[test]
fn rejects_unknown_check_option() {
    let error =
        AppCheckOptions::parse([OsString::from("--unknown")]).expect_err("parse should fail");

    std::assert_matches!(error, AppCommandError::Usage(_));
}

#[test]
fn confirm_delete_app_requires_exact_name() {
    let target = Path::new("/tmp/canic/apps/demo");
    let mut output = Vec::new();

    confirm_delete_app("demo", target, io::Cursor::new(b"demo\n"), &mut output)
        .expect("confirm delete");

    let output = String::from_utf8(output).expect("utf8 prompt");
    assert!(output.contains("Delete Canic app?"));
    assert!(output.contains("app: demo"));
    assert!(output.contains("Type the app name to confirm"));

    let error = confirm_delete_app("demo", target, io::Cursor::new(b"yes\n"), Vec::new())
        .expect_err("wrong confirmation should cancel");
    std::assert_matches!(error, AppCommandError::DeleteCancelled);
}

#[test]
fn delete_target_resolves_config_parent() {
    let root = TempDir::new("canic-app-delete-target");
    let demo = write_app_config(&root, "demo");
    let staging = write_app_config(&root, "staging");
    let choices = vec![demo.join("canic.toml"), staging.join("canic.toml")];

    let target = delete_target_dir_from_choices(&root, &choices, "staging").expect("delete target");

    assert_eq!(target, staging);
}

#[test]
fn renders_app_list_table() {
    let table = render_app_rows(vec![
        AppListRow {
            app: "demo".to_string(),
            environment: "local".to_string(),
            config: "apps/demo/canic.toml".to_string(),
            canisters: "4 (root, app, user_hub, user_shard)".to_string(),
        },
        AppListRow {
            app: "staging".to_string(),
            environment: "local".to_string(),
            config: "apps/staging/canic.toml".to_string(),
            canisters: "2 (root, app)".to_string(),
        },
    ]);

    assert!(table.contains("APP"));
    assert!(table.contains("apps/demo/canic.toml"));
    assert!(table.contains("apps/staging/canic.toml"));
}

#[test]
fn renders_role_lifecycle_table() {
    let table = render_role_lifecycle_rows(&[
        ConfiguredRoleLifecycle {
            app: "demo".to_string(),
            role: "root".to_string(),
            display: "demo.root".to_string(),
            declaration_kind: "root".to_string(),
            package: "canisters/root".to_string(),
            attached: true,
            state: "attached".to_string(),
            topology: Some("default/root".to_string()),
        },
        ConfiguredRoleLifecycle {
            app: "demo".to_string(),
            role: "store".to_string(),
            display: "demo.store".to_string(),
            declaration_kind: "canister".to_string(),
            package: "canisters/store".to_string(),
            attached: false,
            state: "declared".to_string(),
            topology: None,
        },
    ]);

    assert!(table.contains("demo.root"));
    assert!(table.contains("default/root"));
    assert!(table.contains("demo.store"));
    assert!(table.contains("declared"));
}

#[test]
fn renders_declared_only_role_inspection() {
    let output = render_role_inspection(&ConfiguredRoleLifecycle {
        app: "demo".to_string(),
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
    assert!(output.contains("canic app role attach demo store --component-spec <component-spec>"));
}

#[test]
fn renders_role_mutation_outputs() {
    let root = Path::new("/workspace");
    let config = root.join("apps/demo/canic.toml");
    let declared = DeclaredAppRole {
        app: "demo".to_string(),
        role: "store".to_string(),
        display: "demo.store".to_string(),
        package: "store".to_string(),
    };
    let attached = AttachedAppRole {
        app: "demo".to_string(),
        role: "store".to_string(),
        display: "demo.store".to_string(),
        component_spec: "default".to_string(),
        kind: "singleton".to_string(),
        topology: "default/store".to_string(),
    };
    let renamed = RenamedAppRole {
        app: "demo".to_string(),
        old_role: "hub".to_string(),
        new_role: "router".to_string(),
        old_display: "demo.hub".to_string(),
        new_display: "demo.router".to_string(),
        package_manifest: Some(root.join("apps/demo/router/Cargo.toml")),
        package_manifest_note: None,
    };

    assert!(render_declared_role(&declared, root, &config).contains("state: declared"));
    assert!(render_planned_declared_role(&declared, root, &config).contains("files_changed: 0"));
    assert!(render_attached_role(&attached, root, &config).contains("state: attached"));
    assert!(render_planned_attached_role(&attached, root, &config).contains("files_changed: 0"));
    assert!(render_renamed_role(&renamed, root, &config).contains("new: demo.router"));
    assert!(render_planned_renamed_role(&renamed, root, &config).contains("files_changed: 0"));
}

#[test]
fn renders_planned_delete_output() {
    let root = Path::new("/workspace");
    let target = root.join("apps/demo");
    let output = render_planned_delete(root, "demo", &target);

    assert!(output.contains("Planned app delete:"));
    assert!(output.contains("app: demo"));
    assert!(output.contains("would_remove: apps/demo"));
    assert!(output.contains("files_changed: 0"));
}

#[test]
fn app_usage_lists_only_current_subcommands() {
    let text = usage();

    assert!(text.contains("Usage: canic app"));
    for command in ["check", "config", "create", "delete", "list", "role"] {
        assert!(text.contains(command));
    }
    assert!(!text.contains("adoption"));
    assert!(!text.contains("sync"));
    assert_eq!(text.matches("  canic app ").count(), 2);
}

#[test]
fn app_role_usage_lists_current_subcommands() {
    let text = role_usage();

    for command in ["attach", "declare", "inspect", "list", "rename"] {
        assert!(text.contains(command));
    }
    assert!(text.contains("Inspect and list are read-only"));
    assert_eq!(text.matches("  canic app role ").count(), 2);
}

#[test]
fn leaf_usage_covers_current_arguments() {
    assert!(check_usage().contains("Usage: canic app check <name>"));
    assert!(create_usage().contains("Create a minimal Canic app"));
    assert!(list_usage().contains("Usage: canic app list"));

    let delete = delete_usage();
    assert!(delete.contains("Usage: canic app delete"));
    assert!(delete.contains("--dry-run"));

    assert!(role_list_usage().contains("Usage: canic app role list <app>"));
    assert!(role_inspect_usage().contains("Usage: canic app role inspect <app> <role>"));
    assert!(role_declare_usage().contains("--package <path>"));
    assert!(role_attach_usage().contains("--component-spec <component-spec>"));
    assert!(role_rename_usage().contains("<new-role>"));
}

fn write_app_config(root: &Path, name: &str) -> PathBuf {
    let dir = root.join("apps").join(name);
    fs::create_dir_all(dir.join("root")).expect("create root dir");
    fs::write(dir.join("root/Cargo.toml"), "").expect("write root manifest");
    fs::write(
        dir.join("canic.toml"),
        format!(
            r#"
[app]
name = "{name}"

[roles.root]
kind = "root"
package = "root"

[roles.store]
kind = "canister"
package = "store"

[auth.delegated_tokens]
enabled = false
"#
        ),
    )
    .expect("write canic config");
    dir
}
