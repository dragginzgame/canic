//! Module: waveform
//!
//! Responsibility: parse and verify the repository's numeric waveform authority.
//! Does not own: target scaling, smoothing, control policy, or execution.
//! Boundary: structural or digest drift rejects before simulation.

use std::{
    error::Error,
    fmt::{self, Display, Write},
    num::ParseIntError,
};

use sha2::{Digest, Sha256};

const CSV: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../docs/design/ideas/saltz/saltz_24h_waveform_floor_100B_860.csv"
));
const EXPECTED_DURATION_NS: u64 = 86_400_000_000_000;
const EXPECTED_POINT_COUNT: usize = 860;
const EXPECTED_SHA256: &str = "11fd75eb8fd0fed4f075d324051cc880db50619837bfe6c889fe9d654647d911";
const HEADER: &str = "index,bucket_start,bucket_start_offset_ns,bucket_duration_ns,source_x_px,source_y_px,height_px,height_ppm,target_visible_cycles_per_second,target_visible_Bcycles_per_second";

///
/// Waveform
///
/// Verified normalized authoring points consumed by the offline model.
///
pub struct Waveform {
    pub duration_ns: u64,
    pub heights_ppm: Vec<u32>,
    pub sha256: String,
}

///
/// WaveformError
///
/// Exact reason the checked-in numeric authority cannot be simulated.
///
#[derive(Debug)]
pub enum WaveformError {
    Digest,

    Duration,

    Header,

    Parse(ParseIntError),

    Row,
}

impl Display for WaveformError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Digest => formatter.write_str("waveform digest changed"),
            Self::Duration => formatter.write_str("waveform duration changed"),
            Self::Header => formatter.write_str("waveform header changed"),
            Self::Parse(error) => write!(formatter, "waveform integer is invalid: {error}"),
            Self::Row => formatter.write_str("waveform row structure changed"),
        }
    }
}

impl Error for WaveformError {}

impl From<ParseIntError> for WaveformError {
    fn from(error: ParseIntError) -> Self {
        Self::Parse(error)
    }
}

/// Load the digest-verified waveform embedded at compile time.
pub fn waveform() -> Result<Waveform, WaveformError> {
    let sha256 = sha256_hex(CSV.as_bytes());
    if sha256 != EXPECTED_SHA256 {
        return Err(WaveformError::Digest);
    }

    let mut lines = CSV.lines();
    if lines.next() != Some(HEADER) {
        return Err(WaveformError::Header);
    }

    let mut duration_ns = 0_u64;
    let mut normalized_points = Vec::with_capacity(EXPECTED_POINT_COUNT);
    for (expected_index, line) in lines.enumerate() {
        let columns: Vec<_> = line.split(',').collect();
        if columns.len() != 10
            || columns[0].parse::<usize>()? != expected_index
            || columns[2].parse::<u64>()? != duration_ns
            || columns[4].parse::<usize>()? != expected_index
        {
            return Err(WaveformError::Row);
        }

        duration_ns = duration_ns
            .checked_add(columns[3].parse::<u64>()?)
            .ok_or(WaveformError::Duration)?;
        let normalized_height = columns[7].parse::<u32>()?;
        if normalized_height > 1_000_000 {
            return Err(WaveformError::Row);
        }
        normalized_points.push(normalized_height);
    }

    if normalized_points.len() != EXPECTED_POINT_COUNT || duration_ns != EXPECTED_DURATION_NS {
        return Err(WaveformError::Duration);
    }

    Ok(Waveform {
        duration_ns,
        heights_ppm: normalized_points,
        sha256,
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        write!(encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_in_waveform_retains_exact_authority() {
        let waveform = waveform().expect("checked-in waveform should be valid");

        assert_eq!(waveform.heights_ppm.len(), EXPECTED_POINT_COUNT);
        assert_eq!(waveform.duration_ns, EXPECTED_DURATION_NS);
        assert_eq!(waveform.sha256, EXPECTED_SHA256);
    }
}
