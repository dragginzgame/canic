//! Module: canister_build::candid_cache
//!
//! Responsibility: reuse Candid extracted from exact compiled declaration bytes.
//! Does not own: Cargo freshness, role profiles, runtime artifacts or release identity.
//! Boundary: run after Cargo; a cache miss always uses the ordinary extractor.

#[cfg(test)]
mod tests;

use super::{
    WorkspaceBuildContext,
    candid::{extract_candid_bytes, extract_candid_with_tool},
    reuse::{BuildReuseError, file_hash, require_native_tool, resolve_tool},
};
use crate::durable_io::{read_regular_bytes, write_bytes};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    env, io,
    path::{Path, PathBuf},
};

// This bounds optional cache I/O, not accepted Candid size. Larger results are extracted normally.
const CACHE_RECORD_LIMIT: usize = 4 * 1024 * 1024;

/// Invocation-owned extractor identity; declaration Wasm supplies the role's compiled inputs.
pub(super) struct CandidExtractionCache {
    directory: PathBuf,
    extractor: PathBuf,
    extractor_sha256: String,
    identity: String,
}

/// Optional host cache evidence for one normalized extraction result.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CandidExtractionRecord {
    schema_version: u8,
    identity: String,
    wasm_sha256: String,
    candid_sha256: String,
    candid: String,
}

impl CandidExtractionCache {
    pub(super) fn prepare(context: &WorkspaceBuildContext) -> Option<Self> {
        match Self::new(context.icp_root.join(".canic/build-reuse/declarations")) {
            Ok(cache) => Some(cache),
            Err(error) => {
                eprintln!("Declaration reuse unavailable: {error}");
                None
            }
        }
    }

    fn new(directory: PathBuf) -> Result<Self, BuildReuseError> {
        let extractor = resolve_tool("candid-extractor".as_ref())?;
        Self::with_extractor(directory, extractor)
    }

    fn with_extractor(directory: PathBuf, extractor: PathBuf) -> Result<Self, BuildReuseError> {
        require_native_tool(&extractor)?;
        let extractor_sha256 = file_hash(&extractor)?;
        let mut identity = Sha256::new();
        identity.update(b"canic.candid-extraction.v1");
        identity.update(extractor_sha256.as_bytes());
        // Bind the compiled extraction implementation without rehashing a large host binary.
        identity.update(env!("CARGO_PKG_VERSION").as_bytes());
        identity.update(include_bytes!("../candid.rs"));
        identity.update(include_bytes!("mod.rs"));
        let mut environment = env::vars_os().collect::<Vec<_>>();
        environment.sort();
        for pair in environment {
            for field in <[_; 2]>::from(pair) {
                let bytes = field.as_encoded_bytes();
                identity.update((bytes.len() as u64).to_be_bytes());
                identity.update(bytes);
            }
        }
        Ok(Self {
            directory,
            extractor,
            extractor_sha256,
            identity: format!("{:x}", identity.finalize()),
        })
    }

    fn record_path(&self, wasm_sha256: &str) -> PathBuf {
        self.directory
            .join(&self.identity)
            .join(format!("{wasm_sha256}.json"))
    }

    fn extract(&self, wasm: &Path) -> Result<(Vec<u8>, bool), Box<dyn std::error::Error>> {
        self.require_current_extractor()?;
        let wasm_sha256 = file_hash(wasm)?;
        let path = self.record_path(&wasm_sha256);
        match self.load(&path, &wasm_sha256) {
            Ok(Some(candid)) => return Ok((candid, true)),
            Ok(None) => {}
            Err(error) => eprintln!("Declaration cache rejected for {}: {error}", wasm.display()),
        }
        let candid = extract_candid_with_tool(wasm, &self.extractor)?;
        self.require_current_extractor()?;
        if file_hash(wasm)? != wasm_sha256 {
            return Err(BuildReuseError::ChangedInput(wasm.into()).into());
        }
        let record = CandidExtractionRecord {
            schema_version: 1,
            identity: self.identity.clone(),
            wasm_sha256,
            candid_sha256: format!("{:x}", Sha256::digest(&candid)),
            candid: String::from_utf8(candid.clone())?,
        };
        let bytes = serde_json::to_vec(&record)?;
        if bytes.len() <= CACHE_RECORD_LIMIT
            && let Err(error) = write_bytes(&path, &bytes)
        {
            eprintln!(
                "Declaration cache not recorded for {}: {error}",
                wasm.display()
            );
        }
        Ok((candid, false))
    }

    fn require_current_extractor(&self) -> Result<(), BuildReuseError> {
        if file_hash(&self.extractor)? != self.extractor_sha256 {
            return Err(BuildReuseError::ChangedInput(self.extractor.clone()));
        }
        Ok(())
    }

    fn load(&self, path: &Path, wasm_sha256: &str) -> Result<Option<Vec<u8>>, BuildReuseError> {
        let bytes = match read_regular_bytes(path, CACHE_RECORD_LIMIT) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let record: CandidExtractionRecord = serde_json::from_slice(&bytes)?;
        if record.schema_version != 1
            || record.identity != self.identity
            || record.wasm_sha256 != wasm_sha256
            || record.candid_sha256 != format!("{:x}", Sha256::digest(record.candid.as_bytes()))
        {
            return Err(BuildReuseError::Evidence(
                "Candid extraction binding differs".into(),
            ));
        }
        Ok(Some(record.candid.into_bytes()))
    }
}

/// Reuse only a compiled declaration result; derive current role profiles and runtimes afterward.
pub(super) fn extract_configured_candid(
    cache: Option<&CandidExtractionCache>,
    role: &str,
    wasm: &Path,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let Some(cache) = cache else {
        return extract_candid_bytes(wasm);
    };
    let (candid, reused) = cache.extract(wasm)?;
    eprintln!(
        "Build declaration cache {role}: {}",
        if reused {
            "hit (verified compiled Wasm)"
        } else {
            "miss (extracted current Wasm)"
        }
    );
    Ok(candid)
}
