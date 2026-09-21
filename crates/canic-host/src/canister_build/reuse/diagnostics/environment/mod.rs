//! Module: canister_build::reuse::diagnostics::environment
//!
//! Responsibility: optionally attribute changed environment values to bounded key names.
//! Does not own: build fingerprints, environment filtering or cache admission.
//! Boundary: retain keyed comparison tags only; keep their random key private and local.

use crate::durable_io::{create_private_bytes_with_parents, read_private_bytes};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2_host::Sha256;
use std::{collections::BTreeMap, ffi::OsString, path::Path};
use thiserror::Error;

const KEY_PATH: &str = ".canic/local-secrets/build-environment.key";
const MAX_KEYS: usize = 256;

/// Why optional local environment evidence could not be captured.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Error, PartialEq, Serialize)]
pub(super) enum CaptureFailure {
    #[error("local private comparison key is missing, unreadable or unsafe")]
    LocalKeyUnavailable,
    #[error("environment input count exceeds the comparison limit")]
    InputLimitExceeded,
}

/// Why two diagnostic captures cannot attribute changed values safely.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub(super) enum ComparisonFailure {
    #[error("current build: {0}")]
    CurrentCapture(CaptureFailure),
    #[error("previous build: {0}")]
    PreviousCapture(CaptureFailure),
    #[error("builds use different local comparison keys")]
    DifferentLocalKeys,
    #[error("retained comparison evidence exceeds the key limit")]
    EvidenceLimitExceeded,
}

///
/// EnvironmentComparison
///
/// Optional keyed equality evidence, never displayed or used as build authority.
///
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EnvironmentComparison {
    key_id: [u8; 32],
    tags: BTreeMap<String, [u8; 32]>,
}

// Called under the complete-build lock. Failure disables attribution only.
pub(super) fn prepare_key(root: &Path) {
    let path = root.join(KEY_PATH);
    // Never replace an existing unreadable/unsafe key or rotate it implicitly.
    if std::fs::symlink_metadata(&path).is_ok() {
        return;
    }
    let mut key = [0; 32];
    if getrandom::fill(&mut key).is_ok() {
        let _ = create_private_bytes_with_parents(&path, &key);
    }
    key.fill(0);
}

impl EnvironmentComparison {
    pub(super) fn capture(
        root: &Path,
        inputs: &[(OsString, OsString)],
    ) -> Result<Self, CaptureFailure> {
        let mut key = read_private_bytes::<32>(&root.join(KEY_PATH))
            .ok_or(CaptureFailure::LocalKeyUnavailable)?;
        let result = Self::with_key(&key, inputs);
        key.fill(0);
        result
    }

    fn with_key(key: &[u8; 32], inputs: &[(OsString, OsString)]) -> Result<Self, CaptureFailure> {
        if inputs.len() > MAX_KEYS {
            return Err(CaptureFailure::InputLimitExceeded);
        }
        let mut tags = BTreeMap::new();
        for (name, value) in inputs {
            if let Some(name) = name.to_str().filter(|name| super::safe_key(name)) {
                tags.insert(
                    name.to_owned(),
                    tag(
                        key,
                        b"environment-value",
                        &[name.as_bytes(), value.as_encoded_bytes()],
                    ),
                );
            }
        }
        Ok(Self {
            key_id: tag(key, b"environment-key-id", &[]),
            tags,
        })
    }

    pub(super) fn changed_keys(&self, previous: &Self) -> Result<Vec<String>, ComparisonFailure> {
        if self.tags.len() > MAX_KEYS || previous.tags.len() > MAX_KEYS {
            return Err(ComparisonFailure::EvidenceLimitExceeded);
        }
        if self.key_id != previous.key_id {
            return Err(ComparisonFailure::DifferentLocalKeys);
        }
        Ok(self
            .tags
            .iter()
            .filter(|(name, value)| {
                super::safe_key(name) && previous.tags.get(*name).is_some_and(|old| old != *value)
            })
            .take(8)
            .map(|(name, _)| name.clone())
            .collect())
    }
}

fn tag(key: &[u8; 32], domain: &[u8], fields: &[&[u8]]) -> [u8; 32] {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts a 32-byte key");
    mac.update(domain);
    for field in fields {
        mac.update(&(field.len() as u64).to_le_bytes());
        mac.update(field);
    }
    mac.finalize().into_bytes().into()
}

#[cfg(test)]
mod tests;
