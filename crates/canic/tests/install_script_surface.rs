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

// Returns the canonical tagged raw install-script URL for the current release.
fn tagged_install_script_url() -> String {
    let workspace_version = workspace_version();
    format!(
        "https://raw.githubusercontent.com/dragginzgame/canic/v{workspace_version}/scripts/dev/install_dev.sh"
    )
}

// Keeps the curlable setup script pinned to the same CLI version as the workspace release.
#[test]
fn install_script_default_cli_version_matches_workspace_version() {
    let install_script_path = workspace_root().join("scripts/dev/install_dev.sh");
    let install_script = read_text(&install_script_path);
    let workspace_version = workspace_version();
    let expected = format!("CANIC_CLI_VERSION=\"${{CANIC_CLI_VERSION:-{workspace_version}}}\"");

    assert!(
        install_script.contains(&expected),
        "expected {} to contain `{expected}`",
        install_script_path.display()
    );
}

// Keeps the root README setup curl command aligned with the current release tag.
#[test]
fn root_readme_install_url_matches_workspace_version() {
    let readme_path = workspace_root().join("README.md");
    let readme = read_text(&readme_path);
    let expected = tagged_install_script_url();

    assert!(
        readme.contains(&expected),
        "expected {} to contain `{expected}`",
        readme_path.display()
    );
}

// Keeps the host crate README setup curl command aligned with the current release tag.
#[test]
fn host_readme_install_url_matches_workspace_version() {
    let readme_path = workspace_root().join("crates/canic-host/README.md");
    let readme = read_text(&readme_path);
    let expected = tagged_install_script_url();

    assert!(
        readme.contains(&expected),
        "expected {} to contain `{expected}`",
        readme_path.display()
    );
}
