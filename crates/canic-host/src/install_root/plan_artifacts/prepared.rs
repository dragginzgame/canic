//! Module: install_root::plan_artifacts::prepared
//!
//! Responsibility: admit and own one canonical supplied-plan artifact snapshot.
//! Does not own: deployment policy, network mutation, or activation sequencing.
//! Boundary: provides revalidated bytes to truth, manifest, and activation consumers.

use crate::{
    deployment_truth::{
        ArtifactDigestSourceV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentPlanV1, RoleArtifactV1,
    },
    durable_io::write_bytes,
    install_root::{plan_artifacts::error::PlanArtifactError, validate_environment_name},
    release_set::{
        GZIP_MAGIC, ReleaseSetEntry, RootReleaseSetManifest, WASM_MAGIC, artifact_root_path,
        root_release_set_manifest_path, validate_root_release_set_manifest,
    },
};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

use canic_core::{
    CANIC_WASM_CHUNK_BYTES, bootstrap::compiled::validate_canister_role_name,
    cdk::utils::hash::wasm_hash_hex,
};
use flate2::{Compression, GzBuilder, read::GzDecoder};

#[derive(Debug)]
struct PreparedRoleArtifact {
    role: String,
    wasm_path: PathBuf,
    wasm_gz_path: PathBuf,
    wasm_sha256: String,
    wasm_gz_sha256: String,
}

///
/// PreparedPlanArtifacts
///
/// Canonical artifact snapshot admitted from one supplied deployment plan.
/// Truth checks, manifest emission, and activation all consume this authority.
///

#[derive(Debug)]
pub(in crate::install_root) struct PreparedPlanArtifacts {
    plan: DeploymentPlanV1,
    icp_root: PathBuf,
    artifact_root: PathBuf,
    roles: Vec<PreparedRoleArtifact>,
}

impl PreparedPlanArtifacts {
    pub(in crate::install_root) fn materialize(
        plan: &DeploymentPlanV1,
        icp_root: &Path,
        environment: &str,
    ) -> Result<Self, PlanArtifactError> {
        validate_environment_name(environment).map_err(|_| {
            PlanArtifactError::InvalidEnvironment {
                name: environment.to_string(),
            }
        })?;
        if plan.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
            return Err(PlanArtifactError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: plan.schema_version,
            });
        }
        if plan.deployment_identity.environment != environment {
            return Err(PlanArtifactError::EnvironmentMismatch {
                install: environment.to_string(),
                plan: plan.deployment_identity.environment.clone(),
            });
        }
        if plan.plan_id.trim().is_empty() {
            return Err(PlanArtifactError::MissingPlanId);
        }

        let canonical_root = canonicalize(icp_root)?;
        let artifact_root = artifact_root_path(&canonical_root, environment);
        let mut seen = BTreeSet::new();
        let mut admitted = Vec::with_capacity(plan.role_artifacts.len());
        for artifact in &plan.role_artifacts {
            validate_canister_role_name(&artifact.role).map_err(|issue| {
                PlanArtifactError::InvalidRole {
                    role: artifact.role.clone(),
                    reason: issue.to_string(),
                }
            })?;
            if !seen.insert(artifact.role.as_str()) {
                return Err(PlanArtifactError::DuplicateRole {
                    role: artifact.role.clone(),
                });
            }
            admitted.push(admit_role_artifact(&canonical_root, artifact)?);
        }
        if !seen.contains("root") {
            return Err(PlanArtifactError::MissingRoot);
        }

        let mut normalized_plan = plan.clone();
        let mut roles = Vec::with_capacity(admitted.len());
        for admitted in admitted {
            let wasm_path = artifact_root
                .join(&admitted.role)
                .join(format!("{}.wasm", admitted.role));
            let wasm_gz_path = artifact_root
                .join(&admitted.role)
                .join(format!("{}.wasm.gz", admitted.role));
            validate_target_path(&canonical_root, &wasm_path, &admitted.role)?;
            validate_target_path(&canonical_root, &wasm_gz_path, &admitted.role)?;
            write_artifact(&wasm_path, &admitted.wasm)?;
            write_artifact(&wasm_gz_path, &admitted.wasm_gz)?;
            let wasm_path = resolve_source_path(
                &canonical_root,
                &admitted.role,
                &wasm_path.to_string_lossy(),
            )?;
            let wasm_gz_path = resolve_source_path(
                &canonical_root,
                &admitted.role,
                &wasm_gz_path.to_string_lossy(),
            )?;

            let artifact = normalized_plan
                .role_artifacts
                .iter_mut()
                .find(|artifact| artifact.role == admitted.role)
                .ok_or_else(|| PlanArtifactError::MissingSource {
                    role: admitted.role.clone(),
                })?;
            artifact.wasm_path = Some(wasm_path.display().to_string());
            artifact.wasm_gz_path = Some(wasm_gz_path.display().to_string());
            artifact.wasm_gz_size_bytes =
                Some(u64::try_from(admitted.wasm_gz.len()).map_err(|_| {
                    PlanArtifactError::InvalidWasm {
                        role: admitted.role.clone(),
                    }
                })?);
            artifact.wasm_sha256 = Some(admitted.wasm_sha256.clone());
            artifact.wasm_gz_sha256 = Some(admitted.wasm_gz_sha256.clone());
            artifact.wasm_gz_sha256_source = Some(ArtifactDigestSourceV1::ObservedFileDigest);
            artifact.observed_wasm_gz_file_sha256 = Some(admitted.wasm_gz_sha256.clone());
            artifact.observed_wasm_gz_file_sha256_source =
                Some(ArtifactDigestSourceV1::ObservedFileDigest);

            roles.push(PreparedRoleArtifact {
                role: admitted.role,
                wasm_path,
                wasm_gz_path,
                wasm_sha256: admitted.wasm_sha256,
                wasm_gz_sha256: admitted.wasm_gz_sha256,
            });
        }

        Ok(Self {
            plan: normalized_plan,
            icp_root: canonical_root,
            artifact_root,
            roles,
        })
    }

    pub(in crate::install_root) const fn plan(&self) -> &DeploymentPlanV1 {
        &self.plan
    }

    pub(in crate::install_root) fn verified_root_wasm_path(
        &self,
    ) -> Result<PathBuf, PlanArtifactError> {
        let root = self
            .roles
            .iter()
            .find(|artifact| artifact.role == "root")
            .ok_or(PlanArtifactError::MissingRoot)?;
        verify_prepared_role(root)?;
        Ok(root.wasm_path.clone())
    }

    pub(in crate::install_root) fn emit_release_set_manifest(
        &self,
    ) -> Result<PathBuf, PlanArtifactError> {
        let entries = self
            .roles
            .iter()
            .filter(|artifact| !matches!(artifact.role.as_str(), "root" | "wasm_store"))
            .map(|artifact| release_set_entry(&self.icp_root, artifact))
            .collect::<Result<Vec<_>, _>>()?;
        let manifest = RootReleaseSetManifest {
            release_version: self
                .plan
                .deployment_identity
                .canic_version
                .clone()
                .unwrap_or_else(|| self.plan.plan_id.clone()),
            entries,
        };
        let path = root_release_set_manifest_path(&self.artifact_root);
        validate_root_release_set_manifest(&manifest).map_err(|source| {
            PlanArtifactError::InvalidManifest {
                path: path.clone(),
                reason: source.to_string(),
            }
        })?;
        validate_target_path(&self.icp_root, &path, "root")?;
        let bytes = serde_json::to_vec_pretty(&manifest).map_err(|source| {
            PlanArtifactError::Serialization {
                path: path.clone(),
                source,
            }
        })?;
        write_artifact(&path, &bytes)?;
        Ok(path)
    }
}

struct AdmittedRoleArtifact {
    role: String,
    wasm: Vec<u8>,
    wasm_gz: Vec<u8>,
    wasm_sha256: String,
    wasm_gz_sha256: String,
}

fn admit_role_artifact(
    canonical_root: &Path,
    artifact: &RoleArtifactV1,
) -> Result<AdmittedRoleArtifact, PlanArtifactError> {
    let supplied_wasm = artifact
        .wasm_path
        .as_deref()
        .map(|path| read_source(canonical_root, &artifact.role, path))
        .transpose()?;
    let supplied_wasm_gz = artifact
        .wasm_gz_path
        .as_deref()
        .map(|path| read_source(canonical_root, &artifact.role, path))
        .transpose()?;
    if supplied_wasm.is_none() && supplied_wasm_gz.is_none() {
        return Err(PlanArtifactError::MissingSource {
            role: artifact.role.clone(),
        });
    }

    let (wasm, wasm_gz) = normalize_representations(
        &artifact.role,
        supplied_wasm.as_deref(),
        supplied_wasm_gz.as_deref(),
    )?;
    let wasm_sha256 = wasm_hash_hex(&wasm);
    let wasm_gz_sha256 = wasm_hash_hex(&wasm_gz);
    validate_digest_pins(
        artifact,
        supplied_wasm_gz.is_some(),
        &wasm_sha256,
        &wasm_gz_sha256,
    )?;

    Ok(AdmittedRoleArtifact {
        role: artifact.role.clone(),
        wasm,
        wasm_gz,
        wasm_sha256,
        wasm_gz_sha256,
    })
}

fn normalize_representations(
    role: &str,
    supplied_wasm: Option<&[u8]>,
    supplied_wasm_gz: Option<&[u8]>,
) -> Result<(Vec<u8>, Vec<u8>), PlanArtifactError> {
    let decoded = supplied_wasm_gz
        .map(|bytes| decompress_wasm(role, bytes))
        .transpose()?;
    let wasm = match (supplied_wasm, decoded.as_deref()) {
        (Some(wasm), Some(decoded)) if wasm != decoded => {
            return Err(PlanArtifactError::RepresentationMismatch {
                role: role.to_string(),
            });
        }
        (Some(wasm), _) => wasm.to_vec(),
        (None, Some(decoded)) => decoded.to_vec(),
        (None, None) => {
            return Err(PlanArtifactError::MissingSource {
                role: role.to_string(),
            });
        }
    };
    validate_wasm(role, &wasm)?;
    let wasm_gz = match supplied_wasm_gz {
        Some(bytes) => bytes.to_vec(),
        None => compress_wasm(role, &wasm)?,
    };
    Ok((wasm, wasm_gz))
}

fn validate_digest_pins(
    artifact: &RoleArtifactV1,
    supplied_wasm_gz: bool,
    wasm_sha256: &str,
    wasm_gz_sha256: &str,
) -> Result<(), PlanArtifactError> {
    let raw_pins = artifact
        .wasm_sha256
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let gzip_pins = [
        artifact.wasm_gz_sha256.as_deref(),
        artifact.observed_wasm_gz_file_sha256.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    if raw_pins.is_empty() {
        return Err(PlanArtifactError::MissingDigestPin {
            role: artifact.role.clone(),
            kind: "raw Wasm",
        });
    }
    if supplied_wasm_gz && gzip_pins.is_empty() {
        return Err(PlanArtifactError::MissingDigestPin {
            role: artifact.role.clone(),
            kind: "gzip Wasm",
        });
    }

    validate_matching_pins(&artifact.role, "raw Wasm", &raw_pins, wasm_sha256)?;
    validate_matching_pins(&artifact.role, "gzip Wasm", &gzip_pins, wasm_gz_sha256)
}

fn validate_matching_pins(
    role: &str,
    kind: &'static str,
    pins: &[&str],
    actual: &str,
) -> Result<(), PlanArtifactError> {
    if let [first, second, ..] = pins
        && first != second
    {
        return Err(PlanArtifactError::ConflictingDigest {
            role: role.to_string(),
            kind,
            first: (*first).to_string(),
            second: (*second).to_string(),
        });
    }
    if let Some(expected) = pins.first()
        && *expected != actual
    {
        return Err(PlanArtifactError::DigestMismatch {
            role: role.to_string(),
            kind,
            expected: (*expected).to_string(),
            found: actual.to_string(),
        });
    }
    Ok(())
}

fn read_source(
    canonical_root: &Path,
    role: &str,
    source: &str,
) -> Result<Vec<u8>, PlanArtifactError> {
    let path = resolve_source_path(canonical_root, role, source)?;
    fs::read(&path).map_err(|source| PlanArtifactError::Io { path, source })
}

fn resolve_source_path(
    canonical_root: &Path,
    role: &str,
    source: &str,
) -> Result<PathBuf, PlanArtifactError> {
    let supplied = Path::new(source);
    let candidate = if supplied.is_absolute() {
        supplied.to_path_buf()
    } else {
        if supplied.as_os_str().is_empty()
            || supplied
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(PlanArtifactError::UnsafePath {
                role: role.to_string(),
                path: supplied.to_path_buf(),
            });
        }
        canonical_root.join(supplied)
    };
    let relative =
        candidate
            .strip_prefix(canonical_root)
            .map_err(|_| PlanArtifactError::UnsafePath {
                role: role.to_string(),
                path: candidate.clone(),
            })?;
    reject_symlink_components(canonical_root, relative, role)?;
    let canonical = canonicalize(&candidate)?;
    if !canonical.starts_with(canonical_root) {
        return Err(PlanArtifactError::UnsafePath {
            role: role.to_string(),
            path: canonical,
        });
    }
    let metadata = fs::metadata(&canonical).map_err(|source| PlanArtifactError::Io {
        path: canonical.clone(),
        source,
    })?;
    if !metadata.is_file() {
        return Err(PlanArtifactError::UnsafePath {
            role: role.to_string(),
            path: canonical,
        });
    }
    Ok(canonical)
}

fn reject_symlink_components(
    canonical_root: &Path,
    relative: &Path,
    role: &str,
) -> Result<(), PlanArtifactError> {
    let mut current = canonical_root.to_path_buf();
    for component in relative.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(PlanArtifactError::UnsafePath {
                role: role.to_string(),
                path: relative.to_path_buf(),
            });
        }
        current.push(component);
        let metadata = fs::symlink_metadata(&current).map_err(|source| PlanArtifactError::Io {
            path: current.clone(),
            source,
        })?;
        if metadata.file_type().is_symlink() {
            return Err(PlanArtifactError::UnsafePath {
                role: role.to_string(),
                path: current,
            });
        }
    }
    Ok(())
}

fn validate_target_path(
    canonical_root: &Path,
    target: &Path,
    role: &str,
) -> Result<(), PlanArtifactError> {
    let relative =
        target
            .strip_prefix(canonical_root)
            .map_err(|_| PlanArtifactError::UnsafePath {
                role: role.to_string(),
                path: target.to_path_buf(),
            })?;
    let mut current = canonical_root.to_path_buf();
    for component in relative.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(PlanArtifactError::UnsafePath {
                role: role.to_string(),
                path: target.to_path_buf(),
            });
        }
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(PlanArtifactError::UnsafePath {
                    role: role.to_string(),
                    path: current,
                });
            }
            Ok(_) => {}
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(PlanArtifactError::Io {
                    path: current,
                    source,
                });
            }
        }
    }
    Ok(())
}

fn verify_prepared_role(
    artifact: &PreparedRoleArtifact,
) -> Result<(Vec<u8>, Vec<u8>), PlanArtifactError> {
    let wasm = fs::read(&artifact.wasm_path).map_err(|source| PlanArtifactError::Io {
        path: artifact.wasm_path.clone(),
        source,
    })?;
    let wasm_gz = fs::read(&artifact.wasm_gz_path).map_err(|source| PlanArtifactError::Io {
        path: artifact.wasm_gz_path.clone(),
        source,
    })?;
    let (wasm, wasm_gz) = normalize_representations(&artifact.role, Some(&wasm), Some(&wasm_gz))?;
    validate_matching_pins(
        &artifact.role,
        "raw Wasm",
        &[artifact.wasm_sha256.as_str()],
        &wasm_hash_hex(&wasm),
    )?;
    validate_matching_pins(
        &artifact.role,
        "gzip Wasm",
        &[artifact.wasm_gz_sha256.as_str()],
        &wasm_hash_hex(&wasm_gz),
    )?;
    Ok((wasm, wasm_gz))
}

fn release_set_entry(
    icp_root: &Path,
    artifact: &PreparedRoleArtifact,
) -> Result<ReleaseSetEntry, PlanArtifactError> {
    let (_, bytes) = verify_prepared_role(artifact)?;
    let chunk_hashes = bytes
        .chunks(CANIC_WASM_CHUNK_BYTES)
        .map(wasm_hash_hex)
        .collect::<Vec<_>>();
    let artifact_relative_path = artifact
        .wasm_gz_path
        .strip_prefix(icp_root)
        .map_err(|_| PlanArtifactError::UnsafePath {
            role: artifact.role.clone(),
            path: artifact.wasm_gz_path.clone(),
        })?
        .to_string_lossy()
        .to_string();
    Ok(ReleaseSetEntry {
        role: artifact.role.clone(),
        template_id: format!("embedded:{}", artifact.role),
        artifact_relative_path,
        payload_size_bytes: u64::try_from(bytes.len()).map_err(|_| {
            PlanArtifactError::InvalidWasm {
                role: artifact.role.clone(),
            }
        })?,
        payload_sha256_hex: wasm_hash_hex(&bytes),
        chunk_size_bytes: u64::try_from(CANIC_WASM_CHUNK_BYTES).map_err(|_| {
            PlanArtifactError::InvalidWasm {
                role: artifact.role.clone(),
            }
        })?,
        chunk_sha256_hex: chunk_hashes,
    })
}

fn validate_wasm(role: &str, bytes: &[u8]) -> Result<(), PlanArtifactError> {
    if bytes.starts_with(&WASM_MAGIC) {
        Ok(())
    } else {
        Err(PlanArtifactError::InvalidWasm {
            role: role.to_string(),
        })
    }
}

fn decompress_wasm(role: &str, bytes: &[u8]) -> Result<Vec<u8>, PlanArtifactError> {
    if !bytes.starts_with(&GZIP_MAGIC) {
        return Err(PlanArtifactError::InvalidGzip {
            role: role.to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, "missing gzip header"),
        });
    }
    let mut wasm = Vec::new();
    GzDecoder::new(bytes)
        .read_to_end(&mut wasm)
        .map_err(|source| PlanArtifactError::InvalidGzip {
            role: role.to_string(),
            source,
        })?;
    validate_wasm(role, &wasm)?;
    Ok(wasm)
}

fn compress_wasm(role: &str, bytes: &[u8]) -> Result<Vec<u8>, PlanArtifactError> {
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::best());
    encoder
        .write_all(bytes)
        .map_err(|source| PlanArtifactError::InvalidGzip {
            role: role.to_string(),
            source,
        })?;
    encoder
        .finish()
        .map_err(|source| PlanArtifactError::InvalidGzip {
            role: role.to_string(),
            source,
        })
}

fn canonicalize(path: &Path) -> Result<PathBuf, PlanArtifactError> {
    path.canonicalize().map_err(|source| PlanArtifactError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn write_artifact(path: &Path, bytes: &[u8]) -> Result<(), PlanArtifactError> {
    write_bytes(path, bytes).map_err(|source| PlanArtifactError::Io {
        path: path.to_path_buf(),
        source,
    })
}
