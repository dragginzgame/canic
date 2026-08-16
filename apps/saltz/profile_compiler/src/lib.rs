//! Module: saltz_profile_compiler
//!
//! Responsibility: compile the exact selected Saltz JPEG into review-only waveform artifacts.
//! Does not own: Dashboard qualification, run authorization, scheduling, or cycle burning.
//! Boundary: emits deterministic host evidence; it cannot produce an executable run yet.

mod error;
mod extract;
mod output;
mod schedule;
#[cfg(test)]
mod tests;

use std::{fmt::Write, path::Path};

use sha2::{Digest, Sha256};

pub use error::{CompileError, InputError};
pub use extract::{MasterTrace, WavePoint};
pub use schedule::{WaveBucket, bucket_boundary_ns, integrate_controlled_burn};

pub const SELECTED_SOURCE_SHA256: &str =
    "9cd20fa6de0ba665de8a956eb01dfe993af30c678e63fc03093ddd40b1acec06";

///
/// Compilation
///
/// Deterministic, non-executable output derived from the selected Saltz image.
///

#[derive(Debug, Eq, PartialEq)]
pub struct Compilation {
    pub source_sha256: String,
    pub trace: MasterTrace,
    pub buckets: Vec<WaveBucket>,
    pub zero_background_cycles: u128,
}

/// Compile the exact selected JPEG into a normalized master trace and reference buckets.
pub fn compile_selected_reference(bytes: &[u8]) -> Result<Compilation, CompileError> {
    let source_sha256 = sha256_hex(bytes);
    if source_sha256 != SELECTED_SOURCE_SHA256 {
        return Err(InputError::SourceIdentity {
            actual_sha256: source_sha256,
            expected_sha256: SELECTED_SOURCE_SHA256,
        }
        .into());
    }

    let image = extract::decode_rgb(bytes)?;
    let trace = extract::extract_master_trace(&image)?;
    let buckets = schedule::compile_buckets(&trace)?;
    let zero_background_cycles = integrate_controlled_burn(&buckets, 0)?;

    Ok(Compilation {
        source_sha256: SELECTED_SOURCE_SHA256.to_string(),
        trace,
        buckets,
        zero_background_cycles,
    })
}

/// Render the review CSV from one compilation.
#[must_use]
pub fn render_csv(compilation: &Compilation) -> String {
    output::render_csv(compilation)
}

/// Render a source overlay and expected Dashboard-shape preview from one compilation.
#[must_use]
pub fn render_preview_svg(compilation: &Compilation, source_path: &Path) -> String {
    output::render_preview_svg(compilation, source_path)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}
