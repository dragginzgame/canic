use super::*;
use canic_core::bootstrap::{ConfigError, ConfigTomlIssue};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const CONFIG: &str = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.store]
kind = "canister"
package = "store"

[subnets.prime.canisters.root]
kind = "root"
"#;

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
        FleetConfigIoOperation::WritePackageManifest,
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
    fs::write(&config_path, "controllers = [").expect("write invalid config");

    let error = FleetConfigSnapshot::load(&config_path).expect_err("invalid config must fail");
    match error {
        FleetConfigError::ConfigInvalid { path, source } => {
            assert_eq!(path, config_path);
            assert!(matches!(
                *source,
                FleetConfigError::CoreConfig {
                    operation: FleetConfigOperation::Project,
                    source: ConfigError::CannotParseToml { .. },
                }
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
    let source = format!("{CONFIG}\n[subnets.prime.canisters.root.randomness]\nenabled = true\n");
    fs::write(&config_path, source).expect("write invalid config");

    let error = FleetConfigSnapshot::load(&config_path).expect_err("unknown field must fail");
    let FleetConfigError::ConfigInvalid { path, source } = error else {
        panic!("expected config-path boundary");
    };
    assert_eq!(path, config_path);
    let FleetConfigError::CoreConfig { operation, source } = *source else {
        panic!("expected core-config boundary");
    };
    assert_eq!(operation, FleetConfigOperation::Project);
    let ConfigError::CannotParseToml { issue, .. } = source else {
        panic!("expected TOML parse boundary");
    };
    assert_eq!(
        issue,
        ConfigTomlIssue::UnknownField {
            logical_path: "subnets.prime.canisters.root.randomness".to_string(),
            unknown_field: "randomness".to_string(),
        }
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn public_projection_preserves_read_operation_path_and_io_source() {
    let config_path = temp_root("missing-config").join("canic.toml");

    let error = FleetConfigSnapshot::load(&config_path).expect_err("missing config must fail");

    assert_io_error(
        &error,
        FleetConfigIoOperation::ReadConfig,
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
    let snapshot = FleetConfigSnapshot::load(&config_path).expect("load config snapshot");

    fs::write(&config_path, CONFIG.replace("demo", "changed"))
        .expect("replace config after snapshot load");

    assert_eq!(snapshot.fleet_name(), "demo");
    assert_eq!(snapshot.deployable_roles(), vec!["root".to_string()]);
    assert_eq!(
        FleetConfigSnapshot::load(&config_path)
            .expect("load replacement snapshot")
            .fleet_name(),
        "changed"
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn fleet_mutation_failures_are_classified_without_rendered_text() {
    assert!(matches!(
        declare_fleet_role_source(CONFIG, "demo", "bad role", "store")
            .expect_err("invalid role must fail"),
        FleetConfigError::InvalidName {
            field: FleetConfigNameField::Role,
            issue: FleetConfigNameIssue::InvalidSnakeCase,
            ..
        }
    ));
    assert!(matches!(
        attach_fleet_role_source(CONFIG, "demo", "store", "prime", "worker")
            .expect_err("invalid kind must fail"),
        FleetConfigError::InvalidKind { .. }
    ));
    assert!(matches!(
        declare_fleet_role_source(CONFIG, "production", "new_role", "new_role")
            .expect_err("fleet mismatch must fail"),
        FleetConfigError::FleetMismatch { .. }
    ));
    assert!(matches!(
        attach_fleet_role_source(CONFIG, "demo", "missing", "prime", "service")
            .expect_err("missing role must fail"),
        FleetConfigError::DeclarationMissing {
            declaration: FleetConfigDeclaration::Role { .. }
        }
    ));
    assert!(matches!(
        declare_fleet_role_source(CONFIG, "demo", "store", "store")
            .expect_err("duplicate role must fail"),
        FleetConfigError::MutationConflict {
            conflict: FleetConfigMutationConflict::RoleAlreadyDeclared { .. }
        }
    ));
}

#[test]
fn fleet_mutations_use_canonical_canister_role_admission() {
    let declare_error = declare_fleet_role_source(CONFIG, "demo", "user-hub", "store")
        .expect_err("kebab-case declaration must fail");
    let attach_error = attach_fleet_role_source(CONFIG, "demo", "Store", "prime", "service")
        .expect_err("mixed-case attachment must fail");
    let rename_error =
        rename_fleet_role_source(CONFIG, Path::new("canic.toml"), "demo", "store", "store_")
            .expect_err("trailing-underscore rename must fail");

    for error in [declare_error, attach_error, rename_error] {
        assert!(matches!(
            error,
            FleetConfigError::InvalidName {
                field: FleetConfigNameField::Role,
                issue: FleetConfigNameIssue::InvalidSnakeCase,
                ..
            }
        ));
    }

    let long_role = "a".repeat(canic_core::bootstrap::compiled::NAME_MAX_BYTES + 1);
    assert!(matches!(
        declare_fleet_role_source(CONFIG, "demo", &long_role, "store")
            .expect_err("overlong declaration must fail"),
        FleetConfigError::InvalidName {
            field: FleetConfigNameField::Role,
            issue: FleetConfigNameIssue::TooLong { max_bytes },
            ..
        } if max_bytes == canic_core::bootstrap::compiled::NAME_MAX_BYTES
    ));

    declare_fleet_role_source(CONFIG, "demo", "new_role", "store")
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

    let FleetConfigError::RollbackFailed { mutation, rollback } = error else {
        panic!("expected typed rollback failure");
    };
    assert_io_error(
        &mutation,
        FleetConfigIoOperation::WritePackageManifest,
        package_path,
        io::ErrorKind::PermissionDenied,
    );
    assert_io_error(
        &rollback,
        FleetConfigIoOperation::RestoreConfig,
        config_path,
        io::ErrorKind::StorageFull,
    );
}

fn assert_io_error(
    error: &FleetConfigError,
    expected_operation: FleetConfigIoOperation,
    expected_path: &Path,
    expected_kind: io::ErrorKind,
) {
    match error {
        FleetConfigError::Io {
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
