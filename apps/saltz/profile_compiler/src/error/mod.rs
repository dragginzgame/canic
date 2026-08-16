//! Module: error
//!
//! Responsibility: compose host I/O, JPEG and bounded Saltz input failures.
//! Does not own: operator remediation policy or runtime diagnostic allocation.
//! Boundary: callers retain typed error categories without a per-branch error catalogue.

use std::io;

use thiserror::Error;

///
/// CompileError
///
/// Top-level host compiler failure composed from stable subsystem categories.
///

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("waveform arithmetic exceeded its bounded integer representation")]
    Arithmetic,

    #[error("JPEG decode failed")]
    Decode(#[from] jpeg_decoder::Error),

    #[error(transparent)]
    Input(#[from] InputError),

    #[error("artifact I/O failed")]
    Io(#[from] io::Error),
}

///
/// InputError
///
/// Finite validation failures for the one selected Saltz image and its extracted trace.
///

#[derive(Debug, Eq, Error, PartialEq)]
pub enum InputError {
    #[error("selected image dimensions or pixel representation do not match the contract")]
    ImageShape,

    #[error("neon centreline extraction failed at source column {column}")]
    NeonExtraction { column: u16 },

    #[error("selected image produced a flat or otherwise invalid master trace")]
    ProfileShape,

    #[error("source image digest does not match the selected Saltz artifact")]
    SourceIdentity {
        actual_sha256: String,
        expected_sha256: &'static str,
    },
}
