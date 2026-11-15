use crate::{Error, Result, hex, supplier};
use ic_host_process::tool::{
    CompletedExecution, ExecutionEvidence, OutputLimit, OutputLimits, ToolError,
    capture_group_command,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

// Direct ingress installation retains its original transport bound.
pub(crate) const MAX_WASM: usize = 1_500_000;
// Build/inspection can admit the complete Store artifact without granting installation authority.
pub(crate) const MAX_ARTIFACT_BYTES: usize =
    canic_wasm_store_protocol::CHUNK_BYTES * canic_wasm_store_protocol::MAX_CHUNKS;

/// Identity of the exact uncompressed Wasm bytes selected for installation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    /// SHA-256 of the complete module.
    pub sha256: String,
    /// Complete uncompressed module length.
    pub bytes: usize,
}

#[derive(Debug, thiserror::Error)]
#[error("Cargo failed: {source}\n{diagnostic}")]
struct CargoFailure {
    source: ToolError,
    diagnostic: String,
}

// Only Cargo callers expose captured diagnostics. Signer/provider evidence keeps
// its existing disclosure policy; the process supplier owns capture and cleanup.
pub(crate) fn cargo_failure(source: ToolError) -> Error {
    match source.evidence().and_then(cargo_diagnostic) {
        Some(diagnostic) => supplier(CargoFailure { source, diagnostic }),
        None => supplier(source),
    }
}

fn cargo_diagnostic(evidence: &ExecutionEvidence) -> Option<String> {
    let compiler = evidence.stdout.split(|b| *b == b'\n').find_map(|line| {
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
    let text = compiler.unwrap_or_else(|| String::from_utf8_lossy(&evidence.stderr).into_owned());
    let mut characters = text.trim().chars();
    let prefix: String = characters.by_ref().take(4096).collect();
    if prefix.is_empty() {
        return None;
    }
    let mut diagnostic = prefix
        .split('\n')
        .map(ic_host_text::escape::terminal_text)
        .collect::<Vec<_>>()
        .join("\n");
    if characters.next().is_some() || evidence.stdout_truncated || evidence.stderr_truncated {
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
    admit_bounded(
        &ic_host_fs::read::read_file(path, MAX_ARTIFACT_BYTES).map_err(supplier)?,
        MAX_ARTIFACT_BYTES,
    )
}

/// Build one explicit Cargo package and select its Wasm from artifact records.
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
    let manifest = manifest.canonicalize().map_err(supplier)?;
    let mut command = Command::new("cargo");
    command
        .current_dir(
            manifest
                .parent()
                .ok_or_else(|| Error::Invalid("manifest has no parent".into()))?,
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
        .arg(manifest)
        .args(["-p", package]);
    if !features.is_empty() {
        command.arg("--features").arg(features.join(","));
    }
    let output = capture_group_command(
        &mut command,
        OutputLimits {
            stdout: OutputLimit::Terminate(8 * 1024 * 1024),
            stderr: OutputLimit::Terminate(1024 * 1024),
            timeout: Some(Duration::from_secs(600)),
        },
    )
    .and_then(CompletedExecution::require_success)
    .and_then(CompletedExecution::require_complete)
    .map_err(cargo_failure)?;
    let mut paths = Vec::new();
    for line in output
        .stdout
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
    {
        let value: serde_json::Value = serde_json::from_slice(line).map_err(supplier)?;
        if value["reason"] == "compiler-artifact" {
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
    let path = paths.remove(0);
    inspect(&path)?;
    if let Some(output) = destination {
        let bytes = ic_host_fs::read::read_file(&path, MAX_ARTIFACT_BYTES).map_err(supplier)?;
        admit_bounded(&bytes, MAX_ARTIFACT_BYTES)?;
        if let Some(parent) = output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            ic_host_fs::durable::prepare_private_parents(parent).map_err(supplier)?;
        }
        ic_host_fs::durable::write_bytes(
            output,
            ic_host_fs::durable::WriteOptions {
                mode: ic_host_fs::durable::PublicationMode::Replace,
                permissions: 0o600,
            },
            &bytes,
        )
        .map_err(supplier)?;
        return Ok(output.into());
    }
    Ok(path)
}

#[cfg(test)]
mod tests;
