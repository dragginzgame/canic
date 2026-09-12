//! Final artifact measurement for build summaries and evidence.
//! Reuses the artifact owner's section parser; does not transform or admit Wasm.

use crate::{artifact_io::wasm_artifact_metrics, canister_build::WasmArtifactMetrics};
use std::{fs, path::Path};

/// Measure final Wasm sections and the emitted gzip file without recompressing.
///
/// Returns an error if either artifact is unavailable or the Wasm cannot be inspected.
pub fn read_wasm_artifact_metrics(
    wasm_path: &Path,
    wasm_gz_path: &Path,
) -> Result<WasmArtifactMetrics, Box<dyn std::error::Error>> {
    let wasm = fs::read(wasm_path)?;
    let gzip_bytes = usize::try_from(fs::metadata(wasm_gz_path)?.len())?;
    wasm_artifact_metrics(&wasm, gzip_bytes)
}
