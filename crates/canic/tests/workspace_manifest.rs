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

// Reads one checked-in text file for package-surface assertions.
fn read_text(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
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

// Checks whether a manifest value inherits its setting from the workspace.
fn is_workspace_inherited(value: &Value) -> bool {
    value
        .as_table()
        .and_then(|table| table.get("workspace"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

// Returns the package name declared by a member manifest.
fn package_name(manifest: &Value) -> Option<&str> {
    manifest["package"]["name"].as_str()
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

// Returns whether a member manifest is explicitly published.
fn is_explicitly_publishable(manifest: &Value) -> bool {
    manifest["package"]["publish"].as_bool() == Some(true)
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

// Returns the dependency entries declared by one feature.
fn feature_entries<'a>(manifest: &'a Value, feature: &str) -> BTreeSet<&'a str> {
    manifest["features"][feature]
        .as_array()
        .unwrap_or_else(|| panic!("feature {feature} must be declared"))
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .unwrap_or_else(|| panic!("feature {feature} entries must be strings"))
        })
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
            if matches!(name, ".git" | ".tmp" | "target") {
                continue;
            }
            collect_named_files(&path, file_name, files);
        } else if name == file_name {
            files.push(path);
        }
    }
}

// Returns the declared Canic roles from repository configs, keyed by app and role.
fn declared_canic_roles(root: &Path) -> BTreeMap<(String, String), CanicConfigRole> {
    let mut config_paths = Vec::new();
    collect_named_files(root, "canic.toml", &mut config_paths);

    let mut roles = BTreeMap::new();
    for config_path in config_paths {
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
            let attached = config
                .get("subnets")
                .and_then(Value::as_table)
                .is_some_and(|subnets| {
                    subnets.values().any(|subnet| {
                        subnet
                            .get("canisters")
                            .and_then(Value::as_table)
                            .is_some_and(|canisters| canisters.contains_key(role))
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

// Allow the one intentional local-only dev-dependency edge for unpublished
// internal self-test support.
fn allow_local_path_dependency(
    root: &Path,
    manifest_path: &Path,
    section_path: &str,
    name: &str,
) -> bool {
    let manifest = relative_display(root, manifest_path);
    if manifest == "crates/canic-core/Cargo.toml"
        && section_path == "dev-dependencies"
        && name == "canic-testing-internal"
    {
        return true;
    }

    matches!(
        manifest.as_str(),
        "canisters/audit/minimal/Cargo.toml"
            | "canisters/audit/minimal_metrics/Cargo.toml"
            | "canisters/sandbox/blank/Cargo.toml"
    ) && matches!(section_path, "dependencies" | "build-dependencies")
        && name == "canic"
}

// Records dependency tables that pin versions or local paths in member manifests.
fn collect_dependency_failures(
    root: &Path,
    manifest_path: &Path,
    table_path: &str,
    value: &Value,
    workspace_dependencies: &BTreeSet<String>,
    failures: &mut Vec<String>,
) {
    let Some(table) = value.as_table() else {
        return;
    };

    for (key, entry) in table {
        if matches!(
            key.as_str(),
            "dependencies" | "dev-dependencies" | "build-dependencies"
        ) {
            let Some(dependencies) = entry.as_table() else {
                failures.push(format!(
                    "{}: [{}] must be a table",
                    relative_display(root, manifest_path),
                    if table_path.is_empty() {
                        key.clone()
                    } else {
                        format!("{table_path}.{key}")
                    }
                ));
                continue;
            };

            check_dependency_table(
                root,
                manifest_path,
                table_path,
                key,
                dependencies,
                workspace_dependencies,
                failures,
            );
            continue;
        }

        let next_path = if table_path.is_empty() {
            key.clone()
        } else {
            format!("{table_path}.{key}")
        };

        collect_dependency_failures(
            root,
            manifest_path,
            &next_path,
            entry,
            workspace_dependencies,
            failures,
        );
    }
}

// Collects runtime/build dependency tables from nested target-specific
// manifest sections.
fn collect_publish_dependency_failures(
    root: &Path,
    manifest_path: &Path,
    table_path: &str,
    value: &Value,
    unpublished_workspace_members: &BTreeSet<String>,
    failures: &mut Vec<String>,
) {
    let Some(table) = value.as_table() else {
        return;
    };

    for (key, entry) in table {
        if matches!(key.as_str(), "dependencies" | "build-dependencies") {
            let Some(dependencies) = entry.as_table() else {
                failures.push(format!(
                    "{}: [{}] must be a table",
                    relative_display(root, manifest_path),
                    if table_path.is_empty() {
                        key.clone()
                    } else {
                        format!("{table_path}.{key}")
                    }
                ));
                continue;
            };

            let section_path = if table_path.is_empty() {
                key.clone()
            } else {
                format!("{table_path}.{key}")
            };
            for name in dependencies.keys() {
                if unpublished_workspace_members.contains(name) {
                    failures.push(format!(
                        "{}: [{section_path}] {name} is an unpublished workspace crate; publishable crates must not depend on it at runtime or build time",
                        relative_display(root, manifest_path),
                    ));
                }
            }
            continue;
        }

        let next_path = if table_path.is_empty() {
            key.clone()
        } else {
            format!("{table_path}.{key}")
        };

        collect_publish_dependency_failures(
            root,
            manifest_path,
            &next_path,
            entry,
            unpublished_workspace_members,
            failures,
        );
    }
}

// Validates one dependency table against the workspace-inheritance rule.
fn check_dependency_table(
    root: &Path,
    manifest_path: &Path,
    table_path: &str,
    section: &str,
    dependencies: &toml::map::Map<String, Value>,
    workspace_dependencies: &BTreeSet<String>,
    failures: &mut Vec<String>,
) {
    let section_path = if table_path.is_empty() {
        section.to_string()
    } else {
        format!("{table_path}.{section}")
    };

    for (name, spec) in dependencies {
        match spec {
            Value::String(version) => failures.push(format!(
                "{}: [{section_path}] {name} pins version {version:?}; move it to workspace.dependencies and use `workspace = true`",
                relative_display(root, manifest_path),
            )),
            Value::Table(table) => {
                if is_workspace_inherited(spec) {
                    continue;
                }

                if table.contains_key("path") {
                    if allow_local_path_dependency(root, manifest_path, &section_path, name) {
                        continue;
                    }
                    failures.push(format!(
                        "{}: [{section_path}] {name} uses a local `path`; use the workspace root declaration instead",
                        relative_display(root, manifest_path),
                    ));
                    continue;
                }

                if table.contains_key("version") {
                    failures.push(format!(
                        "{}: [{section_path}] {name} pins a local `version`; move it to workspace.dependencies and use `workspace = true`",
                        relative_display(root, manifest_path),
                    ));
                    continue;
                }

                if workspace_dependencies.contains(name) {
                    failures.push(format!(
                        "{}: [{section_path}] {name} should use `workspace = true`",
                        relative_display(root, manifest_path),
                    ));
                }
            }
            _ => failures.push(format!(
                "{}: [{section_path}] {name} has an unsupported dependency shape",
                relative_display(root, manifest_path),
            )),
        }
    }
}

// Verifies every workspace member inherits package and dependency versions from the root.
#[test]
fn workspace_members_inherit_versions_from_root() {
    let root = workspace_root();
    let root_manifest_path = root.join("Cargo.toml");
    let root_manifest = read_manifest(&root_manifest_path);
    let member_manifests = workspace_member_manifests(&root, &root_manifest);
    let workspace_dependencies = root_manifest["workspace"]["dependencies"]
        .as_table()
        .expect("workspace.dependencies must be a table")
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut failures = Vec::new();

    // Validate each workspace member against the root manifest contract.
    for manifest_path in member_manifests {
        let manifest = read_manifest(&manifest_path);
        let Some(package) = manifest.get("package").and_then(Value::as_table) else {
            continue;
        };

        match package.get("version") {
            Some(version) if is_workspace_inherited(version) => {}
            other => failures.push(format!(
                "{}: [package] version must be `{{ workspace = true }}`, found {other:?}",
                relative_display(&root, &manifest_path),
            )),
        }

        collect_dependency_failures(
            &root,
            &manifest_path,
            "",
            &manifest,
            &workspace_dependencies,
            &mut failures,
        );
    }

    if !failures.is_empty() {
        failures.sort();
        panic!(
            "workspace manifest drift detected:\n{}",
            failures.join("\n")
        );
    }
}

// Verifies publishable crates do not compile against local unpublished crates.
#[test]
fn publishable_members_do_not_depend_on_unpublished_workspace_members() {
    let root = workspace_root();
    let root_manifest_path = root.join("Cargo.toml");
    let root_manifest = read_manifest(&root_manifest_path);
    let member_manifests = workspace_member_manifests(&root, &root_manifest);

    let manifests = member_manifests
        .into_iter()
        .map(|manifest_path| {
            let manifest = read_manifest(&manifest_path);
            (manifest_path, manifest)
        })
        .collect::<Vec<_>>();

    let mut publishability = BTreeMap::new();
    for (_, manifest) in &manifests {
        let Some(name) = package_name(manifest) else {
            continue;
        };
        publishability.insert(name.to_string(), is_explicitly_unpublished(manifest));
    }

    let unpublished_workspace_members = publishability
        .into_iter()
        .filter_map(|(name, unpublished)| unpublished.then_some(name))
        .collect::<BTreeSet<_>>();

    let mut failures = Vec::new();
    for (manifest_path, manifest) in manifests {
        if !is_explicitly_publishable(&manifest) {
            continue;
        }

        collect_publish_dependency_failures(
            &root,
            &manifest_path,
            "",
            &manifest,
            &unpublished_workspace_members,
            &mut failures,
        );
    }

    if !failures.is_empty() {
        failures.sort();
        panic!(
            "publishable workspace member depends on unpublished local crate:\n{}",
            failures.join("\n")
        );
    }
}

// Verifies canister artifact crates do not also publish Rust library artifacts.
#[test]
fn cdylib_members_do_not_emit_rlib_artifacts() {
    let root = workspace_root();
    let root_manifest_path = root.join("Cargo.toml");
    let root_manifest = read_manifest(&root_manifest_path);
    let member_manifests = workspace_member_manifests(&root, &root_manifest);

    let mut failures = Vec::new();
    for manifest_path in member_manifests {
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
                    "{}: root package metadata must resolve to attached root topology",
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

// Verifies the blob-storage billing feature stays opt-in and layered.
#[test]
fn blob_storage_billing_feature_is_opt_in_and_implies_blob_storage() {
    let root = workspace_root();
    let core_manifest = read_manifest(&root.join("crates/canic-core/Cargo.toml"));
    let facade_manifest = read_manifest(&root.join("crates/canic/Cargo.toml"));

    let core_default = feature_entries(&core_manifest, "default");
    let core_billing = feature_entries(&core_manifest, "blob-storage-billing");
    assert!(
        core_default.is_empty(),
        "canic-core default features changed"
    );
    assert_eq!(
        core_billing,
        BTreeSet::from(["blob-storage"]),
        "canic-core blob-storage-billing feature must imply only blob-storage"
    );

    let facade_default = feature_entries(&facade_manifest, "default");
    let facade_billing = feature_entries(&facade_manifest, "blob-storage-billing");
    assert!(
        !facade_default.contains("blob-storage-billing"),
        "canic blob-storage-billing must stay off by default"
    );
    assert_eq!(
        facade_billing,
        BTreeSet::from(["blob-storage", "canic-core/blob-storage-billing"]),
        "canic blob-storage-billing feature must imply facade and core blob storage"
    );
}

// Verifies published package feature tables describe every maintained feature
// and identify the exact default set from the owning Cargo manifest.
#[test]
fn published_package_feature_docs_match_manifests() {
    let root = workspace_root();

    for package in ["canic", "canic-control-plane"] {
        let package_root = root.join("crates").join(package);
        let manifest = read_manifest(&package_root.join("Cargo.toml"));
        let readme = read_text(&package_root.join("README.md"));
        let features = manifest["features"]
            .as_table()
            .unwrap_or_else(|| panic!("{package} must declare a feature table"));
        let defaults = feature_entries(&manifest, "default");
        let expected_features = features
            .keys()
            .filter(|feature| feature.as_str() != "default")
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let feature_contract = readme
            .split_once("## Feature Contract\n")
            .unwrap_or_else(|| panic!("{package} README must contain a Feature Contract section"))
            .1
            .split("\n## ")
            .next()
            .expect("feature contract section must be present");
        let documented_features = feature_contract
            .lines()
            .filter_map(|line| line.strip_prefix("| `"))
            .filter_map(|line| line.split_once("` |"))
            .map(|(feature, _)| feature)
            .collect::<BTreeSet<_>>();

        assert_eq!(
            documented_features,
            expected_features,
            "{} must document exactly its maintained feature set",
            package_root.join("README.md").display()
        );

        for feature in features
            .keys()
            .filter(|feature| feature.as_str() != "default")
        {
            let default_label = if defaults.contains(feature.as_str()) {
                "Yes"
            } else {
                "No"
            };
            let row_prefix = format!("| `{feature}` | {default_label} |");
            assert!(
                readme.lines().any(|line| line.starts_with(&row_prefix)),
                "{} must document feature {feature} with default={default_label}",
                package_root.join("README.md").display()
            );
        }
    }
}
