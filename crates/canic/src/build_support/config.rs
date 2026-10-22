use canic_contracts::ids::CanisterRole;
use canic_core::bootstrap::{
    compiled::{ConfigModel, validate_canister_role_name},
    emit_config_model_source, emit_role_runtime_authority_source,
};
use std::{error::Error, fmt, fs, io, path::Path};
use toml::Value as TomlValue;

/// Root-only build outputs retained by the full control-plane configuration owner.

#[derive(Clone, Debug)]
pub struct RootBuildSources {
    pub config_toml: String,
    pub config_model: String,
}

/// Exact generated source set for one canister role.
#[derive(Clone, Debug)]
pub struct RoleBuildSources {
    pub role_runtime_authority: String,
    pub root: Option<RootBuildSources>,
}

/// Failure while compiling the exact generated source set for one role.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleBuildSourcesError {
    detail: String,
}

impl fmt::Display for RoleBuildSourcesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for RoleBuildSourcesError {}

///
/// PackageCanicMetadata
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageCanicMetadata {
    pub app: String,
    pub role: String,
}

/// Compile only the generated sources owned by one exact role artifact.
pub fn compile_role_build_sources(
    config: &ConfigModel,
    role: &CanisterRole,
    wasm_store: bool,
) -> Result<RoleBuildSources, RoleBuildSourcesError> {
    let role_runtime_authority = emit_role_runtime_authority_source(config, role, wasm_store)
        .map_err(|error| RoleBuildSourcesError {
            detail: error.to_string(),
        })?;
    let root = if role.is_root() {
        Some(RootBuildSources {
            config_toml: toml::to_string(config).map_err(|error| RoleBuildSourcesError {
                detail: error.to_string(),
            })?,
            config_model: emit_config_model_source(config),
        })
    } else {
        None
    };

    Ok(RoleBuildSources {
        role_runtime_authority,
        root,
    })
}

/// Preserve unchanged generated source timestamps so output repair watches settle.
///
/// Missing or different source is recreated from the current configuration.
/// Linked output files reject before reading or writing another checkout's source.
pub fn write_build_source_if_changed(path: &Path, source: &str) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "generated build source is a symlink: {}; use an independent Cargo target/build directory",
                    path.display()
                ),
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    match fs::read(path) {
        Ok(existing) if existing == source.as_bytes() => return Ok(()),
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    fs::write(path, source)
}

/// Reject authoritative wasm builds that bypass host role-contract validation.
///
/// # Panics
///
/// Panics for the canonical canister wasm target when the private host marker
/// is absent.
pub fn assert_canonical_role_contract_build(target: &str, marker: Option<&str>) {
    assert!(
        target != "wasm32-unknown-unknown"
            || marker == Some(canic_core::role_contract::CANONICAL_BUILD_MARKER_VALUE),
        "authoritative Canic wasm builds must use `canic build <app>` or `canic build <app> <role>`; direct `cargo build --target wasm32-unknown-unknown` is unsupported"
    );
}

/// Read a Canic config source, or generate a minimal standalone config when allowed.
///
/// # Panics
///
/// Panics when the config file is missing and no default role is available,
/// when an explicitly requested config file is missing, or when reading an
/// existing config file fails.
#[must_use]
pub fn read_config_source_or_default(
    config_path: &Path,
    explicit_config: bool,
    default_role: Option<&str>,
) -> (String, bool) {
    match fs::read_to_string(config_path) {
        Ok(source) => (source, false),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let role = default_role
                .unwrap_or_else(|| panic!("Missing Canic config at {}", config_path.display()));

            assert!(
                !explicit_config,
                "Missing explicit Canic config at {}",
                config_path.display()
            );

            (standalone_config_source(role), true)
        }
        Err(err) => panic!("Failed to read {}: {err}", config_path.display()),
    }
}

/// Read optional Canic metadata declared in the package manifest.
#[must_use]
fn declared_package_metadata(manifest_dir: &Path) -> Option<PackageCanicMetadata> {
    let manifest = fs::read_to_string(manifest_dir.join("Cargo.toml")).ok()?;
    let canic = toml::from_str::<TomlValue>(&manifest)
        .ok()?
        .get("package")?
        .get("metadata")?
        .get("canic")?
        .clone();
    let app = canic.get("app")?.as_str()?.to_string();
    let role = canic.get("role")?.as_str()?.to_string();

    Some(PackageCanicMetadata { app, role })
}

/// Read the required Canic metadata declared in package manifest metadata.
///
/// # Panics
///
/// Panics when `Cargo.toml` does not declare `[package.metadata.canic]` with
/// both `app` and `role`.
#[must_use]
pub fn required_package_metadata(manifest_dir: &Path) -> PackageCanicMetadata {
    let manifest_path = manifest_dir.join("Cargo.toml");
    declared_package_metadata(manifest_dir).unwrap_or_else(|| {
        panic!(
            "missing Canic package metadata in {}; add [package.metadata.canic] app = \"<app>\" and role = \"<role>\"",
            manifest_path.display()
        )
    })
}

/// Return whether a validated config declares the requested App role.
#[must_use]
pub fn config_declares_role(config: &ConfigModel, app_id: &str, role_name: &str) -> bool {
    config.app_id().as_str() == app_id
        && config
            .roles
            .contains_key(&CanisterRole::owned(role_name.to_string()))
}

/// Return the App identity declared by a validated config.
#[must_use]
pub const fn config_app_id(config: &ConfigModel) -> &str {
    config.app_id().as_str()
}

/// Render the minimal declared-only config needed by a standalone non-root canister.
///
/// # Panics
///
/// Panics when `role` is not a canonical non-root canister role.
#[must_use]
fn standalone_config_source(role: &str) -> String {
    assert!(
        role != "root" && validate_canister_role_name(role).is_ok(),
        "standalone Canic config requires a canonical non-root role"
    );

    format!(
        r#"[app]
name = "standalone"
init_mode = "enabled"


[roles.{role}]
kind = "canister"
package = "."

[auth.delegated_tokens]
enabled = false
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::bootstrap::parse_config_model;

    const ROLE_BUILD_CONFIG: &str = r#"
[app]
name = "test"

[roles.root]
kind = "root"

[roles.app]
kind = "canister"
package = "app"

[auth.delegated_tokens]
enabled = false

[component_specs.app]
component_role = "app"
maximum_instances = 1
"#;

    #[cfg(unix)]
    #[test]
    fn generated_source_links_never_reuse_or_overwrite_their_target() {
        let root = std::env::temp_dir().join(format!(
            "canic-generated-source-links-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let original = root.join("original.rs");
        let generated = root.join("generated.rs");
        fs::write(&original, "original").unwrap();
        std::os::unix::fs::symlink(&original, &generated).unwrap();
        for candidate in ["original", "changed"] {
            assert_eq!(
                write_build_source_if_changed(&generated, candidate)
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::InvalidInput
            );
            assert_eq!(fs::read_to_string(&original).unwrap(), "original");
            assert!(fs::symlink_metadata(&generated).unwrap().is_symlink());
        }
        fs::remove_file(&original).unwrap();
        assert_eq!(
            write_build_source_if_changed(&generated, "new")
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
        assert!(!original.exists());
        fs::remove_file(&generated).unwrap();
        write_build_source_if_changed(&generated, "local").unwrap();
        let modified = fs::metadata(&generated).unwrap().modified().unwrap();
        write_build_source_if_changed(&generated, "local").unwrap();
        assert_eq!(
            fs::metadata(&generated).unwrap().modified().unwrap(),
            modified
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn canonical_role_contract_marker_is_required_only_for_wasm_builds() {
        assert_canonical_role_contract_build("x86_64-unknown-linux-gnu", None);
        assert_canonical_role_contract_build(
            "wasm32-unknown-unknown",
            Some(canic_core::role_contract::CANONICAL_BUILD_MARKER_VALUE),
        );

        assert!(
            std::panic::catch_unwind(|| {
                assert_canonical_role_contract_build("wasm32-unknown-unknown", None);
            })
            .is_err()
        );
    }

    #[test]
    fn role_build_sources_retain_full_configuration_only_for_root() {
        let config = parse_config_model(ROLE_BUILD_CONFIG).expect("build config parses");

        let ordinary = compile_role_build_sources(&config, &CanisterRole::from("app"), false)
            .expect("ordinary build sources compile");
        assert!(ordinary.root.is_none());
        assert!(
            ordinary
                .role_runtime_authority
                .contains("RoleRuntimeAuthority")
        );

        let wasm_store = compile_role_build_sources(&config, &CanisterRole::WASM_STORE, true)
            .expect("Store build sources compile");
        assert!(wasm_store.root.is_none());
        assert!(
            wasm_store
                .role_runtime_authority
                .contains("RuntimeCanisterConfig")
        );

        let root = compile_role_build_sources(&config, &CanisterRole::ROOT, false)
            .expect("Root build sources compile");
        let root_sources = root.root.expect("Root owns full configuration outputs");
        assert!(root_sources.config_model.contains("ConfigModel"));
        let reported: TomlValue = toml::from_str(&root_sources.config_toml).unwrap();
        assert_eq!(reported["roles"]["root"]["kind"].as_str(), Some("root"));
    }

    #[test]
    fn root_config_toml_preserves_multiline_values_and_defaults() {
        let package = "  app\n# part of the package path\n\n  directory\n";
        let source =
            ROLE_BUILD_CONFIG.replace("package = \"app\"", &format!("package = '''\n{package}'''"));
        let config = parse_config_model(&source).unwrap();
        let sources = compile_role_build_sources(&config, &CanisterRole::ROOT, false)
            .unwrap()
            .root
            .unwrap();
        let reported: TomlValue = toml::from_str(&sources.config_toml).unwrap();
        assert_eq!(reported["roles"]["app"]["package"].as_str(), Some(package));
        assert_eq!(
            reported["log"]["max_entries"].as_integer(),
            Some(i64::try_from(config.log.max_entries).unwrap()),
        );
        assert_eq!(reported, TomlValue::try_from(&config).unwrap());
    }

    #[test]
    fn root_config_toml_includes_resolved_public_verification_data() {
        let source = include_str!("../../../../canisters/test/delegation_root_stub/canic.toml")
            .lines()
            .filter(|line| {
                !line.starts_with("public_key_hex =")
                    && !line.starts_with("derivation_path_hash_hex =")
            })
            .collect::<Vec<_>>()
            .join("\n")
            .replace(
                "enabled = true",
                "enabled = true\nroot_canister_id = \"5lnwm-ziaaa-aaaae-agtqa-cai\"",
            )
            .replace(
                "key_id = \"key_1\"",
                "key_id = \"key_1\"\npublic_key_derivation = \"pocketic\"",
            );
        let config = parse_config_model(&source).unwrap();
        let sources = compile_role_build_sources(&config, &CanisterRole::ROOT, false)
            .unwrap()
            .root
            .unwrap();
        let reported: TomlValue = toml::from_str(&sources.config_toml).unwrap();
        let expected = &config.auth.delegated_tokens.chain_key_root_proof;
        assert!(expected.public_key_hex.is_some());
        assert!(expected.derivation_path_hash_hex.is_some());
        let proof = &reported["auth"]["delegated_tokens"]["chain_key_root_proof"];
        assert_eq!(
            proof["public_key_hex"].as_str(),
            expected.public_key_hex.as_deref()
        );
        assert_eq!(
            proof["derivation_path_hash_hex"].as_str(),
            expected.derivation_path_hash_hex.as_deref(),
        );
    }

    #[test]
    fn standalone_config_source_parses_for_plain_role() {
        let source = standalone_config_source("sandbox_blank");
        let cfg = parse_config_model(&source).expect("generated standalone config parses");

        assert_eq!(cfg.app_id().as_str(), "standalone");
        assert!(cfg.roles.contains_key("sandbox_blank"));
        assert!(!cfg.roles.contains_key("root"));
        assert!(cfg.component_specs.is_empty());
        assert!(!cfg.auth.delegated_tokens.enabled);
        assert!(
            !cfg.deployable_roles()
                .contains(&CanisterRole::from("sandbox_blank"))
        );
    }

    #[test]
    fn standalone_config_source_uses_canonical_bare_role_key() {
        let source = standalone_config_source("demo_role");
        let cfg = parse_config_model(&source).expect("generated standalone config parses");

        assert_eq!(cfg.app_id().as_str(), "standalone");
        assert!(source.contains("[roles.demo_role]"));
        assert!(cfg.roles.contains_key("demo_role"));
        assert!(cfg.component_specs.is_empty());
    }

    #[test]
    fn fleet_subnet_root_is_deployable_outside_component_specs() {
        let cfg = parse_config_model(
            r#"
[app]
name = "demo"

[roles.root]
kind = "root"
"#,
        )
        .expect("root infrastructure config parses");

        assert!(cfg.deployable_roles().contains(&CanisterRole::from("root")));
        assert!(cfg.attached_roles().is_empty());
    }

    #[test]
    #[should_panic(expected = "standalone Canic config requires a canonical non-root role")]
    fn standalone_config_source_rejects_root_role() {
        let _ = standalone_config_source("root");
    }

    #[test]
    #[should_panic(expected = "standalone Canic config requires a canonical non-root role")]
    fn standalone_config_source_rejects_noncanonical_role() {
        let _ = standalone_config_source("demo.role");
    }

    #[test]
    fn read_config_source_or_default_generates_when_implicit_file_is_missing() {
        let missing_path =
            std::env::temp_dir().join(format!("canic-missing-default-{}.toml", std::process::id()));
        let (source, generated) =
            read_config_source_or_default(missing_path.as_path(), false, Some("test"));

        assert!(generated);
        assert!(source.contains("[roles.test]"));
        assert!(!source.contains("[component_specs."));
    }

    #[test]
    fn declared_package_metadata_reads_canic_metadata() {
        let dir = std::env::temp_dir().join(format!("canic-role-metadata-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp manifest dir");
        fs::write(
            dir.join("Cargo.toml"),
            r#"[package]
name = "canister_scale"
version = "0.1.0"
edition = "2024"

[package.metadata.canic]
app = "test"
role = "scale_replica"
"#,
        )
        .expect("write manifest");

        let metadata = declared_package_metadata(&dir).unwrap();
        assert_eq!(metadata.app, "test");
        assert_eq!(metadata.role, "scale_replica");
        fs::remove_dir_all(&dir).expect("remove temp manifest dir");
    }

    #[test]
    fn required_package_metadata_rejects_missing_canic_metadata() {
        let dir = std::env::temp_dir().join(format!(
            "canic-missing-role-metadata-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("create temp manifest dir");
        fs::write(
            dir.join("Cargo.toml"),
            r#"[package]
name = "canister_missing"
version = "0.1.0"
edition = "2024"
"#,
        )
        .expect("write manifest");

        assert!(std::panic::catch_unwind(|| required_package_metadata(&dir)).is_err());
        fs::remove_dir_all(&dir).expect("remove temp manifest dir");
    }

    #[test]
    fn config_declares_role_accepts_exact_metadata_role() {
        let cfg = parse_config_model(
            r#"


[app]
name = "test"

[roles.root]
kind = "root"

[roles.app]
kind = "canister"
package = "app"

[auth.delegated_tokens]
enabled = false

[component_specs.app]
component_role = "app"
maximum_instances = 1
"#,
        )
        .expect("config parses");

        assert!(config_declares_role(&cfg, "test", "root"));
        assert!(config_declares_role(&cfg, "test", "app"));
        assert!(!config_declares_role(&cfg, "other", "root"));
        assert!(!config_declares_role(&cfg, "other", "app"));
    }

    #[test]
    fn config_declares_role_rejects_role_typos() {
        let cfg = parse_config_model(
            r#"


[app]
name = "test"

[roles.root]
kind = "root"

[roles.app]
kind = "canister"
package = "app"

[auth.delegated_tokens]
enabled = false

[component_specs.app]
component_role = "app"
maximum_instances = 1
"#,
        )
        .expect("config parses");

        assert!(!config_declares_role(&cfg, "test", "Root"));
        assert!(!config_declares_role(&cfg, "test", "roots"));
        assert!(!config_declares_role(&cfg, "test", "missing"));
    }
}
