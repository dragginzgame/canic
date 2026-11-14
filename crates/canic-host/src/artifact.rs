use crate::{Error, Result, hex, supplier};
use ic_host_process::tool::{OutputLimit, OutputLimits, capture_group_command};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

pub(crate) const MAX_WASM: usize = 1_500_000;

/// Identity of the exact uncompressed Wasm bytes selected for installation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    /// SHA-256 of the complete module.
    pub sha256: String,
    /// Complete uncompressed module length.
    pub bytes: usize,
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

pub(crate) fn admit(bytes: &[u8]) -> Result<Artifact> {
    if bytes.len() > MAX_WASM || !bytes.starts_with(b"\0asm\x01\0\0\0") {
        return Err(Error::Invalid(
            "expected uncompressed Wasm v1, at most 1,500,000 bytes".into(),
        ));
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
    admit(&ic_host_fs::read::read_file(path, MAX_WASM).map_err(supplier)?)
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
    let mut command = Command::new("cargo");
    command
        .args([
            "build",
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
    .map_err(supplier)?
    .require_success()
    .map_err(supplier)?
    .require_complete()
    .map_err(supplier)?;
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
        let bytes = ic_host_fs::read::read_file(&path, MAX_WASM).map_err(supplier)?;
        admit(&bytes)?;
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
