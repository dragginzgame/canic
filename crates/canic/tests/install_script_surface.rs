use std::fs;
use std::path::{Path, PathBuf};

// Returns the repository root so release-facing scripts can be read from disk.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory should have a parent")
        .parent()
        .expect("workspace root should exist")
        .to_path_buf()
}

// Reads a checked-in text fixture from the repository root.
fn read_text(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

// Reads the current workspace package version from the root Cargo manifest.
fn workspace_version() -> String {
    let cargo_toml_path = workspace_root().join("Cargo.toml");
    let cargo_toml = read_text(&cargo_toml_path);
    let parsed: toml::Table = cargo_toml
        .parse()
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", cargo_toml_path.display()));

    parsed
        .get("workspace")
        .and_then(|value| value.as_table())
        .and_then(|table| table.get("package"))
        .and_then(|value| value.as_table())
        .and_then(|table| table.get("version"))
        .and_then(|value| value.as_str())
        .expect("workspace.package.version must be a string")
        .to_string()
}

// Keeps the curlable installer pinned to the same version as the current workspace release.
#[test]
fn install_script_default_installer_version_matches_workspace_version() {
    let install_script_path = workspace_root().join("scripts/install.sh");
    let install_script = read_text(&install_script_path);
    let workspace_version = workspace_version();
    let expected =
        format!("CANIC_INSTALLER_VERSION=\"${{CANIC_INSTALLER_VERSION:-{workspace_version}}}\"");

    assert!(
        install_script.contains(&expected),
        "expected {} to contain `{expected}`",
        install_script_path.display()
    );
}
