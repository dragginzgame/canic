//! Stable source, Cargo, and artifact provenance for build outputs.

use crate::{
    canister_build::{CanisterArtifactBuildOutput, CanisterBuildProfile},
    cargo_command,
    evidence_envelope::{
        CommandProvenanceV1, EvidenceEnvelopeV1, EvidenceMessageSeverityV1, EvidenceMessageV1,
        EvidenceSummaryV1, EvidenceTargetKindV1, EvidenceTargetV1, ExitClassV1, InputFingerprintV1,
        InputPathDisplayV1, PayloadSchemaRefV1, evidence_envelope_schema, file_input_fingerprint,
        json_payload_sha256, sha256_hex,
    },
    release_set::canister_manifest_path,
};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
use toml::Value as TomlValue;

pub const BUILD_PROVENANCE_SCHEMA_ID: &str = "canic.build_provenance.v1";
const WASM_TARGET: &str = "wasm32-unknown-unknown";
const DIRTY_SUMMARY_ALGORITHM: &str = "git-status-porcelain-v1-z-sha256";

///
/// BuildProvenanceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BuildProvenanceV1 {
    pub schema_version: u8,
    pub generated_at: String,
    pub canic_version: String,
    pub command: CommandProvenanceV1,
    pub build_status: BuildProvenanceStatusV1,
    pub source: SourceProvenanceV1,
    pub cargo: CargoProvenanceV1,
    pub artifacts: Vec<ArtifactProvenanceV1>,
    pub warnings: Vec<EvidenceMessageV1>,
}

///
/// BuildProvenanceStatusV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildProvenanceStatusV1 {
    Success,
    Failed,
    NotRecorded,
}

///
/// SourceProvenanceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceProvenanceV1 {
    pub schema_version: u8,
    pub vcs: SourceVcsV1,
    pub revision: Option<String>,
    pub branch: Option<String>,
    pub dirty: Option<bool>,
    pub dirty_policy: SourceDirtyPolicyV1,
    pub dirty_summary_digest: Option<String>,
    pub dirty_summary_algorithm: Option<String>,
}

///
/// SourceVcsV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceVcsV1 {
    Git,
    Unknown,
}

///
/// SourceDirtyPolicyV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceDirtyPolicyV1 {
    Clean,
    DirtyRecorded,
    Unknown,
}

///
/// CargoProvenanceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CargoProvenanceV1 {
    pub cargo_lock_sha256: Option<String>,
    pub package_manifest_sha256: Option<String>,
    pub package_name: String,
    pub package_manifest: String,
    pub package_metadata_fleet: String,
    pub package_metadata_role: String,
    pub rustc_version: Option<String>,
    pub cargo_version: Option<String>,
    pub target: Option<String>,
    pub profile: String,
    pub features: Vec<String>,
    pub default_features: Option<bool>,
    pub rustflags_digest: Option<String>,
    pub rustflags_digest_algorithm: Option<String>,
    pub cargo_config_fingerprints: Vec<InputFingerprintV1>,
    pub build_script_inputs: BuildScriptInputStateV1,
}

///
/// BuildScriptInputStateV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildScriptInputStateV1 {
    NotRecorded,
    Recorded,
    Unknown,
}

///
/// ArtifactProvenanceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactProvenanceV1 {
    pub role: String,
    pub fleet: String,
    pub artifact_kind: ArtifactProvenanceKindV1,
    pub path: Option<String>,
    pub path_display: InputPathDisplayV1,
    pub hash_algorithm: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub produced_by: String,
}

///
/// ArtifactProvenanceKindV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactProvenanceKindV1 {
    Wasm,
    WasmGzip,
    Candid,
    Metadata,
    Other,
}

///
/// BuildProvenanceRequest
///
#[derive(Clone, Debug)]
pub struct BuildProvenanceRequest {
    pub fleet: String,
    pub role: String,
    pub network: String,
    pub profile: CanisterBuildProfile,
    pub workspace_root: PathBuf,
    pub config_path: PathBuf,
    pub output: CanisterArtifactBuildOutput,
    pub command: CommandProvenanceV1,
    pub generated_at: String,
    pub canic_version: String,
}

#[must_use]
pub fn build_provenance_schema() -> PayloadSchemaRefV1 {
    PayloadSchemaRefV1::stable(BUILD_PROVENANCE_SCHEMA_ID, "1")
}

pub fn build_provenance_envelope(
    request: &BuildProvenanceRequest,
) -> Result<EvidenceEnvelopeV1, Box<dyn std::error::Error>> {
    let payload = build_provenance_payload(request)?;
    let payload_sha256 = Some(json_payload_sha256(&payload)?);
    let payload_value = serde_json::to_value(&payload)?;
    let summary = EvidenceSummaryV1 {
        warnings: payload.warnings.clone(),
        blocked_actions: Vec::new(),
        missing_or_stale_evidence: Vec::new(),
        evidence_conflicts: Vec::new(),
    };
    let generated_at = payload.generated_at;
    let exit_class = if summary.warnings.is_empty() {
        ExitClassV1::Success
    } else {
        ExitClassV1::SuccessWithWarnings
    };

    Ok(EvidenceEnvelopeV1 {
        envelope_schema: evidence_envelope_schema(),
        canic_version: request.canic_version.clone(),
        command: request.command.clone(),
        target: EvidenceTargetV1 {
            kind: EvidenceTargetKindV1::Artifact,
            deployment: None,
            fleet: Some(request.fleet.clone()),
            role: Some(request.role.clone()),
            profile: Some(request.profile.target_dir_name().to_string()),
            network: Some(request.network.clone()),
        },
        generated_at,
        source_config: Some(file_input_fingerprint(
            "canic_config",
            &request.config_path,
            &request.workspace_root,
            Some(PayloadSchemaRefV1::internal("canic.config.toml", "1")),
            None,
        )?),
        inputs: build_input_fingerprints(request)?,
        payload_schema: build_provenance_schema(),
        payload_sha256,
        payload: payload_value,
        summary,
        exit_class,
    })
}

pub fn build_provenance_payload(
    request: &BuildProvenanceRequest,
) -> Result<BuildProvenanceV1, Box<dyn std::error::Error>> {
    let mut warnings = Vec::new();
    let source = source_provenance(&request.workspace_root);
    if source.dirty == Some(true) {
        warnings.push(EvidenceMessageV1::new(
            "build_provenance.source_dirty",
            "build used uncommitted local source state",
            EvidenceMessageSeverityV1::Warning,
        ));
    }
    if source.vcs == SourceVcsV1::Unknown {
        warnings.push(EvidenceMessageV1::new(
            "build_provenance.source_unknown",
            "source revision could not be read from git",
            EvidenceMessageSeverityV1::Warning,
        ));
    }

    Ok(BuildProvenanceV1 {
        schema_version: 1,
        generated_at: request.generated_at.clone(),
        canic_version: request.canic_version.clone(),
        command: request.command.clone(),
        build_status: BuildProvenanceStatusV1::Success,
        source,
        cargo: cargo_provenance(request)?,
        artifacts: artifact_provenance(request)?,
        warnings,
    })
}

fn source_provenance(workspace_root: &Path) -> SourceProvenanceV1 {
    let Some(revision) = git_output_text(workspace_root, ["rev-parse", "HEAD"]) else {
        return unknown_source_provenance();
    };
    let branch = git_output_text(workspace_root, ["rev-parse", "--abbrev-ref", "HEAD"]);
    let Some(status) = git_output_bytes(workspace_root, ["status", "--porcelain=v1", "-z"]) else {
        return SourceProvenanceV1 {
            schema_version: 1,
            vcs: SourceVcsV1::Git,
            revision: Some(revision),
            branch,
            dirty: None,
            dirty_policy: SourceDirtyPolicyV1::Unknown,
            dirty_summary_digest: None,
            dirty_summary_algorithm: None,
        };
    };

    let dirty = !status.is_empty();
    SourceProvenanceV1 {
        schema_version: 1,
        vcs: SourceVcsV1::Git,
        revision: Some(revision),
        branch,
        dirty: Some(dirty),
        dirty_policy: if dirty {
            SourceDirtyPolicyV1::DirtyRecorded
        } else {
            SourceDirtyPolicyV1::Clean
        },
        dirty_summary_digest: dirty.then(|| sha256_hex(&status)),
        dirty_summary_algorithm: dirty.then(|| DIRTY_SUMMARY_ALGORITHM.to_string()),
    }
}

const fn unknown_source_provenance() -> SourceProvenanceV1 {
    SourceProvenanceV1 {
        schema_version: 1,
        vcs: SourceVcsV1::Unknown,
        revision: None,
        branch: None,
        dirty: None,
        dirty_policy: SourceDirtyPolicyV1::Unknown,
        dirty_summary_digest: None,
        dirty_summary_algorithm: None,
    }
}

fn cargo_provenance(
    request: &BuildProvenanceRequest,
) -> Result<CargoProvenanceV1, Box<dyn std::error::Error>> {
    let package_manifest = canister_manifest_path(&request.workspace_root, &request.role)?;
    let manifest_source = fs::read_to_string(&package_manifest)?;
    let manifest = toml::from_str::<TomlValue>(&manifest_source)?;
    let cargo_lock_path = request.workspace_root.join("Cargo.lock");
    let package_metadata_fleet = required_manifest_str(
        &manifest,
        &["package", "metadata", "canic", "fleet"],
        &package_manifest,
    )?;
    let package_metadata_role = required_manifest_str(
        &manifest,
        &["package", "metadata", "canic", "role"],
        &package_manifest,
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
        package_name: required_manifest_str(&manifest, &["package", "name"], &package_manifest)?,
        package_manifest: display_path(&package_manifest, &request.workspace_root),
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

fn artifact_provenance(
    request: &BuildProvenanceRequest,
) -> Result<Vec<ArtifactProvenanceV1>, Box<dyn std::error::Error>> {
    let mut artifacts = Vec::new();
    push_artifact(
        &mut artifacts,
        request,
        ArtifactProvenanceKindV1::Wasm,
        &request.output.wasm_path,
    )?;
    push_artifact(
        &mut artifacts,
        request,
        ArtifactProvenanceKindV1::WasmGzip,
        &request.output.wasm_gz_path,
    )?;
    push_existing_artifact(
        &mut artifacts,
        request,
        ArtifactProvenanceKindV1::Candid,
        &request.output.did_path,
    )?;
    if let Some(path) = &request.output.manifest_path {
        push_existing_artifact(
            &mut artifacts,
            request,
            ArtifactProvenanceKindV1::Metadata,
            path,
        )?;
    }

    Ok(artifacts)
}

fn push_existing_artifact(
    artifacts: &mut Vec<ArtifactProvenanceV1>,
    request: &BuildProvenanceRequest,
    kind: ArtifactProvenanceKindV1,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if path.is_file() {
        push_artifact(artifacts, request, kind, path)?;
    }
    Ok(())
}

fn push_artifact(
    artifacts: &mut Vec<ArtifactProvenanceV1>,
    request: &BuildProvenanceRequest,
    kind: ArtifactProvenanceKindV1,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let fingerprint =
        file_input_fingerprint("build_artifact", path, &request.workspace_root, None, None)?;
    artifacts.push(ArtifactProvenanceV1 {
        role: request.role.clone(),
        fleet: request.fleet.clone(),
        artifact_kind: kind,
        path: fingerprint.path,
        path_display: fingerprint.path_display,
        hash_algorithm: "sha256".to_string(),
        sha256: fingerprint
            .sha256
            .ok_or_else(|| format!("missing sha256 for {}", path.display()))?,
        size_bytes: fingerprint
            .size_bytes
            .ok_or_else(|| format!("missing size for {}", path.display()))?,
        produced_by: "canic build".to_string(),
    });
    Ok(())
}

fn build_input_fingerprints(
    request: &BuildProvenanceRequest,
) -> Result<Vec<InputFingerprintV1>, Box<dyn std::error::Error>> {
    let package_manifest = canister_manifest_path(&request.workspace_root, &request.role)?;
    let mut inputs = vec![file_input_fingerprint(
        "cargo_package_manifest",
        &package_manifest,
        &request.workspace_root,
        Some(PayloadSchemaRefV1::internal(
            "cargo.package_manifest.toml",
            "1",
        )),
        None,
    )?];
    let cargo_lock_path = request.workspace_root.join("Cargo.lock");
    if cargo_lock_path.is_file() {
        inputs.push(file_input_fingerprint(
            "cargo_lock",
            &cargo_lock_path,
            &request.workspace_root,
            Some(PayloadSchemaRefV1::internal("cargo.lock", "1")),
            None,
        )?);
    }
    inputs.extend(cargo_config_fingerprints(&request.workspace_root)?);
    Ok(inputs)
}

fn cargo_config_fingerprints(
    workspace_root: &Path,
) -> Result<Vec<InputFingerprintV1>, Box<dyn std::error::Error>> {
    [".cargo/config.toml", ".cargo/config"]
        .into_iter()
        .map(|relative| workspace_root.join(relative))
        .filter(|path| path.is_file())
        .map(|path| {
            Ok(file_input_fingerprint(
                "cargo_config",
                &path,
                workspace_root,
                Some(PayloadSchemaRefV1::internal("cargo.config.toml", "1")),
                None,
            )?)
        })
        .collect()
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

fn git_output_text<const N: usize>(workspace_root: &Path, args: [&str; N]) -> Option<String> {
    String::from_utf8(git_output_bytes(workspace_root, args)?)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn git_output_bytes<const N: usize>(workspace_root: &Path, args: [&str; N]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .current_dir(workspace_root)
        .args(args)
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    #[test]
    fn build_provenance_schema_is_stable() {
        assert_eq!(
            build_provenance_schema(),
            PayloadSchemaRefV1::stable("canic.build_provenance.v1", "1")
        );
    }

    #[test]
    fn unknown_source_provenance_is_explicit() {
        let root = temp_dir("canic-build-provenance-no-git");
        fs::create_dir_all(&root).expect("create root");

        let provenance = source_provenance(&root);

        fs::remove_dir_all(&root).expect("remove root");
        assert_eq!(provenance.vcs, SourceVcsV1::Unknown);
        assert_eq!(provenance.dirty_policy, SourceDirtyPolicyV1::Unknown);
    }

    #[test]
    fn artifact_provenance_records_wasm_and_gzip_separately() {
        let root = temp_dir("canic-build-provenance-artifacts");
        let artifact_root = root.join(".icp/local/canisters/app");
        fs::create_dir_all(&artifact_root).expect("create artifacts");
        let wasm_path = artifact_root.join("app.wasm");
        let wasm_gz_path = artifact_root.join("app.wasm.gz");
        let did_path = artifact_root.join("app.did");
        fs::write(&wasm_path, b"wasm").expect("write wasm");
        fs::write(&wasm_gz_path, b"gzip").expect("write gzip");

        let request = sample_request(
            &root,
            CanisterArtifactBuildOutput {
                artifact_root,
                wasm_path,
                wasm_gz_path,
                did_path,
                manifest_path: None,
            },
        );
        let artifacts = artifact_provenance(&request).expect("artifact provenance");

        fs::remove_dir_all(&root).expect("remove root");
        assert_eq!(artifacts.len(), 2);
        assert_eq!(artifacts[0].artifact_kind, ArtifactProvenanceKindV1::Wasm);
        assert_eq!(
            artifacts[1].artifact_kind,
            ArtifactProvenanceKindV1::WasmGzip
        );
        assert_ne!(artifacts[0].sha256, artifacts[1].sha256);
    }

    #[test]
    fn build_provenance_envelope_wraps_stable_payload() {
        let root = temp_dir("canic-build-provenance-envelope");
        write_sample_workspace(&root, "demo", "app");
        let output = write_sample_artifacts(&root, "app");
        let request = BuildProvenanceRequest {
            fleet: "demo".to_string(),
            role: "app".to_string(),
            network: "local".to_string(),
            profile: CanisterBuildProfile::Fast,
            workspace_root: root.clone(),
            config_path: root.join("fleets/demo/canic.toml"),
            output,
            command: sample_command(),
            generated_at: "unix:1".to_string(),
            canic_version: "0.0.0-test".to_string(),
        };

        let envelope = build_provenance_envelope(&request).expect("build envelope");
        let payload = serde_json::from_value::<BuildProvenanceV1>(envelope.payload.clone())
            .expect("decode payload");

        fs::remove_dir_all(&root).expect("remove root");
        assert_eq!(envelope.target.kind, EvidenceTargetKindV1::Artifact);
        assert_eq!(envelope.target.fleet.as_deref(), Some("demo"));
        assert_eq!(envelope.target.role.as_deref(), Some("app"));
        assert_eq!(envelope.payload_schema, build_provenance_schema());
        assert_eq!(payload.cargo.package_metadata_fleet, "demo");
        assert_eq!(payload.cargo.package_metadata_role, "app");
        assert!(payload.cargo.cargo_lock_sha256.is_some());
        assert_eq!(payload.artifacts.len(), 2);
    }

    fn sample_request(root: &Path, output: CanisterArtifactBuildOutput) -> BuildProvenanceRequest {
        BuildProvenanceRequest {
            fleet: "demo".to_string(),
            role: "app".to_string(),
            network: "local".to_string(),
            profile: CanisterBuildProfile::Fast,
            workspace_root: root.to_path_buf(),
            config_path: root.join("fleets/demo/canic.toml"),
            output,
            command: sample_command(),
            generated_at: "unix:1".to_string(),
            canic_version: "0.0.0-test".to_string(),
        }
    }

    fn sample_command() -> CommandProvenanceV1 {
        CommandProvenanceV1 {
            name: "canic build".to_string(),
            argv_normalized: vec!["canic".to_string(), "build".to_string()],
            argv_redactions: Vec::new(),
            format: "provenance".to_string(),
        }
    }

    fn write_sample_workspace(root: &Path, fleet: &str, role: &str) {
        let package_dir = root.join("fleets").join(fleet).join(role);
        fs::create_dir_all(package_dir.join("src")).expect("create package");
        fs::write(
            root.join("Cargo.toml"),
            format!(
                r#"[workspace]
members = ["fleets/{fleet}/{role}"]
resolver = "3"
"#
            ),
        )
        .expect("write workspace manifest");
        fs::write(root.join("Cargo.lock"), "# lock\n").expect("write lock");
        fs::write(
            root.join("fleets").join(fleet).join("canic.toml"),
            format!(
                r#"[fleet]
name = "{fleet}"

[roles.{role}]
kind = "canister"
package = "{role}"

[subnets.prime.canisters.{role}]
kind = "singleton"
"#
            ),
        )
        .expect("write canic config");
        fs::write(
            package_dir.join("Cargo.toml"),
            format!(
                r#"[package]
name = "canister_{fleet}_{role}"
version = "0.0.0"
edition = "2024"

[package.metadata.canic]
fleet = "{fleet}"
role = "{role}"
"#
            ),
        )
        .expect("write package manifest");
        fs::write(package_dir.join("src/lib.rs"), "").expect("write lib");
    }

    fn write_sample_artifacts(root: &Path, role: &str) -> CanisterArtifactBuildOutput {
        let artifact_root = root.join(".icp/local/canisters").join(role);
        fs::create_dir_all(&artifact_root).expect("create artifacts");
        let wasm_path = artifact_root.join(format!("{role}.wasm"));
        let wasm_gz_path = artifact_root.join(format!("{role}.wasm.gz"));
        let did_path = artifact_root.join(format!("{role}.did"));
        fs::write(&wasm_path, b"wasm").expect("write wasm");
        fs::write(&wasm_gz_path, b"gzip").expect("write gzip");

        CanisterArtifactBuildOutput {
            artifact_root,
            wasm_path,
            wasm_gz_path,
            did_path,
            manifest_path: None,
        }
    }
}
