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
use ic_build::artifacts::{
    ArtifactCacheError, ArtifactCachePreparation, ArtifactCacheSpec, prepare_artifact_cache,
};
use ic_host_fs::durable::{PublicationMode, WriteOptions, write_bytes};
use ic_host_fs::read::read_file_no_follow;
use std::path::{Path, PathBuf};

// This bounds optional cached Candid reads and writes, not accepted Candid size. Larger results are extracted normally.
const CACHE_CANDID_LIMIT: usize = 4 * 1024 * 1024;
const MAX_EXTRACTORS: usize = 4;

/// One role's completed declaration, ready for independent extraction.
pub(super) struct CandidExtractionInput<'a> {
    pub role: &'a str,
    pub wasm: PathBuf,
}

/// Failure of a drained extraction batch; no partial profiles are published.
#[derive(Debug, thiserror::Error)]
pub(super) enum CandidBatchError {
    #[error("Candid extraction for {role} failed: {detail}")]
    Extraction { role: String, detail: String },

    #[error("Candid extraction worker for {role} panicked")]
    WorkerPanicked { role: String },
}

/// Extract completed declarations with bounded workers and input-order results.
pub(super) fn extract_configured_candids(
    cache: Option<&CandidExtractionCache>,
    inputs: &[CandidExtractionInput<'_>],
) -> Result<Vec<Vec<u8>>, CandidBatchError> {
    crate::bounded_reads::collect(
        inputs,
        std::num::NonZeroUsize::new(MAX_EXTRACTORS).expect("nonzero extraction concurrency"),
        |input| {
            extract_configured_candid(cache, input.role, &input.wasm).map_err(|error| {
                CandidBatchError::Extraction {
                    role: input.role.to_string(),
                    detail: error.to_string(),
                }
            })
        },
        |input, _| CandidBatchError::WorkerPanicked {
            role: input.role.to_string(),
        },
    )
}

/// Invocation-owned extractor identity; declaration Wasm supplies the role's compiled inputs.
pub(super) struct CandidExtractionCache {
    directory: PathBuf,
    extractor: PathBuf,
    extractor_sha256: String,
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
        Ok(Self {
            directory,
            extractor,
            extractor_sha256,
        })
    }

    fn spec(&self, wasm: &Path, wasm_sha256: &str) -> ArtifactCacheSpec {
        // This distinct store is owned by IC Build. Existing Canic JSON evidence
        // is retained but neither read nor rewritten; no cache migration or pruning.
        ArtifactCacheSpec::new(
            &self.directory.join("ic-build"),
            "canic-candid-extraction",
            "canic.candid-extraction.v1",
        )
        .with_coordination_scope(&format!("canic.candid-extraction.{wasm_sha256}"))
        .with_input("declaration", wasm)
        .with_tool("extractor", &self.extractor)
        .with_environment(crate::build_environment::inputs())
        .with_identity_bytes("version", env!("CARGO_PKG_VERSION").as_bytes())
        .with_identity_bytes(
            "environment-recipe",
            include_bytes!("../../build_environment/mod.rs"),
        )
        .with_identity_bytes("extraction-recipe", include_bytes!("../candid.rs"))
        .with_identity_bytes("cache-recipe", include_bytes!("mod.rs"))
        .with_output(
            "candid",
            &self
                .directory
                .join("materialized")
                .join(format!("{wasm_sha256}.did")),
        )
    }

    fn extract(&self, wasm: &Path) -> Result<(Vec<u8>, bool), Box<dyn std::error::Error>> {
        // Invocation custody applies even when optional cache I/O is unavailable.
        self.require_current_extractor()?;
        let wasm_sha256 = file_hash(wasm)?;
        let transaction = match prepare_artifact_cache(&self.spec(wasm, &wasm_sha256)) {
            Ok(ArtifactCachePreparation::Reused(record)) => {
                // Hold the shared retention record through the bounded byte read.
                match read_file_no_follow(record.artifacts()[0].path(), CACHE_CANDID_LIMIT) {
                    Ok(candid) => {
                        self.require_current_inputs(wasm, &wasm_sha256)?;
                        return Ok((candid, true));
                    }
                    Err(error) => {
                        eprintln!("Declaration cache not read for {}: {error}", wasm.display());
                    }
                }
                None
            }
            Ok(ArtifactCachePreparation::Build(transaction)) => Some(transaction),
            Err(error) => {
                optional_cache_failure(wasm, error)?;
                None
            }
        };
        let candid = extract_candid_with_tool(wasm, &self.extractor)?;
        self.require_current_inputs(wasm, &wasm_sha256)?;
        if let Some(transaction) = transaction {
            if candid.len() > CACHE_CANDID_LIMIT {
                // The cache budget never reduces the extractor's admitted output.
                if let Err(error) = transaction.abort() {
                    optional_cache_failure(wasm, error)?;
                }
            } else {
                let path = transaction.output_path("candid")?;
                match write_bytes(
                    &path,
                    WriteOptions {
                        mode: PublicationMode::Replace,
                        permissions: 0o666,
                    },
                    &candid,
                ) {
                    Ok(()) => {
                        if let Err(error) = transaction.commit() {
                            optional_cache_failure(wasm, error)?;
                        }
                    }
                    Err(error) => {
                        eprintln!(
                            "Declaration cache not staged for {}: {error}",
                            wasm.display()
                        );
                        if let Err(error) = transaction.abort() {
                            optional_cache_failure(wasm, error)?;
                        }
                    }
                }
            }
        }
        self.require_current_inputs(wasm, &wasm_sha256)?;
        Ok((candid, false))
    }

    fn require_current_inputs(
        &self,
        wasm: &Path,
        wasm_sha256: &str,
    ) -> Result<(), BuildReuseError> {
        self.require_current_extractor()?;
        if file_hash(wasm)? != wasm_sha256 {
            return Err(BuildReuseError::ChangedInput(wasm.into()));
        }
        Ok(())
    }

    fn require_current_extractor(&self) -> Result<(), BuildReuseError> {
        if file_hash(&self.extractor)? != self.extractor_sha256 {
            return Err(BuildReuseError::ChangedInput(self.extractor.clone()));
        }
        Ok(())
    }
}

// Input mutation is never an optional-cache failure, including cleanup wrappers.
fn optional_cache_failure(
    wasm: &Path,
    error: ArtifactCacheError,
) -> Result<(), Box<dyn std::error::Error>> {
    fn changed(error: &ArtifactCacheError) -> bool {
        match error {
            ArtifactCacheError::InputsChangedDuringPreparation { .. }
            | ArtifactCacheError::InputsChangedDuringAcquisition { .. } => true,
            ArtifactCacheError::FailedTransactionCleanup {
                transaction_error, ..
            } => changed(transaction_error),
            _ => false,
        }
    }
    if changed(&error) {
        return Err(error.into());
    }
    eprintln!(
        "Declaration cache unavailable for {}: {error}",
        wasm.display()
    );
    Ok(())
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
