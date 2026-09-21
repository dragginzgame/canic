//! Module: canister_build::reuse::diagnostics
//!
//! Responsibility: explain differences from the last successful reuse snapshot.
//! Does not own: cache identity, hit admission or release authority.
//! Boundary: bounded optional evidence contains no plaintext values or unkeyed per-value hashes.

mod environment;
mod rejection;
#[cfg(test)]
mod tests;

use crate::{
    canister_build::{
        WorkspaceBuildContext,
        reuse::{hash_field, snapshot::BuildInputSnapshot},
    },
    durable_io::{read_regular_bytes, write_bytes},
};
use canic_core::cdk::utils::hash::hex_bytes;
use serde::{Deserialize, Serialize};
use sha2_host::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    ffi::OsString,
    path::{Path, PathBuf},
};

pub(super) use rejection::{InputLocations, retain_rejection};

const LIMIT: usize = 256 * 1024;

///
/// InputDiagnostics
///
/// Optional host comparison with a verified complete build, never cache authority.
///

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InputDiagnostics {
    schema_version: u8,
    environment: String,
    environment_keys: BTreeSet<String>,
    value_comparison: Result<environment::EnvironmentComparison, environment::CaptureFailure>,
    source: String,
    configuration: String,
}

impl InputDiagnostics {
    pub(super) fn prepare(root: &Path) {
        environment::prepare_key(root);
    }

    pub(super) fn capture(
        context: &WorkspaceBuildContext,
        tools: &[PathBuf],
        inputs: &BuildInputSnapshot,
    ) -> Self {
        let mut source = Sha256::new();
        let mut configuration = Sha256::new();
        hash_field(
            &mut configuration,
            context.profile.target_dir_name().as_bytes(),
        );
        hash_field(
            &mut configuration,
            context.build_network.as_str().as_bytes(),
        );
        hash_field(
            &mut configuration,
            context.config_path.as_os_str().as_encoded_bytes(),
        );
        for (path, hash) in &inputs.files {
            let path_ref = Path::new(path);
            let is_configuration = path_ref == context.config_path
                || tools.iter().any(|tool| tool == path_ref)
                || path_ref
                    .components()
                    .any(|part| part.as_os_str() == "rustlib")
                || matches!(
                    path_ref.file_name().and_then(|name| name.to_str()),
                    Some(
                        "Cargo.toml"
                            | "Cargo.lock"
                            | "rust-toolchain"
                            | "rust-toolchain.toml"
                            | "config"
                            | "config.toml"
                    )
                );
            let digest = if is_configuration {
                &mut configuration
            } else {
                &mut source
            };
            hash_field(digest, path.as_bytes());
            hash_field(digest, hash.as_bytes());
        }
        let values = crate::build_environment::inputs();
        let value_comparison =
            environment::EnvironmentComparison::capture(&context.icp_root, &values);
        let (environment, environment_keys) = environment_evidence(values);
        Self {
            schema_version: 1,
            environment,
            environment_keys,
            value_comparison,
            source: hex_bytes(source.finalize()),
            configuration: hex_bytes(configuration.finalize()),
        }
    }

    pub(super) fn explain_miss(&self, directory: &Path) -> String {
        let previous = read_regular_bytes(&directory.join("last-input-diagnostics.json"), LIMIT)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .filter(|record| record.schema_version == 1);
        previous.map_or_else(
            || "no comparable prior input evidence".into(),
            |previous| self.compare(&previous),
        )
    }

    // Failure only loses diagnostic attribution; it cannot invalidate verified artifacts.
    pub(super) fn retain(&self, directory: &Path) {
        if let Ok(bytes) = serde_json::to_vec(self)
            && bytes.len() <= LIMIT
        {
            let _ = write_bytes(&directory.join("last-input-diagnostics.json"), &bytes);
        }
    }

    fn compare(&self, previous: &Self) -> String {
        let mut reasons = Vec::new();
        if self.source != previous.source {
            reasons.push("source/dependency inputs changed".to_string());
        }
        if self.configuration != previous.configuration {
            reasons.push("toolchain/configuration inputs changed".to_string());
        }
        if self.environment != previous.environment {
            let keys = self
                .environment_keys
                .symmetric_difference(&previous.environment_keys)
                .filter(|key| safe_key(key))
                .take(8)
                .cloned()
                .collect::<Vec<_>>();
            let changed = self.changed_environment_keys(previous);
            let mut detail = Vec::new();
            if !keys.is_empty() {
                detail.push(format!("added/removed keys, up to 8: {}", keys.join(", ")));
            }
            match changed {
                Ok(keys) if !keys.is_empty() => {
                    detail.push(format!("changed-value keys, up to 8: {}", keys.join(", ")));
                }
                Ok(_) => detail.push("no changed values among comparable safe keys".into()),
                Err(reason) => detail.push(format!(
                    "changed-value key attribution unavailable: {reason}"
                )),
            }
            reasons.push(format!("environment changed ({})", detail.join("; ")));
        }
        if reasons.is_empty() {
            return "no retained exact-build evidence; comparison cannot attribute the miss".into();
        }
        format!(
            "{} (compared with last recorded successful build)",
            reasons.join("; ")
        )
    }

    fn changed_environment_keys(
        &self,
        previous: &Self,
    ) -> Result<Vec<String>, environment::ComparisonFailure> {
        let current = self
            .value_comparison
            .as_ref()
            .map_err(|reason| environment::ComparisonFailure::CurrentCapture(*reason))?;
        let previous = previous
            .value_comparison
            .as_ref()
            .map_err(|reason| environment::ComparisonFailure::PreviousCapture(*reason))?;
        current.changed_keys(previous)
    }
}

fn safe_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 80
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn environment_evidence(mut environment: Vec<(OsString, OsString)>) -> (String, BTreeSet<String>) {
    environment.sort();
    let mut digest = Sha256::new();
    let mut keys = BTreeSet::new();
    for (key, value) in environment {
        hash_field(&mut digest, key.as_encoded_bytes());
        hash_field(&mut digest, value.as_encoded_bytes());
        if let Some(key) = key.to_str().filter(|key| safe_key(key)) {
            keys.insert(key.to_string());
        }
    }
    (hex_bytes(digest.finalize()), keys)
}
