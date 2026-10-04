use super::*;
use canic_core::bootstrap::{ConfigError, ConfigTomlIssue};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const CONFIG: &str = r#"
[app]
name = "demo"

[roles.root]
kind = "root"

[roles.store]
kind = "canister"
package = "store"


"#;

#[test]
fn role_rename_updates_directory_and_manifest_package_selectors() {
    let root = temp_root("rename-package-selectors");
    let package_root = root.join("store");
    fs::create_dir_all(&package_root).expect("create package root");
    let config_path = root.join("canic.toml");
    let manifest_path = package_root.join("Cargo.toml");
    let original_manifest = r#"[package]
name = "store"
version = "0.1.0"

[package.metadata.canic]
app = "demo"
role = "store"
"#;
    for package in [
        "store".to_string(),
        "store/Cargo.toml".to_string(),
        package_root.to_str().unwrap().to_string(),
        manifest_path.to_str().unwrap().to_string(),
    ] {
        let quoted_package = toml::Value::String(package).to_string();
        let source = CONFIG.replace(
            "package = \"store\"",
            &format!("package = {quoted_package}"),
        );
        fs::write(&config_path, &source).expect("write config");
        fs::write(&manifest_path, original_manifest).expect("write package");

        let renamed = rename_app_role(&config_path, "demo", "store", "frontend")
            .expect("rename role and package metadata");
        assert_eq!(renamed.package_manifest.as_ref(), Some(&manifest_path));
        assert_eq!(renamed.package_manifest_note, None);
        let config = parse_config_model(&fs::read_to_string(&config_path).unwrap()).unwrap();
        assert!(config.declares_role(&canic_core::ids::CanisterRole::new("frontend")));
        assert!(!config.declares_role(&canic_core::ids::CanisterRole::new("store")));
        let manifest: toml::Value =
            toml::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        assert_eq!(
            manifest["package"]["metadata"]["canic"]["app"].as_str(),
            Some("demo")
        );
        assert_eq!(
            manifest["package"]["metadata"]["canic"]["role"].as_str(),
            Some("frontend")
        );
    }
    fs::remove_dir_all(root).expect("clean package selector fixture");
}

#[test]
fn role_rename_rejects_unreadable_or_malformed_package_before_writing_config() {
    let root = temp_root("rename-invalid-package");
    let package_root = root.join("store");
    fs::create_dir_all(&package_root).expect("create package root");
    let config_path = root.join("canic.toml");
    let manifest_path = package_root.join("Cargo.toml");
    fs::write(&config_path, CONFIG).expect("write config");
    let missing = rename_app_role(&config_path, "demo", "store", "frontend")
        .expect_err("missing declared package must fail");
    assert_io_error(
        &missing,
        AppConfigIoOperation::ReadPackageManifest,
        &manifest_path,
        io::ErrorKind::NotFound,
    );
    assert_eq!(fs::read_to_string(&config_path).unwrap(), CONFIG);

    fs::write(&manifest_path, "[package").expect("write malformed package");
    let malformed = rename_app_role(&config_path, "demo", "store", "frontend")
        .expect_err("malformed declared package must fail");
    assert!(matches!(
        malformed,
        AppConfigError::ConfigInvalid { path, source }
            if path == manifest_path && matches!(*source, AppConfigError::Toml {
                operation: AppConfigTomlOperation::ParsePackageManifest, ..
            })
    ));
    assert_eq!(fs::read_to_string(&config_path).unwrap(), CONFIG);
    assert_eq!(fs::read_to_string(&manifest_path).unwrap(), "[package");
    fs::remove_dir_all(root).expect("clean invalid package fixture");
}

#[test]
fn failed_package_manifest_write_restores_original_config() {
    let root = temp_root("rename-rollback");
    fs::create_dir_all(&root).expect("create temp root");
    let config_path = root.join("canic.toml");
    let invalid_package_target = root.join("Cargo.toml");
    fs::write(&config_path, "original config").expect("write original config");
    fs::create_dir(&invalid_package_target).expect("create invalid package target directory");

    let error = commit_role_rename_sources(
        &config_path,
        "original config",
        "updated config",
        Some((&invalid_package_target, "updated package")),
    )
    .expect_err("package write must fail");

    assert_io_error(
        &error,
        AppConfigIoOperation::WritePackageManifest,
        &invalid_package_target,
        io::ErrorKind::IsADirectory,
    );

    assert_eq!(
        fs::read_to_string(&config_path).expect("read rolled back config"),
        "original config"
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn public_projection_preserves_config_path_and_core_parse_source() {
    let root = temp_root("typed-core-source");
    fs::create_dir_all(&root).expect("create temp root");
    let config_path = root.join("canic.toml");
    fs::write(&config_path, "[app").expect("write invalid config");

    let error = AppConfigSnapshot::load(&config_path).expect_err("invalid config must fail");
    match error {
        AppConfigError::ConfigInvalid { path, source } => {
            assert_eq!(path, config_path);
            assert!(matches!(
                *source,
                AppConfigError::CoreConfig {
                    operation: AppConfigOperation::Project,
                    source,
                }
                    if matches!(*source, ConfigError::CannotParseToml { .. })
            ));
        }
        other => panic!("expected typed config parse error, got {other:?}"),
    }

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn public_projection_preserves_typed_nested_unknown_field() {
    let root = temp_root("typed-unknown-field");
    fs::create_dir_all(&root).expect("create temp root");
    let config_path = root.join("canic.toml");
    let source = format!(
        "{CONFIG}\n\
         [component_specs.default]\n\
         component_role = \"store\"\n\
         maximum_instances = 1\n\n\
         [component_specs.default.randomness]\n\
         enabled = true\n"
    );
    fs::write(&config_path, source).expect("write invalid config");

    let error = AppConfigSnapshot::load(&config_path).expect_err("unknown field must fail");
    let AppConfigError::ConfigInvalid { path, source } = error else {
        panic!("expected config-path boundary");
    };
    assert_eq!(path, config_path);
    let AppConfigError::CoreConfig { operation, source } = *source else {
        panic!("expected core-config boundary");
    };
    assert_eq!(operation, AppConfigOperation::Project);
    let ConfigError::CannotParseToml { issue, .. } = *source else {
        panic!("expected TOML parse boundary");
    };
    assert_eq!(
        issue,
        ConfigTomlIssue::UnknownField {
            logical_path: "component_specs.default.randomness".to_string(),
            unknown_field: "randomness".to_string(),
        }
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn public_projection_preserves_read_operation_path_and_io_source() {
    let config_path = temp_root("missing-config").join("canic.toml");

    let error = AppConfigSnapshot::load(&config_path).expect_err("missing config must fail");

    assert_io_error(
        &error,
        AppConfigIoOperation::ReadConfig,
        &config_path,
        io::ErrorKind::NotFound,
    );
}

#[test]
fn loaded_snapshot_keeps_one_validated_file_state_across_projections() {
    let root = temp_root("immutable-snapshot");
    fs::create_dir_all(&root).expect("create temp root");
    let config_path = root.join("canic.toml");
    fs::write(&config_path, CONFIG).expect("write initial config");
    let snapshot = AppConfigSnapshot::load(&config_path).expect("load config snapshot");

    fs::write(&config_path, CONFIG.replace("demo", "changed"))
        .expect("replace config after snapshot load");

    assert_eq!(snapshot.app_id(), "demo");
    assert_eq!(snapshot.deployable_roles(), vec!["root".to_string()]);
    assert_eq!(
        AppConfigSnapshot::load(&config_path)
            .expect("load replacement snapshot")
            .app_id(),
        "changed"
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn app_mutation_failures_are_classified_without_rendered_text() {
    assert!(matches!(
        declare_app_role_source(CONFIG, "demo", "bad role", "store")
            .expect_err("invalid role must fail"),
        AppConfigError::InvalidName {
            field: AppConfigNameField::Role,
            issue: AppConfigNameIssue::InvalidSnakeCase,
            ..
        }
    ));
    assert!(matches!(
        attach_app_role_source(CONFIG, "demo", "store", "default", "worker")
            .expect_err("invalid kind must fail"),
        AppConfigError::InvalidKind { .. }
    ));
    assert!(matches!(
        declare_app_role_source(CONFIG, "production", "new_role", "new_role")
            .expect_err("App mismatch must fail"),
        AppConfigError::AppMismatch { .. }
    ));
    assert!(matches!(
        attach_app_role_source(CONFIG, "demo", "missing", "default", "singleton")
            .expect_err("missing role must fail"),
        AppConfigError::DeclarationMissing {
            declaration: AppConfigDeclaration::Role { .. }
        }
    ));
    assert!(matches!(
        declare_app_role_source(CONFIG, "demo", "store", "store")
            .expect_err("duplicate role must fail"),
        AppConfigError::MutationConflict {
            conflict: AppConfigMutationConflict::RoleAlreadyDeclared { .. }
        }
    ));
}

#[test]
fn app_mutations_use_canonical_canister_role_admission() {
    let declare_error = declare_app_role_source(CONFIG, "demo", "user-hub", "store")
        .expect_err("kebab-case declaration must fail");
    let attach_error = attach_app_role_source(CONFIG, "demo", "Store", "default", "service")
        .expect_err("mixed-case attachment must fail");
    let rename_error =
        rename_app_role_source(CONFIG, Path::new("canic.toml"), "demo", "store", "store_")
            .expect_err("trailing-underscore rename must fail");

    for error in [declare_error, attach_error, rename_error] {
        assert!(matches!(
            error,
            AppConfigError::InvalidName {
                field: AppConfigNameField::Role,
                issue: AppConfigNameIssue::InvalidSnakeCase,
                ..
            }
        ));
    }

    let long_role = "a".repeat(canic_core::bootstrap::compiled::NAME_MAX_BYTES + 1);
    assert!(matches!(
        declare_app_role_source(CONFIG, "demo", &long_role, "store")
            .expect_err("overlong declaration must fail"),
        AppConfigError::InvalidName {
            field: AppConfigNameField::Role,
            issue: AppConfigNameIssue::TooLong { max_bytes },
            ..
        } if max_bytes == canic_core::bootstrap::compiled::NAME_MAX_BYTES
    ));

    declare_app_role_source(CONFIG, "demo", "new_role", "store")
        .expect("canonical role should be admitted");
}

#[test]
fn rollback_failure_preserves_mutation_and_rollback_sources() {
    let config_path = Path::new("canic.toml");
    let package_path = Path::new("store/Cargo.toml");
    let mut writes = 0;

    let error = commit_role_rename_sources_with_writer(
        config_path,
        "original config",
        "updated config",
        Some((package_path, "updated package")),
        |_, _| {
            writes += 1;
            match writes {
                1 => Ok(()),
                2 => Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "package write failed",
                )),
                3 => Err(io::Error::new(
                    io::ErrorKind::StorageFull,
                    "config rollback failed",
                )),
                _ => unreachable!("rename commit performs at most three writes"),
            }
        },
    )
    .expect_err("rollback failure must retain both causes");

    let AppConfigError::RollbackFailed { mutation, rollback } = error else {
        panic!("expected typed rollback failure");
    };
    assert_io_error(
        &mutation,
        AppConfigIoOperation::WritePackageManifest,
        package_path,
        io::ErrorKind::PermissionDenied,
    );
    assert_io_error(
        &rollback,
        AppConfigIoOperation::RestoreConfig,
        config_path,
        io::ErrorKind::StorageFull,
    );
}

fn assert_io_error(
    error: &AppConfigError,
    expected_operation: AppConfigIoOperation,
    expected_path: &Path,
    expected_kind: io::ErrorKind,
) {
    match error {
        AppConfigError::Io {
            operation,
            path,
            source,
        } => {
            assert_eq!(*operation, expected_operation);
            assert_eq!(path, expected_path);
            assert_eq!(source.kind(), expected_kind);
        }
        other => panic!("expected typed I/O error, got {other:?}"),
    }
}

fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "canic-host-release-config-{label}-{}-{nanos}",
        std::process::id()
    ))
}
