use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;

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

// Formats a path relative to the workspace root for stable test output.
fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

// Allow the one intentional local-only dev-dependency edge that breaks the
// publish-time test cycle between `canic-core` and the later-published
// `canic-testkit` crate.
fn allow_local_path_dependency(
    root: &Path,
    manifest_path: &Path,
    section_path: &str,
    name: &str,
) -> bool {
    relative_display(root, manifest_path) == "crates/canic-core/Cargo.toml"
        && section_path == "dev-dependencies"
        && name == "canic-testkit"
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
