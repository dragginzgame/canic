use std::{env, fs, path::Path, process::Command};

use toml::Value as TomlValue;

use crate::{
    cargo_command,
    evidence_envelope::{file_input_fingerprint, sha256_hex},
};

use super::{
    inputs::cargo_config_fingerprints,
    model::{BuildProvenanceRequest, BuildScriptInputStateV1, CargoProvenanceV1, WASM_TARGET},
};

pub(super) fn cargo_provenance(
    request: &BuildProvenanceRequest,
    package_manifest: &Path,
) -> Result<CargoProvenanceV1, Box<dyn std::error::Error>> {
    let manifest_source = fs::read_to_string(package_manifest)?;
    let manifest = toml::from_str::<TomlValue>(&manifest_source)?;
    let cargo_lock_path = request.workspace_root.join("Cargo.lock");
    let package_metadata_fleet = required_manifest_str(
        &manifest,
        &["package", "metadata", "canic", "fleet"],
        package_manifest,
    )?;
    let package_metadata_role = required_manifest_str(
        &manifest,
        &["package", "metadata", "canic", "role"],
        package_manifest,
    )?;
    if package_metadata_fleet != request.fleet || package_metadata_role != request.role {
        return Err(format!(
            "{} declares [package.metadata.canic] fleet={:?} role={:?}, not {}.{}",
            package_manifest.display(),
            package_metadata_fleet,
            package_metadata_role,
            request.fleet,
            request.role
        )
        .into());
    }

    Ok(CargoProvenanceV1 {
        cargo_lock_sha256: optional_file_sha256(&cargo_lock_path)?,
        package_manifest_sha256: Some(sha256_hex(manifest_source.as_bytes())),
        package_name: required_manifest_str(&manifest, &["package", "name"], package_manifest)?,
        package_manifest: display_path(package_manifest, &request.workspace_root),
        package_metadata_fleet,
        package_metadata_role,
        rustc_version: command_version("rustc", ["--version"]),
        cargo_version: cargo_version(),
        target: Some(WASM_TARGET.to_string()),
        profile: request.profile.target_dir_name().to_string(),
        features: Vec::new(),
        default_features: None,
        rustflags_digest: env::var("RUSTFLAGS")
            .ok()
            .map(|value| sha256_hex(value.as_bytes())),
        rustflags_digest_algorithm: env::var_os("RUSTFLAGS")
            .is_some()
            .then(|| "sha256".to_string()),
        cargo_config_fingerprints: cargo_config_fingerprints(&request.workspace_root)?,
        build_script_inputs: BuildScriptInputStateV1::NotRecorded,
    })
}

fn optional_file_sha256(path: &Path) -> Result<Option<String>, Box<dyn std::error::Error>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(sha256_hex(&bytes))),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

fn required_manifest_str(
    manifest: &TomlValue,
    path: &[&str],
    manifest_path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut value = manifest;
    for segment in path {
        value = value
            .get(*segment)
            .ok_or_else(|| format!("missing {} in {}", path.join("."), manifest_path.display()))?;
    }

    value.as_str().map(ToString::to_string).ok_or_else(|| {
        format!(
            "{} must be a string in {}",
            path.join("."),
            manifest_path.display()
        )
        .into()
    })
}

fn display_path(path: &Path, root: &Path) -> String {
    file_input_fingerprint("path", path, root, None, None)
        .ok()
        .and_then(|fingerprint| fingerprint.path)
        .unwrap_or_else(|| "<redacted:absolute-outside-root>".to_string())
}

fn command_version<const N: usize>(command: &str, args: [&str; N]) -> Option<String> {
    let mut command = Command::new(command);
    if let Some(toolchain) = env::var_os("RUSTUP_TOOLCHAIN") {
        command.env("RUSTUP_TOOLCHAIN", toolchain);
    }
    let output = command.args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}

fn cargo_version() -> Option<String> {
    let output = cargo_command().arg("--version").output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}
