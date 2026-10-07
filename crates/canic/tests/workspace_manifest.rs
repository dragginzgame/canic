use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;

///
/// CanicPackageMetadata
///
struct CanicPackageMetadata {
    app: String,
    role: String,
}

///
/// CanicConfigRole
///
struct CanicConfigRole {
    config_path: PathBuf,
    kind: Option<String>,
    package_manifest: Option<PathBuf>,
    attached: bool,
}

// Returns the repository root for manifest inspection.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory should have a parent")
        .parent()
        .expect("workspace root should exist")
        .to_path_buf()
}

// Reads and parses a Cargo manifest from disk.
fn read_manifest(path: &Path) -> Value {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));

    toml::from_str::<Value>(&source)
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", path.display()))
}

// Reads and parses one Canic config from disk.
fn read_canic_config(path: &Path) -> Value {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));

    toml::from_str::<Value>(&source)
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", path.display()))
}

// Collects all workspace member Cargo manifests from the root manifest.
fn workspace_member_manifests(root: &Path, manifest: &Value) -> Vec<PathBuf> {
    let members = manifest["workspace"]["members"]
        .as_array()
        .expect("workspace.members must be an array");

    members
        .iter()
        .map(|member| {
            let member = member
                .as_str()
                .expect("workspace member entries must be strings");

            root.join(member).join("Cargo.toml")
        })
        .collect()
}

// Returns whether a workspace member belongs to Canic's governed package tree.
fn is_canic_owned_workspace_member(root: &Path, manifest_path: &Path) -> bool {
    let Ok(relative) = manifest_path.strip_prefix(root) else {
        return false;
    };

    relative
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .is_some_and(|component| matches!(component, "canisters" | "crates"))
}

// Returns Canic package metadata from one Cargo manifest.
fn canic_package_metadata(manifest: &Value) -> Option<CanicPackageMetadata> {
    let canic = manifest
        .get("package")?
        .get("metadata")?
        .get("canic")?
        .as_table()?;
    Some(CanicPackageMetadata {
        app: canic.get("app")?.as_str()?.to_string(),
        role: canic.get("role")?.as_str()?.to_string(),
    })
}

// Returns whether a member manifest is explicitly unpublished.
fn is_explicitly_unpublished(manifest: &Value) -> bool {
    manifest["package"]["publish"].as_bool() == Some(false)
}

// Returns the crate types declared by a member manifest's [lib] section.
fn lib_crate_types(manifest: &Value) -> BTreeSet<&str> {
    manifest
        .get("lib")
        .and_then(|lib| lib.get("crate-type"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

// Walks a directory tree and collects files with the requested name.
fn collect_named_files(root: &Path, file_name: &str, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).unwrap_or_else(|err| {
        panic!(
            "failed to read directory while collecting {file_name}: {}: {err}",
            root.display()
        )
    });

    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| {
                panic!(
                    "failed to read directory entry in {}: {err}",
                    root.display()
                )
            })
            .path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if path.is_dir() {
            if matches!(name, ".canic" | ".git" | ".tmp" | "target") {
                continue;
            }
            collect_named_files(&path, file_name, files);
        } else if name == file_name {
            files.push(path);
        }
    }
}

// Returns maintained roles, excluding isolated Host role-contract templates.
fn declared_canic_roles(root: &Path) -> BTreeMap<(String, String), CanicConfigRole> {
    let mut config_paths = Vec::new();
    for source_root in ["apps", "canisters", "crates"] {
        collect_named_files(&root.join(source_root), "canic.toml", &mut config_paths);
    }

    let mut roles = BTreeMap::new();
    for config_path in config_paths {
        // These intentionally include invalid inputs. Their owning Host tests
        // materialize Cargo.toml.fixture into separate invocation workspaces.
        if config_path.starts_with(root.join("crates/canic-host/tests/fixtures/role_contract")) {
            continue;
        }
        let config = read_canic_config(&config_path);
        let Some(app) = config
            .get("app")
            .and_then(|app| app.get("name"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        let Some(role_table) = config.get("roles").and_then(Value::as_table) else {
            continue;
        };

        for (role, declaration) in role_table {
            let declaration = declaration.as_table();
            let kind = declaration
                .and_then(|table| table.get("kind"))
                .and_then(Value::as_str)
                .map(str::to_string);
            let package_manifest = declaration
                .and_then(|table| table.get("package"))
                .and_then(Value::as_str)
                .map(|package| {
                    config_path
                        .parent()
                        .expect("config should have a parent directory")
                        .join(package)
                        .join("Cargo.toml")
                });
            let attached = (role == "root" && kind.as_deref() == Some("root"))
                || config
                    .get("component_specs")
                    .and_then(Value::as_table)
                    .is_some_and(|component_specs| {
                        component_specs.values().any(|component_spec| {
                            component_spec
                                .get("component_role")
                                .and_then(Value::as_str)
                                .is_some_and(|component_role| component_role == role)
                                || component_spec
                                    .get("children")
                                    .and_then(Value::as_table)
                                    .is_some_and(|children| children.contains_key(role))
                        })
                    });

            roles.insert(
                (app.to_string(), role.clone()),
                CanicConfigRole {
                    config_path: config_path.clone(),
                    kind,
                    package_manifest,
                    attached,
                },
            );
        }
    }

    roles
}

// Formats a path relative to the workspace root for stable test output.
fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

// Returns a stable absolute path when the path exists.
fn comparable_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

// Returns whether a package intentionally relies on generated standalone config.
fn uses_generated_standalone_config(
    manifest_path: &Path,
    manifest: &Value,
    metadata: &CanicPackageMetadata,
) -> bool {
    if metadata.app != "standalone" || metadata.role == "root" {
        return false;
    }
    if !is_explicitly_unpublished(manifest) || !lib_crate_types(manifest).contains("cdylib") {
        return false;
    }

    let Some(package_dir) = manifest_path.parent() else {
        return false;
    };
    if package_dir.join("canic.toml").exists() {
        return false;
    }

    fs::read_to_string(package_dir.join("build.rs")).is_ok_and(|source| {
        source.contains("canic::build!(\"canic.toml\")")
            || source.contains("canic::build!(\"./canic.toml\")")
    })
}

// Records dependency tables that pin versions or local paths in member manifests.
#[test]
fn cdylib_members_do_not_emit_rlib_artifacts() {
    let root = workspace_root();
    let root_manifest_path = root.join("Cargo.toml");
    let root_manifest = read_manifest(&root_manifest_path);
    let member_manifests = workspace_member_manifests(&root, &root_manifest);

    let mut failures = Vec::new();
    for manifest_path in member_manifests {
        if !is_canic_owned_workspace_member(&root, &manifest_path) {
            continue;
        }

        let manifest = read_manifest(&manifest_path);
        let crate_types = lib_crate_types(&manifest);

        if crate_types.contains("cdylib") && crate_types.contains("rlib") {
            failures.push(format!(
                "{}: [lib] crate-type must not combine `cdylib` canister artifacts with `rlib` Rust library artifacts",
                relative_display(&root, &manifest_path),
            ));
        }
    }

    if !failures.is_empty() {
        failures.sort();
        panic!(
            "canister artifact crates expose Rust library artifacts:\n{}",
            failures.join("\n")
        );
    }
}

// Verifies checked-in role declarations point at real package manifests.
#[test]
fn canic_role_declaration_packages_exist() {
    let root = workspace_root();
    let declared_roles = declared_canic_roles(&root);

    let mut failures = Vec::new();
    for ((app, role), declaration) in declared_roles {
        match declaration.package_manifest.as_ref() {
            None if role == "root" && declaration.kind.as_deref() == Some("root") => {}
            Some(package_manifest) if package_manifest.is_file() => {}
            Some(package_manifest) => failures.push(format!(
                "{}: [roles.{role}] package for {app}.{role} must contain Cargo.toml, missing {}",
                relative_display(&root, &declaration.config_path),
                relative_display(&root, package_manifest)
            )),
            None => failures.push(format!(
                "{}: [roles.{role}] package for {app}.{role} must be declared",
                relative_display(&root, &declaration.config_path)
            )),
        }
    }

    if !failures.is_empty() {
        failures.sort();
        panic!(
            "Canic role declaration packages are not concrete package paths:\n{}",
            failures.join("\n")
        );
    }
}

// Verifies canister package metadata stays aligned with app role declarations.
#[test]
fn canic_package_metadata_resolves_to_declared_app_roles() {
    let root = workspace_root();
    let root_manifest_path = root.join("Cargo.toml");
    let root_manifest = read_manifest(&root_manifest_path);
    let member_manifests = workspace_member_manifests(&root, &root_manifest);
    let declared_roles = declared_canic_roles(&root);

    let mut failures = Vec::new();
    for manifest_path in member_manifests {
        let manifest = read_manifest(&manifest_path);
        let Some(metadata) = canic_package_metadata(&manifest) else {
            continue;
        };
        if metadata.app.trim().is_empty() || metadata.role.trim().is_empty() {
            failures.push(format!(
                "{}: [package.metadata.canic] app and role must be non-empty strings",
                relative_display(&root, &manifest_path)
            ));
            continue;
        }

        let Some(role) = declared_roles.get(&(metadata.app.clone(), metadata.role.clone())) else {
            if uses_generated_standalone_config(&manifest_path, &manifest, &metadata) {
                continue;
            }
            failures.push(format!(
                "{}: [package.metadata.canic] {}.{} is not declared by any canic.toml [roles.{}]",
                relative_display(&root, &manifest_path),
                metadata.app,
                metadata.role,
                metadata.role
            ));
            continue;
        };

        match role.package_manifest.as_ref() {
            None if metadata.role == "root"
                && manifest_path.starts_with(root.join("canisters"))
                && is_explicitly_unpublished(&manifest) => {}
            Some(package_manifest)
                if comparable_path(package_manifest) == comparable_path(&manifest_path) => {}
            Some(package_manifest) => failures.push(format!(
                "{}: [package.metadata.canic] {}.{} package path points at {}, declared in {}",
                relative_display(&root, &manifest_path),
                metadata.app,
                metadata.role,
                relative_display(&root, package_manifest),
                relative_display(&root, &role.config_path)
            )),
            None => failures.push(format!(
                "{}: [package.metadata.canic] {}.{} resolves to a role without a package path in {}",
                relative_display(&root, &manifest_path),
                metadata.app,
                metadata.role,
                relative_display(&root, &role.config_path)
            )),
        }

        if metadata.role == "root" {
            if role.kind.as_deref() != Some("root") {
                failures.push(format!(
                    "{}: root package metadata must resolve to [roles.root] kind = \"root\"",
                    relative_display(&root, &manifest_path)
                ));
            }
            if !role.attached {
                failures.push(format!(
                    "{}: root package metadata must resolve to Fleet Subnet Root infrastructure",
                    relative_display(&root, &manifest_path)
                ));
            }
        }
    }

    if !failures.is_empty() {
        failures.sort();
        panic!(
            "Canic package metadata is not aligned with app role declarations:\n{}",
            failures.join("\n")
        );
    }
}
