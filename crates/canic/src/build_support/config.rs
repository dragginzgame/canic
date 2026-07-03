use std::{collections::BTreeSet, fmt::Write as _, fs, path::Path};

use canic_core::{
    bootstrap::{CanicFeatureRequirement, compiled::ConfigModel},
    ids::CanisterRole,
};
use toml::Value as TomlValue;

///
/// PackageCanicMetadata
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageCanicMetadata {
    pub fleet: String,
    pub role: String,
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
pub fn declared_package_metadata(manifest_dir: &Path) -> Option<PackageCanicMetadata> {
    let manifest = fs::read_to_string(manifest_dir.join("Cargo.toml")).ok()?;
    let canic = toml::from_str::<TomlValue>(&manifest)
        .ok()?
        .get("package")?
        .get("metadata")?
        .get("canic")?
        .clone();
    let fleet = canic.get("fleet")?.as_str()?.to_string();
    let role = canic.get("role")?.as_str()?.to_string();

    Some(PackageCanicMetadata { fleet, role })
}

/// Read an optional Canic role declared in the package manifest metadata.
#[must_use]
pub fn declared_package_role(manifest_dir: &Path) -> Option<String> {
    declared_package_metadata(manifest_dir).map(|metadata| metadata.role)
}

/// Read the required Canic metadata declared in package manifest metadata.
///
/// # Panics
///
/// Panics when `Cargo.toml` does not declare `[package.metadata.canic]` with
/// both `fleet` and `role`.
#[must_use]
pub fn required_package_metadata(manifest_dir: &Path) -> PackageCanicMetadata {
    let manifest_path = manifest_dir.join("Cargo.toml");
    declared_package_metadata(manifest_dir).unwrap_or_else(|| {
        panic!(
            "missing Canic package metadata in {}; add [package.metadata.canic] fleet = \"<fleet>\" and role = \"<role>\"",
            manifest_path.display()
        )
    })
}

/// Read the required Canic role declared in package manifest metadata.
#[must_use]
pub fn required_package_role(manifest_dir: &Path) -> String {
    required_package_metadata(manifest_dir).role
}

/// Fail when the package's runtime `canic` dependency lacks required features.
///
/// # Panics
///
/// Panics when `Cargo.toml` cannot be read or parsed, or when a required
/// feature is missing from `[dependencies].canic`.
pub fn assert_required_canic_dependency_features(
    manifest_dir: &Path,
    fleet: &str,
    role: &str,
    requirements: &[CanicFeatureRequirement],
) {
    if requirements.is_empty() {
        return;
    }

    let manifest_path = manifest_dir.join("Cargo.toml");
    let features = declared_canic_dependency_features(manifest_dir);
    let missing = requirements
        .iter()
        .filter(|requirement| !features.contains(requirement.feature))
        .collect::<Vec<_>>();

    if missing.is_empty() {
        return;
    }

    let missing_features = missing
        .iter()
        .map(|requirement| format!("`{}`", requirement.feature))
        .collect::<Vec<_>>()
        .join(", ");
    let reasons = missing
        .iter()
        .map(|requirement| {
            format!(
                "{} requires {} ({})",
                requirement.config_key, requirement.feature, requirement.reason
            )
        })
        .collect::<Vec<_>>()
        .join("; ");

    panic!(
        "canister role '{fleet}.{role}' requires missing canic feature(s) {missing_features}; {reasons}; add the feature(s) to [dependencies].canic features or inherited [workspace.dependencies].canic features in {}",
        manifest_path.display()
    );
}

/// Read features enabled on this package's runtime `canic` dependency.
#[must_use]
pub fn declared_canic_dependency_features(manifest_dir: &Path) -> BTreeSet<String> {
    let manifest_path = manifest_dir.join("Cargo.toml");
    let manifest_source = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", manifest_path.display()));
    let manifest = toml::from_str::<TomlValue>(&manifest_source)
        .unwrap_or_else(|err| panic!("invalid {}: {err}", manifest_path.display()));

    let mut features = canic_dependency_features(&manifest);
    if canic_dependency_inherits_workspace(&manifest) {
        features.extend(workspace_canic_dependency_features(manifest_dir));
    }
    features
}

fn canic_dependency_features(manifest: &TomlValue) -> BTreeSet<String> {
    manifest
        .get("dependencies")
        .and_then(|dependencies| dependencies.get("canic"))
        .map(toml_dependency_features)
        .unwrap_or_default()
}

fn canic_dependency_inherits_workspace(manifest: &TomlValue) -> bool {
    manifest
        .get("dependencies")
        .and_then(|dependencies| dependencies.get("canic"))
        .and_then(|canic| canic.get("workspace"))
        .and_then(TomlValue::as_bool)
        .unwrap_or(false)
}

fn workspace_canic_dependency_features(manifest_dir: &Path) -> BTreeSet<String> {
    for dir in manifest_dir.ancestors() {
        let manifest_path = dir.join("Cargo.toml");
        let Ok(manifest_source) = fs::read_to_string(&manifest_path) else {
            continue;
        };
        let Ok(manifest) = toml::from_str::<TomlValue>(&manifest_source) else {
            continue;
        };
        if let Some(features) = workspace_canic_dependency_features_from_manifest(&manifest) {
            return features;
        }
    }

    BTreeSet::new()
}

fn workspace_canic_dependency_features_from_manifest(
    manifest: &TomlValue,
) -> Option<BTreeSet<String>> {
    manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(|dependencies| dependencies.get("canic"))
        .map(toml_dependency_features)
}

fn toml_dependency_features(dependency: &TomlValue) -> BTreeSet<String> {
    dependency
        .get("features")
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(TomlValue::as_str)
        .map(ToString::to_string)
        .collect()
}

/// Return whether a validated config declares the requested fleet role.
#[must_use]
pub fn config_declares_role(config: &ConfigModel, fleet_name: &str, role_name: &str) -> bool {
    config.fleet_name() == Some(fleet_name)
        && config
            .roles
            .contains_key(&CanisterRole::owned(role_name.to_string()))
}

/// Return the fleet name declared by a validated config.
#[must_use]
pub fn config_fleet_name(config: &ConfigModel) -> Option<&str> {
    config.fleet_name()
}

/// Return whether a validated config attaches the requested fleet role.
#[must_use]
pub fn config_attaches_role(config: &ConfigModel, fleet_name: &str, role_name: &str) -> bool {
    if config.fleet_name() != Some(fleet_name) {
        return false;
    }

    config
        .attached_roles()
        .contains(&CanisterRole::owned(role_name.to_string()))
}

/// Return whether a validated config contains the requested canister role.
#[must_use]
pub fn config_contains_role(config: &ConfigModel, role_name: &str) -> bool {
    config_declares_role(config, config.fleet_name().unwrap_or_default(), role_name)
}

/// Render the minimal declared-only config needed by a standalone non-root canister.
///
/// # Panics
///
/// Panics when `role` is empty or names the root canister.
#[must_use]
pub fn standalone_config_source(role: &str) -> String {
    assert!(
        !role.is_empty() && role != "root",
        "standalone Canic config requires a non-root role"
    );

    let role_key = toml_basic_string(role);

    format!(
        r#"controllers = []
app_index = []

[fleet]
name = "standalone"

[roles.{role_key}]
kind = "canister"
package = "."

[app]
init_mode = "enabled"

[app.whitelist]

[auth.delegated_tokens]
enabled = false
"#
    )
}

// Escape a role name as a TOML basic string for quoted table keys.
fn toml_basic_string(value: &str) -> String {
    let mut rendered = String::with_capacity(value.len() + 2);
    rendered.push('"');

    for ch in value.chars() {
        match ch {
            '"' => rendered.push_str("\\\""),
            '\\' => rendered.push_str("\\\\"),
            '\u{08}' => rendered.push_str("\\b"),
            '\t' => rendered.push_str("\\t"),
            '\n' => rendered.push_str("\\n"),
            '\u{0c}' => rendered.push_str("\\f"),
            '\r' => rendered.push_str("\\r"),
            ch if ch.is_control() => {
                let _ = write!(rendered, "\\u{:04X}", ch as u32);
            }
            ch => rendered.push(ch),
        }
    }

    rendered.push('"');
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::bootstrap::parse_config_model;

    #[test]
    fn standalone_config_source_parses_for_plain_role() {
        let source = standalone_config_source("sandbox_blank");
        let cfg = parse_config_model(&source).expect("generated standalone config parses");

        assert_eq!(cfg.fleet_name(), Some("standalone"));
        assert!(cfg.roles.contains_key("sandbox_blank"));
        assert!(!cfg.roles.contains_key("root"));
        assert!(cfg.subnets.is_empty());
        assert!(!cfg.auth.delegated_tokens.enabled);
        assert!(!config_attaches_role(&cfg, "standalone", "sandbox_blank"));
    }

    #[test]
    fn standalone_config_source_quotes_role_keys() {
        let source = standalone_config_source("demo.role");
        let cfg = parse_config_model(&source).expect("generated standalone config parses");

        assert_eq!(cfg.fleet_name(), Some("standalone"));
        assert!(cfg.roles.contains_key("demo.role"));
        assert!(cfg.subnets.is_empty());
    }

    #[test]
    #[should_panic(expected = "standalone Canic config requires a non-root role")]
    fn standalone_config_source_rejects_root_role() {
        let _ = standalone_config_source("root");
    }

    #[test]
    fn read_config_source_or_default_generates_when_implicit_file_is_missing() {
        let missing_path =
            std::env::temp_dir().join(format!("canic-missing-default-{}.toml", std::process::id()));
        let (source, generated) =
            read_config_source_or_default(missing_path.as_path(), false, Some("test"));

        assert!(generated);
        assert!(source.contains("[roles.\"test\"]"));
        assert!(!source.contains("[subnets."));
    }

    #[test]
    fn declared_package_role_reads_canic_metadata() {
        let dir = std::env::temp_dir().join(format!("canic-role-metadata-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp manifest dir");
        fs::write(
            dir.join("Cargo.toml"),
            r#"[package]
name = "canister_scale"
version = "0.1.0"
edition = "2024"

[package.metadata.canic]
fleet = "test"
role = "scale_replica"
"#,
        )
        .expect("write manifest");

        assert_eq!(
            declared_package_role(&dir).as_deref(),
            Some("scale_replica")
        );
        fs::remove_dir_all(&dir).expect("remove temp manifest dir");
    }

    #[test]
    fn required_package_role_rejects_missing_canic_metadata() {
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

        let panic = std::panic::catch_unwind(|| required_package_role(&dir))
            .expect_err("missing metadata should panic");
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .expect("panic should include a message");

        assert!(message.contains("missing Canic package metadata"));
        fs::remove_dir_all(&dir).expect("remove temp manifest dir");
    }

    #[test]
    fn declared_canic_dependency_features_reads_runtime_dependency_features() {
        let dir =
            std::env::temp_dir().join(format!("canic-runtime-features-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp manifest dir");
        fs::write(
            dir.join("Cargo.toml"),
            r#"[package]
name = "canister_app"
version = "0.1.0"
edition = "2024"

[dependencies]
canic = { workspace = true, features = ["auth-root-canister-sig-verify"] }
"#,
        )
        .expect("write manifest");

        let features = declared_canic_dependency_features(&dir);

        assert!(features.contains("auth-root-canister-sig-verify"));
        fs::remove_dir_all(&dir).expect("remove temp manifest dir");
    }

    #[test]
    fn declared_canic_dependency_features_reads_workspace_dependency_features() {
        let dir = std::env::temp_dir().join(format!(
            "canic-runtime-workspace-features-{}",
            std::process::id()
        ));
        let app = dir.join("app");
        fs::create_dir_all(&app).expect("create temp app dir");
        fs::write(
            dir.join("Cargo.toml"),
            r#"[workspace]
members = ["app"]

[workspace.dependencies]
canic = { path = "../canic", features = ["auth-root-canister-sig-verify"] }
"#,
        )
        .expect("write workspace manifest");
        fs::write(
            app.join("Cargo.toml"),
            r#"[package]
name = "canister_app"
version = "0.1.0"
edition = "2024"

[dependencies]
canic = { workspace = true }
"#,
        )
        .expect("write app manifest");

        let features = declared_canic_dependency_features(&app);

        assert!(features.contains("auth-root-canister-sig-verify"));
        fs::remove_dir_all(&dir).expect("remove temp manifest dir");
    }

    #[test]
    fn required_canic_dependency_features_rejects_missing_runtime_feature() {
        let dir = std::env::temp_dir().join(format!(
            "canic-missing-runtime-feature-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("create temp manifest dir");
        fs::write(
            dir.join("Cargo.toml"),
            r#"[package]
name = "canister_app"
version = "0.1.0"
edition = "2024"

[dependencies]
canic = { workspace = true, features = ["auth-delegated-token-verify"] }
"#,
        )
        .expect("write manifest");

        let requirement = CanicFeatureRequirement {
            config_key: "auth.role_attestation_cache",
            feature: "auth-root-canister-sig-verify",
            reason: "role-attestation cache verifies root canister-signature proofs locally",
        };
        let panic = std::panic::catch_unwind(|| {
            assert_required_canic_dependency_features(&dir, "demo", "app", &[requirement]);
        })
        .expect_err("missing feature should panic");
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .expect("panic should include a message");

        assert!(message.contains("demo.app"));
        assert!(message.contains("auth-root-canister-sig-verify"));
        assert!(message.contains("[workspace.dependencies].canic features"));
        fs::remove_dir_all(&dir).expect("remove temp manifest dir");
    }

    #[test]
    fn config_contains_role_accepts_exact_metadata_role() {
        let cfg = parse_config_model(
            r#"
[subnets.prime.canisters.root]
kind = "root"

[fleet]
name = "test"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[auth.delegated_tokens]
enabled = false

[subnets.prime.canisters.app]
kind = "service"
"#,
        )
        .expect("config parses");

        assert!(config_contains_role(&cfg, "root"));
        assert!(config_contains_role(&cfg, "app"));
    }

    #[test]
    fn config_contains_role_rejects_role_typos() {
        let cfg = parse_config_model(
            r#"
[subnets.prime.canisters.root]
kind = "root"

[fleet]
name = "test"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[auth.delegated_tokens]
enabled = false

[subnets.prime.canisters.app]
kind = "service"
"#,
        )
        .expect("config parses");

        assert!(!config_contains_role(&cfg, "Root"));
        assert!(!config_contains_role(&cfg, "roots"));
        assert!(!config_contains_role(&cfg, "missing"));
    }
}
