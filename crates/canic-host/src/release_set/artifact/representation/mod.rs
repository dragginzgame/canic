//! Qualify the raw and compressed representations of the same release Wasm.
//!
//! Manifest owners retain role, package, protocol and path authority checks.

#[cfg(test)]
mod tests;

use crate::release_set::{GZIP_MAGIC, WASM_MAGIC};
use std::io::{self, Read};

use canic_core::cdk::utils::hash::sha256_hex;
use flate2::read::GzDecoder;

///
/// QualifiedRepresentation
///
/// Qualified byte facts shared by application and infrastructure manifests.
///

#[derive(Debug)]
pub(in crate::release_set) struct QualifiedRepresentation {
    pub wasm_size_bytes: u64,
    pub wasm_gz_size_bytes: u64,
    pub wasm_sha256_hex: String,
    pub wasm_gz_sha256_hex: String,
}

///
/// RepresentationError
///
/// Byte-level failure mapped by manifest owners to their exact role diagnostics.
///

#[derive(Debug)]
pub(in crate::release_set) enum RepresentationError {
    ArtifactSizeOverflow { kind: &'static str },

    EmptyArtifact { kind: &'static str },

    InvalidGzip { source: io::Error },

    InvalidWasm,

    RepresentationMismatch,
}

pub(in crate::release_set) fn qualify_representation(
    wasm: &[u8],
    wasm_gz: &[u8],
) -> Result<QualifiedRepresentation, RepresentationError> {
    if wasm.is_empty() {
        return Err(RepresentationError::EmptyArtifact { kind: "raw Wasm" });
    }
    if !wasm.starts_with(&WASM_MAGIC) {
        return Err(RepresentationError::InvalidWasm);
    }
    if wasm_gz.is_empty() {
        return Err(RepresentationError::EmptyArtifact { kind: "gzip Wasm" });
    }
    if !wasm_gz.starts_with(&GZIP_MAGIC) {
        return Err(RepresentationError::InvalidGzip {
            source: io::Error::new(io::ErrorKind::InvalidData, "missing gzip header"),
        });
    }
    let wasm_size_bytes = u64::try_from(wasm.len())
        .map_err(|_| RepresentationError::ArtifactSizeOverflow { kind: "raw Wasm" })?;
    let wasm_gz_size_bytes = u64::try_from(wasm_gz.len())
        .map_err(|_| RepresentationError::ArtifactSizeOverflow { kind: "gzip Wasm" })?;
    let mut decoded = Vec::new();
    GzDecoder::new(wasm_gz)
        .take(wasm_size_bytes.saturating_add(1))
        .read_to_end(&mut decoded)
        .map_err(|source| RepresentationError::InvalidGzip { source })?;
    if decoded != wasm {
        return Err(RepresentationError::RepresentationMismatch);
    }
    Ok(QualifiedRepresentation {
        wasm_size_bytes,
        wasm_gz_size_bytes,
        wasm_sha256_hex: sha256_hex(wasm),
        wasm_gz_sha256_hex: sha256_hex(wasm_gz),
    })
}
