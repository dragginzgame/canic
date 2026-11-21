mod source_layout;
mod supplier_source;
pub(crate) use supplier_source::manifest as build_manifest;
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
    check_wasm_header_and_size(bytes, limit)?;
    Ok(Artifact {
        sha256: digest(bytes),
        bytes: bytes.len(),
    })
}

pub(crate) fn check_wasm_header_and_size(bytes: &[u8], limit: usize) -> Result<()> {
    if bytes.len() > limit || !bytes.starts_with(b"\0asm\x01\0\0\0") {
        return Err(Error::Invalid(format!(
            "expected uncompressed Wasm v1, at most {limit} bytes"
        )));
    }
    Ok(())
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

/// A callable method name and its IC export mode.
#[derive(Debug, Serialize)]
pub struct WasmEndpoint {
    /// Public IC method name.
    pub name: String,
    /// One of query, composite_query or update.
    pub mode: &'static str,
}

/// Read callable function exports in name order without executing the module.
/// # Errors
/// Refuses unreadable, oversized or malformed export sections.
pub fn endpoints(path: &Path) -> Result<Vec<WasmEndpoint>> {
    let bytes = crate::store::read(&path.canonicalize().map_err(supplier)?, MAX_ARTIFACT_BYTES)?;
    exported_methods(&bytes)
}

pub(crate) fn exported_methods(bytes: &[u8]) -> Result<Vec<WasmEndpoint>> {
    use wasmparser::{ExternalKind, Parser, Payload};
    admit_bounded(bytes, MAX_ARTIFACT_BYTES)?;
    let mut methods = Vec::new();
    for payload in Parser::new(0).parse_all(bytes) {
        let Payload::ExportSection(exports) = payload.map_err(supplier)? else {
            continue;
        };
        for export in exports {
            let export = export.map_err(supplier)?;
            if export.kind != ExternalKind::Func {
                continue;
            }
            let Some((prefix, name)) = export.name.split_once(' ') else {
                continue;
            };
            let mode = match prefix {
                "canister_query" => "query",
                "canister_update" => "update",
                "canister_composite_query" => "composite_query",
                _ => continue,
            };
            if !name.is_empty() {
                methods.push(WasmEndpoint {
                    name: name.to_owned(),
                    mode,
                });
            }
        }
    }
    methods.sort_by(|a, b| (&a.name, a.mode).cmp(&(&b.name, b.mode)));
    Ok(methods)
}

/// Build one explicit Cargo package and select its Wasm from artifact records.
/// The published Root and Store use the selected registry archives and their packaged lockfiles.
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
    #[serde(skip)]
    pub(crate) modules: Vec<&'static str>,
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
    let source = build_manifest(manifest, package)?;
    build_for_role(
        manifest,
        &source,
        package,
        features,
        destination,
        None,
        None,
    )
}

pub(crate) fn build_for_role(
    workspace_manifest: &Path,
    manifest: &Path,
    package: &str,
    features: &[String],
    destination: Option<&Path>,
    role: Option<&str>,
    children: Option<&str>,
) -> Result<BuildReport> {
    let workspace_manifest = workspace_manifest.canonicalize().map_err(supplier)?;
    let workspace = workspace_manifest
        .parent()
        .ok_or_else(|| Error::Invalid("manifest has no parent".into()))?;
    let manifest = manifest.canonicalize().map_err(supplier)?;
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
    // Strip large Root builds before hashing for direct install. Other Roots retain
    // symbolized traps for diagnosing refused stable-layout transitions.
    if package == "canic-root"
        && (features.iter().any(|f| f == "blob" || f == "role-issuer")
            || (features.iter().any(|f| f == "auth") && features.iter().any(|f| f == "sharding")))
    {
        command.args([
            "--config",
            "profile.release.package.canic-root.strip='symbols'",
            "--config",
            "profile.release.package.canic-root.opt-level='s'",
            "--config",
            "profile.release.package.canic-management.opt-level='s'",
        ]);
    }
    let child_input = format!("CANIC_APP_CHILDREN_{package}");
    if let Some(names) = children {
        command.env(child_input, names);
    } else {
        command.env_remove(child_input);
    }
    let role_input = format!("CANIC_APP_ROLE_{package}");
    if let Some(name) = role {
        command.env(role_input, name);
    } else {
        command.env_remove(role_input);
    }
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
    let modules = compiled_modules(
        &bytes,
        features
            .iter()
            .any(|feature| feature == "canic/endpoint-metrics"),
    )?;
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
    Ok(BuildReport {
        wasm,
        artifact,
        modules,
    })
}

// Verify the selected metrics build before publishing the exact bytes we inspected.
fn compiled_modules(bytes: &[u8], endpoint_metrics: bool) -> Result<Vec<&'static str>> {
    use wasmparser::{ExternalKind, Parser, Payload};

    let mut has_db = false;
    let mut has_auth = false;
    let mut has_blob = false;
    let mut mount = false;
    let mut query = false;
    for payload in Parser::new(0).parse_all(bytes) {
        match payload.map_err(supplier)? {
            Payload::CustomSection(section) => match section.name() {
                "canic:module:db" => has_db = true,
                "canic:module:auth" => has_auth = true,
                "canic:module:blob" => has_blob = true,
                "canic:module:endpoint-metrics" => mount = true,
                _ => {}
            },
            Payload::ExportSection(exports) => {
                for export in exports {
                    let export = export.map_err(supplier)?;
                    query |= export.kind == ExternalKind::Func
                        && export.name == "canister_query metrics";
                }
            }
            _ => {}
        }
    }
    if endpoint_metrics && !(mount && query) {
        return Err(Error::Invalid("canic/endpoint-metrics was selected but the built Wasm is missing the Canic metrics marker or shared metrics query".into()));
    }
    if !endpoint_metrics && mount {
        return Err(Error::Invalid("endpoint metrics compiled without an explicit selection; select canic/endpoint-metrics in the build recipe or remove the dependency feature".into()));
    }
    Ok([
        ("DB", has_db),
        ("Auth", has_auth),
        ("Blob", has_blob),
        ("Endpoint Metrics", mount),
    ]
    .into_iter()
    .filter_map(|(name, present)| present.then_some(name))
    .collect())
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
