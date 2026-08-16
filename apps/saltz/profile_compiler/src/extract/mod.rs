//! Module: extract
//!
//! Responsibility: decode the selected JPEG and extract one vertical neon centre per column.
//! Does not own: horizontal smoothing, artistic reconstruction, timing, or rate mapping.
//! Boundary: converts exact image pixels into one immutable normalized master trace.

use crate::{CompileError, InputError};
use std::{fmt::Write, io::Cursor};

use jpeg_decoder::{Decoder, PixelFormat};
use sha2::{Digest, Sha256};

pub const ALGORITHM_ID: &str = "saltz-neon-centreline-v1";
const EXPECTED_HEIGHT: u16 = 573;
const EXPECTED_WIDTH: u16 = 860;
const HEIGHT_PPM_SCALE: u32 = 1_000_000;
const MIN_NEON_SCORE: u32 = 512;
const ROI_BOTTOM_EXCLUSIVE: u16 = 170;
const ROI_TOP: u16 = 80;

///
/// WavePoint
///
/// One source-column centreline coordinate and normalized waveform height.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WavePoint {
    pub x_px: u16,
    pub source_y_millipx: u32,
    pub height_ppm: u32,
}

///
/// MasterTrace
///
/// Immutable 860-column Saltz centreline produced by the selected extraction algorithm.
///

#[derive(Debug, Eq, PartialEq)]
pub struct MasterTrace {
    pub algorithm_id: &'static str,
    pub highest_y_millipx: u32,
    pub lowest_y_millipx: u32,
    pub points_sha256: String,
    pub points: Vec<WavePoint>,
}

pub struct RgbImage {
    width: u16,
    height: u16,
    pixels: Vec<u8>,
}

pub fn decode_rgb(bytes: &[u8]) -> Result<RgbImage, CompileError> {
    let mut decoder = Decoder::new(Cursor::new(bytes));
    decoder.set_max_decoding_buffer_size(
        usize::from(EXPECTED_WIDTH) * usize::from(EXPECTED_HEIGHT) * 3,
    );
    let pixels = decoder.decode()?;
    let Some(info) = decoder.info() else {
        return Err(InputError::ImageShape.into());
    };
    if info.width != EXPECTED_WIDTH
        || info.height != EXPECTED_HEIGHT
        || info.pixel_format != PixelFormat::RGB24
        || pixels.len() != usize::from(EXPECTED_WIDTH) * usize::from(EXPECTED_HEIGHT) * 3
    {
        return Err(InputError::ImageShape.into());
    }

    Ok(RgbImage {
        width: info.width,
        height: info.height,
        pixels,
    })
}

pub fn extract_master_trace(image: &RgbImage) -> Result<MasterTrace, CompileError> {
    if image.width != EXPECTED_WIDTH || image.height != EXPECTED_HEIGHT {
        return Err(InputError::ImageShape.into());
    }

    let mut centres = Vec::with_capacity(usize::from(image.width));
    for x in 0..image.width {
        centres.push(extract_column_centre(image, x)?);
    }

    let highest_y_millipx = centres
        .iter()
        .copied()
        .min()
        .ok_or(InputError::ProfileShape)?;
    let lowest_y_millipx = centres
        .iter()
        .copied()
        .max()
        .ok_or(InputError::ProfileShape)?;
    let range = lowest_y_millipx
        .checked_sub(highest_y_millipx)
        .filter(|range| *range != 0)
        .ok_or(InputError::ProfileShape)?;

    let points = centres
        .into_iter()
        .enumerate()
        .map(|(x, source_y_millipx)| {
            let height_ppm = u64::from(lowest_y_millipx - source_y_millipx)
                .checked_mul(u64::from(HEIGHT_PPM_SCALE))
                .map(|value| value / u64::from(range))
                .and_then(|value| u32::try_from(value).ok())
                .ok_or(CompileError::Arithmetic)?;
            Ok(WavePoint {
                x_px: u16::try_from(x).map_err(|_| CompileError::Arithmetic)?,
                source_y_millipx,
                height_ppm,
            })
        })
        .collect::<Result<Vec<_>, CompileError>>()?;
    let points_sha256 = points_digest(&points);

    Ok(MasterTrace {
        algorithm_id: ALGORITHM_ID,
        highest_y_millipx,
        lowest_y_millipx,
        points_sha256,
        points,
    })
}

fn extract_column_centre(image: &RgbImage, x: u16) -> Result<u32, CompileError> {
    let mut scores = Vec::with_capacity(usize::from(ROI_BOTTOM_EXCLUSIVE - ROI_TOP));
    let mut maximum_score = 0;
    let mut maximum_offset = 0;
    for (offset, y) in (ROI_TOP..ROI_BOTTOM_EXCLUSIVE).enumerate() {
        let score = neon_score(image, x, y);
        scores.push(score);
        if score > maximum_score {
            maximum_score = score;
            maximum_offset = offset;
        }
    }
    if maximum_score < MIN_NEON_SCORE {
        return Err(InputError::NeonExtraction { column: x }.into());
    }

    let threshold = maximum_score * 3 / 4;
    let mut first = maximum_offset;
    while first > 0 && scores[first - 1] >= threshold {
        first -= 1;
    }
    let mut last = maximum_offset;
    while last + 1 < scores.len() && scores[last + 1] >= threshold {
        last += 1;
    }

    let mut weighted_y = 0_u64;
    let mut total_weight = 0_u64;
    for (offset, score) in scores.iter().enumerate().take(last + 1).skip(first) {
        let y = u64::from(ROI_TOP) + u64::try_from(offset).map_err(|_| CompileError::Arithmetic)?;
        let weight = u64::from(*score);
        weighted_y = weighted_y
            .checked_add(y.checked_mul(weight).ok_or(CompileError::Arithmetic)?)
            .ok_or(CompileError::Arithmetic)?;
        total_weight = total_weight
            .checked_add(weight)
            .ok_or(CompileError::Arithmetic)?;
    }

    weighted_y
        .checked_mul(1_000)
        .and_then(|value| value.checked_add(total_weight / 2))
        .and_then(|value| value.checked_div(total_weight))
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(CompileError::Arithmetic)
}

fn neon_score(image: &RgbImage, x: u16, y: u16) -> u32 {
    let offset = (usize::from(y) * usize::from(image.width) + usize::from(x)) * 3;
    let red = image.pixels[offset];
    let green = image.pixels[offset + 1];
    let blue = image.pixels[offset + 2];

    (u32::from(green) * 4 + u32::from(blue)).saturating_sub(u32::from(red.abs_diff(green)))
}

fn points_digest(points: &[WavePoint]) -> String {
    let mut digest = Sha256::new();
    digest.update(ALGORITHM_ID.as_bytes());
    for point in points {
        digest.update(point.x_px.to_be_bytes());
        digest.update(point.source_y_millipx.to_be_bytes());
        digest.update(point.height_ppm.to_be_bytes());
    }
    let digest = digest.finalize();
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}
