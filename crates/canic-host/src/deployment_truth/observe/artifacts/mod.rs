use super::super::*;
use super::shared::observation_gap;
use crate::release_set::{
    AppConfigSnapshot, ROOT_RELEASE_SET_MANIFEST_FILE, artifact_root_path,
    load_root_release_set_manifest, resolve_artifact_root,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

///
/// LocalArtifactManifestRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalArtifactManifestRequest {
    pub environment: String,
    pub artifact_environment: String,
    pub workspace_root: PathBuf,
    pub icp_root: PathBuf,
    pub config_path: Option<PathBuf>,
}

/// Collect a read-only manifest of locally materialized role artifacts.
#[must_use]
pub fn collect_local_role_artifact_manifest(
    request: &LocalArtifactManifestRequest,
) -> RoleArtifactManifestV1 {
    let config = deployment_config_path(&request.workspace_root, request.config_path.as_deref());
    let mut unresolved_artifacts = Vec::new();
    let (fleet_name, roles) = match AppConfigSnapshot::load(&config) {
        Ok(snapshot) => (
            snapshot.app_id().to_string(),
            deployment_truth_roles_with_implicit_wasm_store(snapshot.deployable_roles()),
        ),
        Err(err) => {
            for (code, subject) in [
                ("local_config.fleet_name", "fleet name"),
                ("local_config.roles", "configured roles"),
            ] {
                unresolved_artifacts.push(observation_gap(
                    code,
                    format!(
                        "could not resolve {subject} from {}: {err}",
                        config.display()
                    ),
                ));
            }
            ("unknown".to_string(), Vec::new())
        }
    };
    let projected_artifact_root =
        artifact_root_path(&request.icp_root, &request.artifact_environment);
    let artifact_root =
        match resolve_artifact_root(&request.icp_root, &request.artifact_environment) {
            Ok(root) => Some(root),
            Err(err) => {
                unresolved_artifacts.push(observation_gap(
                    "local_artifacts.root",
                    format!(
                        "could not resolve artifact root for environment {}: {err}",
                        request.artifact_environment
                    ),
                ));
                None
            }
        };
    let release_entries = artifact_root
        .as_ref()
        .and_then(|root| load_release_entries(root, &mut unresolved_artifacts));
    let role_artifacts = roles
        .iter()
        .map(|role| {
            role_artifact_from_local_files(
                &projected_artifact_root,
                role,
                release_entries.as_ref(),
                &mut unresolved_artifacts,
            )
        })
        .collect();

    RoleArtifactManifestV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        manifest_id: format!("local:{}:{fleet_name}:artifacts", request.environment),
        environment: request.environment.clone(),
        artifact_root: artifact_root.map(|root| root.display().to_string()),
        role_artifacts,
        unresolved_artifacts,
    }
}

pub(super) fn collect_observed_artifacts(
    icp_root: &Path,
    artifact_environment: &str,
    roles: &[String],
    unresolved_observations: &mut Vec<DeploymentObservationGapV1>,
) -> Vec<ObservedArtifactV1> {
    let artifact_root = match resolve_artifact_root(icp_root, artifact_environment) {
        Ok(root) => root,
        Err(err) => {
            unresolved_observations.push(observation_gap(
                "local_artifacts.root",
                format!(
                    "could not resolve artifact root for environment {artifact_environment}: {err}"
                ),
            ));
            return Vec::new();
        }
    };

    roles
        .iter()
        .filter_map(|role| {
            let path = artifact_root.join(role).join(format!("{role}.wasm.gz"));
            if !path.is_file() {
                unresolved_observations.push(observation_gap(
                    format!("local_artifacts.{role}"),
                    format!("missing built artifact {}", path.display()),
                ));
                return None;
            }
            let size = fs::metadata(&path).ok().map(|metadata| metadata.len());
            let file_sha256 = observe_file_sha256(&path, role, unresolved_observations);
            let file_sha256_source = file_sha256
                .as_ref()
                .map(|_| ArtifactDigestSourceV1::ObservedFileDigest);
            Some(ObservedArtifactV1 {
                role: role.clone(),
                artifact_path: path.display().to_string(),
                file_sha256,
                file_sha256_source,
                payload_sha256: None,
                payload_size_bytes: size,
                source: deployment_truth_artifact_source(role),
            })
        })
        .collect()
}

fn load_release_entries(
    artifact_root: &Path,
    unresolved_artifacts: &mut Vec<DeploymentObservationGapV1>,
) -> Option<BTreeMap<String, crate::release_set::ReleaseSetEntry>> {
    let manifest_path = artifact_root
        .join("root")
        .join(ROOT_RELEASE_SET_MANIFEST_FILE);
    if !manifest_path.is_file() {
        unresolved_artifacts.push(observation_gap(
            "local_artifacts.release_set_manifest",
            format!("missing release-set manifest {}", manifest_path.display()),
        ));
        return None;
    }
    match load_root_release_set_manifest(&manifest_path) {
        Ok(manifest) => Some(
            manifest
                .entries
                .into_iter()
                .map(|entry| (entry.role.clone(), entry))
                .collect(),
        ),
        Err(err) => {
            unresolved_artifacts.push(observation_gap(
                "local_artifacts.release_set_manifest",
                format!(
                    "could not read release-set manifest {}: {err}",
                    manifest_path.display()
                ),
            ));
            None
        }
    }
}

pub(in crate::deployment_truth) fn release_set_manifest_digest(
    icp_root: &Path,
    artifact_environment: &str,
    gaps: &mut Vec<DeploymentObservationGapV1>,
) -> Option<String> {
    let artifact_root = match resolve_artifact_root(icp_root, artifact_environment) {
        Ok(root) => root,
        Err(err) => {
            gaps.push(observation_gap(
                "deployment_manifest.path",
                format!("could not resolve release-set manifest root: {err}"),
            ));
            return None;
        }
    };
    let manifest_path = artifact_root
        .join("root")
        .join(ROOT_RELEASE_SET_MANIFEST_FILE);
    if !manifest_path.is_file() {
        gaps.push(observation_gap(
            "deployment_manifest.path",
            format!("missing release-set manifest {}", manifest_path.display()),
        ));
        return None;
    }

    match file_sha256_hex(&manifest_path) {
        Ok(hash) => Some(hash),
        Err(err) => {
            gaps.push(observation_gap(
                "deployment_manifest.sha256",
                format!(
                    "could not hash release-set manifest {}: {err}",
                    manifest_path.display()
                ),
            ));
            None
        }
    }
}

fn role_artifact_from_local_files(
    artifact_root: &Path,
    role: &str,
    release_entries: Option<&BTreeMap<String, crate::release_set::ReleaseSetEntry>>,
    unresolved_artifacts: &mut Vec<DeploymentObservationGapV1>,
) -> RoleArtifactV1 {
    let wasm_gz_path = artifact_root.join(role).join(format!("{role}.wasm.gz"));
    let (wasm_gz_size_bytes, observed_wasm_gz_file_sha256) = if wasm_gz_path.is_file() {
        (
            fs::metadata(&wasm_gz_path)
                .ok()
                .map(|metadata| metadata.len()),
            observe_file_sha256(&wasm_gz_path, role, unresolved_artifacts),
        )
    } else {
        unresolved_artifacts.push(observation_gap(
            format!("local_artifacts.{role}"),
            format!("missing built artifact {}", wasm_gz_path.display()),
        ));
        (None, None)
    };
    let observed_wasm_gz_file_sha256_source = observed_wasm_gz_file_sha256
        .as_ref()
        .map(|_| ArtifactDigestSourceV1::ObservedFileDigest);
    let release_entry = release_entries.and_then(|entries| entries.get(role));
    RoleArtifactV1 {
        role: role.to_string(),
        source: deployment_truth_artifact_source(role),
        build_profile: "unknown".to_string(),
        wasm_path: None,
        wasm_gz_path: Some(wasm_gz_path.display().to_string()),
        wasm_gz_size_bytes,
        wasm_sha256: None,
        wasm_gz_sha256: release_entry.map(|entry| entry.payload_sha256_hex.clone()),
        wasm_gz_sha256_source: release_entry.map(|_| ArtifactDigestSourceV1::ReleaseSetManifest),
        observed_wasm_gz_file_sha256,
        observed_wasm_gz_file_sha256_source,
        installed_module_hash: None,
        candid_path: None,
        candid_sha256: None,
        raw_config_sha256: None,
        canonical_embedded_config_sha256: None,
        embedded_topology_sha256: None,
        builder_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        rust_toolchain: None,
        package_version: None,
    }
}

fn observe_file_sha256(
    path: &Path,
    role: &str,
    gaps: &mut Vec<DeploymentObservationGapV1>,
) -> Option<String> {
    match file_sha256_hex(path) {
        Ok(hash) => Some(hash),
        Err(err) => {
            gaps.push(observation_gap(
                format!("local_artifacts.{role}.file_sha256"),
                format!("could not hash artifact {}: {err}", path.display()),
            ));
            None
        }
    }
}

pub(super) fn observe_config_sha256(
    path: &Path,
    gaps: &mut Vec<DeploymentObservationGapV1>,
) -> Option<String> {
    match file_sha256_hex(path) {
        Ok(hash) => Some(hash),
        Err(err) => {
            gaps.push(observation_gap(
                "local_config.raw_sha256",
                format!("could not hash config {}: {err}", path.display()),
            ));
            None
        }
    }
}

pub(super) fn observe_deployment_manifest_digest(
    icp_root: &Path,
    artifact_environment: &str,
    gaps: &mut Vec<DeploymentObservationGapV1>,
) -> Option<String> {
    release_set_manifest_digest(icp_root, artifact_environment, gaps)
}

pub(super) fn observe_canonical_runtime_config_digest(
    path: &Path,
    gaps: &mut Vec<DeploymentObservationGapV1>,
) -> Option<String> {
    match canonical_runtime_config_sha256_hex(path) {
        Ok(hash) => Some(hash),
        Err(err) => {
            gaps.push(observation_gap(
                "local_config.canonical_runtime_config_sha256",
                format!(
                    "could not hash canonical runtime config {}: {err}",
                    path.display()
                ),
            ));
            None
        }
    }
}
