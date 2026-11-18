#[cfg(test)]
mod inspection_tests;
#[cfg(test)]
mod tests;

use crate::{Error, Result, hex, supplier};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
mod source_layout;
mod store_source;

// Leave at least 190 KiB for Candid/envelope overhead below the IC's 2 MiB ingress limit.
// This is Host direct installation, independent of Root's retained inline-child quota.
pub(crate) const MAX_WASM: usize = 1_900_000;
// Build/inspection can admit the complete Store artifact without granting installation authority.
pub(crate) const MAX_ARTIFACT_BYTES: usize =
    ic_wasm_store_protocol::CHUNK_BYTES * ic_wasm_store_protocol::MAX_CHUNKS;

/// Exact byte identity: Wasm for deployment, request payload for an application call.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    /// SHA-256 of the complete payload.
    pub sha256: String,
    /// Complete payload length.
    pub bytes: usize,
}

#[derive(Debug, thiserror::Error)]
#[error("Cargo failed: {source}\n{diagnostic}")]
struct CargoFailure {
    source: crate::process::CaptureFailure,
    diagnostic: String,
}

// Only Cargo callers expose captured diagnostics. Signer/provider evidence keeps
// its existing disclosure policy; the process supplier owns capture and cleanup.
pub(crate) fn cargo_failure(source: crate::process::CaptureFailure) -> Error {
    match cargo_diagnostic(
        &source.stdout,
        &source.stderr,
        matches!(
            source.report.failure,
            Some(ic_host_process::Failure::Limit(_))
        ),
    ) {
        Some(diagnostic) => supplier(CargoFailure { source, diagnostic }),
        None => supplier(source),
    }
}

fn cargo_diagnostic(stdout: &[u8], stderr: &[u8], partial: bool) -> Option<String> {
    let compiler = stdout.split(|b| *b == b'\n').find_map(|line| {
        let value: serde_json::Value = serde_json::from_slice(line).ok()?;
        if value["reason"] != "compiler-message" || value["message"]["level"] != "error" {
            return None;
        }
        value["message"]["rendered"]
            .as_str()
            .or_else(|| value["message"]["message"].as_str())
            .filter(|text| !text.trim().is_empty())
            .map(str::to_owned)
    });
    let text = compiler.unwrap_or_else(|| String::from_utf8_lossy(stderr).into_owned());
    let mut characters = text.trim().chars();
    let prefix: String = characters.by_ref().take(4096).collect();
    if prefix.is_empty() {
        return None;
    }
    let mut diagnostic = prefix
        .split('\n')
        .map(ic_host_text::escape)
        .collect::<Vec<_>>()
        .join("\n");
    if characters.next().is_some() || partial {
        diagnostic.push_str("\n[Cargo diagnostic truncated]");
    }
    Some(diagnostic)
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

pub(crate) fn admit(bytes: &[u8]) -> Result<Artifact> {
    admit_bounded(bytes, MAX_WASM)
}

pub(crate) fn admit_bounded(bytes: &[u8], limit: usize) -> Result<Artifact> {
    if bytes.len() > limit || !bytes.starts_with(b"\0asm\x01\0\0\0") {
        return Err(Error::Invalid(format!(
            "expected uncompressed Wasm v1, at most {limit} bytes"
        )));
    }
    Ok(Artifact {
        sha256: digest(bytes),
        bytes: bytes.len(),
    })
}

/// Inspect bounded Wasm bytes without executing or installing them.
///
/// # Errors
/// Refuses unreadable, oversized or non-Wasm inputs. Full Wasm validation is the IC's responsibility.
pub fn inspect(path: &Path) -> Result<Artifact> {
    // Inspection follows caller-selected aliases; retained-record reads do not.
    let resolved = path.canonicalize().map_err(supplier)?;
    let bytes = crate::store::read(&resolved, MAX_ARTIFACT_BYTES)?;
    admit_bounded(&bytes, MAX_ARTIFACT_BYTES)
}

/// Build one explicit Cargo package and select its Wasm from artifact records.
/// `ic-wasm-store` uses the workspace's locked registry artifact input and its packaged lockfile.
/// An output path atomically receives a copy, replacing a prior build input.
///
/// # Errors
/// Refuses build failure, output overflow, timeout or ambiguous artifacts. Installs no tools.
pub fn build(
    manifest: &Path,
    package: &str,
    features: &[String],
    destination: Option<&Path>,
) -> Result<PathBuf> {
    Ok(build_report(manifest, package, features, destination)?.wasm)
}

/// Published path and identity of the exact bytes from this Cargo invocation.
#[derive(Debug, Serialize)]
pub struct BuildReport {
    /// Content-addressed default path or the caller's replaceable destination.
    pub wasm: PathBuf,
    /// Digest and size measured before atomic publication.
    pub artifact: Artifact,
}

/// Build and report the original artifact even if another caller replaces its destination.
/// # Errors
/// Returns the same build and publication errors as [`build`].
pub fn build_report(
    manifest: &Path,
    package: &str,
    features: &[String],
    destination: Option<&Path>,
) -> Result<BuildReport> {
    let workspace_manifest = manifest.canonicalize().map_err(supplier)?;
    let workspace = workspace_manifest
        .parent()
        .ok_or_else(|| Error::Invalid("manifest has no parent".into()))?;
    let manifest = if package == "ic-wasm-store" {
        store_source::manifest(&workspace_manifest)?
    } else {
        workspace_manifest.clone()
    };
    // Dependencies can also emit cdylib artifacts. Resolve Cargo's exact selected
    // package identity instead of treating every Wasm in the build as ours.
    let mut identity = Command::new("cargo");
    identity
        .current_dir(workspace)
        .args(["pkgid", "--locked", "--manifest-path"])
        .arg(&manifest)
        .args(["-p", package]);
    let package_id =
        crate::process::capture(&mut identity, 65536, 1024 * 1024, Duration::from_secs(60))
            .map_err(cargo_failure)?;
    let package_id = std::str::from_utf8(&package_id).map_err(supplier)?.trim();
    if package_id.is_empty() || package_id.lines().count() != 1 {
        return Err(Error::Invalid(
            "Cargo package identity is not one line".into(),
        ));
    }
    // Cargo's package/feature-specific intermediates remain cached. Its final
    // cdylib filename is shared across variants, so isolate only the final links.
    #[derive(Deserialize)]
    struct Layout {
        target_directory: PathBuf,
        build_directory: PathBuf,
        packages: Vec<source_layout::Package>,
    }
    let layout: Layout = serde_json::from_slice(
        &crate::process::capture(
            Command::new("cargo")
                .current_dir(workspace)
                .args([
                    "metadata",
                    "--locked",
                    "--no-deps",
                    "--format-version",
                    "1",
                    "--manifest-path",
                ])
                .arg(&workspace_manifest),
            8 * 1024 * 1024,
            1024 * 1024,
            Duration::from_secs(60),
        )
        .map_err(cargo_failure)?,
    )
    .map_err(supplier)?;
    if let Some(selected) = layout.packages.iter().find(|p| p.id == package_id) {
        selected.validate()?;
    }
    let cache = if package == "ic-wasm-store" {
        workspace.join("target/store-build")
    } else {
        layout.build_directory
    };
    let outputs = layout.target_directory.join("canic-artifacts");
    crate::output_publication::prepare(&outputs, 0o700)?;
    let private = tempfile::Builder::new()
        .prefix(".cargo-")
        .tempdir_in(&outputs)
        .map_err(supplier)?;
    let mut command = Command::new("cargo");
    command
        .current_dir(
            // Registry archives have no rust-toolchain.toml. Keep the consumer's
            // toolchain selection even when building the standalone Store manifest.
            workspace,
        )
        .args([
            "build",
            "--lib",
            "--locked",
            "--release",
            "--target",
            "wasm32-unknown-unknown",
            "--message-format=json-render-diagnostics",
            "--manifest-path",
        ])
        .arg(&manifest)
        .args(["-p", package]);
    command
        .arg("--target-dir")
        .arg(private.path())
        .env("CARGO_BUILD_BUILD_DIR", cache);
    if !features.is_empty() {
        command.arg("--features").arg(features.join(","));
    }
    let output = crate::process::capture(
        &mut command,
        8 * 1024 * 1024,
        1024 * 1024,
        Duration::from_secs(600),
    )
    .map_err(cargo_failure)?;
    let path = selected_wasm(&output, package_id)?;
    let bytes = crate::store::read(&path, MAX_ARTIFACT_BYTES)?;
    let artifact = admit_bounded(&bytes, MAX_ARTIFACT_BYTES)?;
    let wasm = match destination {
        Some(path) => path.to_owned(),
        None => outputs.join(&artifact.sha256).join(
            path.file_name()
                .ok_or_else(|| Error::Invalid("Cargo artifact has no filename".into()))?,
        ),
    };
    crate::output_publication::publish(
        &wasm,
        &bytes,
        ic_host_fs::Destination::Replace,
        0o700,
        0o600,
    )?;
    Ok(BuildReport { wasm, artifact })
}

fn selected_wasm(output: &[u8], package_id: &str) -> Result<PathBuf> {
    let mut paths = Vec::new();
    for line in output
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
    {
        let value: serde_json::Value = serde_json::from_slice(line).map_err(supplier)?;
        if value["reason"] == "compiler-artifact" && value["package_id"] == package_id {
            let files = value["filenames"]
                .as_array()
                .ok_or_else(|| Error::Invalid("Cargo artifact missing filenames".into()))?;
            for file in files {
                let file = file
                    .as_str()
                    .ok_or_else(|| Error::Invalid("Cargo filename is not text".into()))?;
                if file.ends_with(".wasm") {
                    paths.push(PathBuf::from(file));
                }
            }
        }
    }
    paths.sort();
    paths.dedup();
    if paths.len() != 1 {
        return Err(Error::Invalid(
            "build must produce exactly one Wasm artifact".into(),
        ));
    }
    Ok(paths.remove(0))
}
