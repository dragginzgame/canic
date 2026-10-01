//! Select the current Cargo invocation's exact cdylib Wasm by its admitted manifest.
//!
//! Never infer an output from the package name or accept an unrelated stale target file.

#[cfg(test)]
mod tests;

use serde::Deserialize;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Cargo output cannot establish one exact Wasm for the selected package.
#[derive(Debug, Error)]
pub(super) enum CargoArtifactError {
    #[error("Cargo artifact message is invalid: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("Cargo artifact manifest cannot be resolved: {0}")]
    Io(#[from] std::io::Error),
    #[error(
        "Cargo did not report exactly one cdylib Wasm for {manifest}; observed {count} outputs"
    )]
    Selection { manifest: PathBuf, count: usize },
}

#[derive(Deserialize)]
struct Artifact {
    manifest_path: PathBuf,
    target: Target,
    filenames: Vec<PathBuf>,
}

#[derive(Deserialize)]
struct Target {
    crate_types: Vec<String>,
}

pub(super) fn select(messages: &[u8], manifest: &Path) -> Result<PathBuf, CargoArtifactError> {
    let manifest = manifest.canonicalize()?;
    let mut outputs = BTreeSet::new();
    for line in messages
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let value: serde_json::Value = serde_json::from_slice(line)?;
        if value["reason"] != "compiler-artifact" {
            continue;
        }
        let artifact: Artifact = serde_json::from_value(value)?;
        if !artifact
            .target
            .crate_types
            .iter()
            .any(|kind| kind == "cdylib")
            || artifact.manifest_path.canonicalize()? != manifest
        {
            continue;
        }
        outputs.extend(artifact.filenames.into_iter().filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "wasm")
        }));
    }
    if outputs.len() != 1 {
        return Err(CargoArtifactError::Selection {
            manifest,
            count: outputs.len(),
        });
    }
    Ok(outputs.pop_first().expect("one selected Cargo artifact"))
}
