//! Module: schedule
//!
//! Responsibility: map normalized points onto the exact rational 24-hour reference axis.
//! Does not own: Dashboard cadence qualification, forecasts, authorization, or execution.
//! Boundary: produces review buckets with checked integer arithmetic only.

use crate::{CompileError, MasterTrace};

pub const TARGET_AMPLITUDE_CYCLES_PER_SECOND: u128 = 50_000_000_000;
pub const TARGET_FLOOR_CYCLES_PER_SECOND: u128 = 100_000_000_000;
pub const RUN_DURATION_NS: u64 = 86_400_000_000_000;
const HEIGHT_PPM_SCALE: u128 = 1_000_000;
const NANOS_PER_SECOND: u128 = 1_000_000_000;

///
/// WaveBucket
///
/// One review-only target point on the exact rational reference time axis.
///

#[derive(Debug, Eq, PartialEq)]
pub struct WaveBucket {
    pub index: u32,
    pub bucket_start_offset_ns: u64,
    pub bucket_duration_ns: u64,
    pub source_x_px: u16,
    pub source_y_millipx: u32,
    pub height_ppm: u32,
    pub target_visible_cycles_per_second: u128,
}

pub fn compile_buckets(trace: &MasterTrace) -> Result<Vec<WaveBucket>, CompileError> {
    let point_count = u32::try_from(trace.points.len()).map_err(|_| CompileError::Arithmetic)?;
    if point_count == 0 {
        return Err(CompileError::Arithmetic);
    }

    trace
        .points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let index = u32::try_from(index).map_err(|_| CompileError::Arithmetic)?;
            let start = bucket_boundary_ns(index, point_count, RUN_DURATION_NS)?;
            let end = bucket_boundary_ns(
                index.checked_add(1).ok_or(CompileError::Arithmetic)?,
                point_count,
                RUN_DURATION_NS,
            )?;
            let amplitude = checked_mul_div_floor(
                TARGET_AMPLITUDE_CYCLES_PER_SECOND,
                u128::from(point.height_ppm),
                HEIGHT_PPM_SCALE,
            )?;

            Ok(WaveBucket {
                index,
                bucket_start_offset_ns: start,
                bucket_duration_ns: end.checked_sub(start).ok_or(CompileError::Arithmetic)?,
                source_x_px: point.x_px,
                source_y_millipx: point.source_y_millipx,
                height_ppm: point.height_ppm,
                target_visible_cycles_per_second: TARGET_FLOOR_CYCLES_PER_SECOND
                    .checked_add(amplitude)
                    .ok_or(CompileError::Arithmetic)?,
            })
        })
        .collect()
}

/// Derive one exact rational boundary without accumulated integer-duration drift.
pub fn bucket_boundary_ns(
    index: u32,
    point_count: u32,
    run_duration_ns: u64,
) -> Result<u64, CompileError> {
    if point_count == 0 || index > point_count {
        return Err(CompileError::Arithmetic);
    }
    let boundary = u128::from(index)
        .checked_mul(u128::from(run_duration_ns))
        .ok_or(CompileError::Arithmetic)?
        / u128::from(point_count);
    u64::try_from(boundary).map_err(|_| CompileError::Arithmetic)
}

/// Integrate the controlled burn implied by one constant background assumption.
pub fn integrate_controlled_burn(
    buckets: &[WaveBucket],
    background_cycles_per_second: u128,
) -> Result<u128, CompileError> {
    let mut cycles = 0_u128;
    let mut remainder = 0_u128;
    for bucket in buckets {
        let controlled_rate = bucket
            .target_visible_cycles_per_second
            .saturating_sub(background_cycles_per_second);
        let numerator = controlled_rate
            .checked_mul(u128::from(bucket.bucket_duration_ns))
            .and_then(|value| value.checked_add(remainder))
            .ok_or(CompileError::Arithmetic)?;
        cycles = cycles
            .checked_add(numerator / NANOS_PER_SECOND)
            .ok_or(CompileError::Arithmetic)?;
        remainder = numerator % NANOS_PER_SECOND;
    }
    Ok(cycles)
}

fn checked_mul_div_floor(
    value: u128,
    multiplier: u128,
    divisor: u128,
) -> Result<u128, CompileError> {
    if divisor == 0 {
        return Err(CompileError::Arithmetic);
    }
    let quotient = value / divisor;
    let remainder = value % divisor;
    quotient
        .checked_mul(multiplier)
        .and_then(|whole| {
            remainder
                .checked_mul(multiplier)
                .and_then(|fraction| whole.checked_add(fraction / divisor))
        })
        .ok_or(CompileError::Arithmetic)
}
