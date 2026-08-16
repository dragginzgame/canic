//! Module: executable
//!
//! Responsibility: compile the one immutable global-homepage burn schedule with integer arithmetic.
//! Does not own: timers, deployment, funding, caller authority, or execution.
//! Boundary: the emitted cycle amounts and digest are the only executable waveform authority.

use std::{
    error::Error,
    fmt::{self, Display},
};

use sha2::{Digest, Sha256};

use crate::Waveform;

pub const BACKGROUND_CYCLES_PER_SECOND: u64 = 30_000_000_000;
pub const CHART_STEP_SECONDS: u64 = 600;
pub const CONTROL_STEP_SECONDS: u64 = 100;
pub const EXECUTION_ALLOWANCE_CYCLES: u128 = 100_000_000_000;
pub const INITIAL_FUNDING_STEP_COUNT: usize = 42;
pub const KERNEL_WINDOW_SECONDS: u64 = 4_201;
pub const MAX_BURN_RATE_CYCLES_PER_SECOND: u64 = 500_000_000_000;
pub const MAX_TOTAL_BURN_CYCLES: u128 = 8_500_000_000_000_000;
pub const MIN_RETAINED_CYCLES: u128 = 1_000_000_000_000;
pub const PRE_ROLL_STEP_COUNT: usize = 42;
pub const TARGET_AMPLITUDE_CYCLES_PER_SECOND: u64 = 50_000_000_000;
pub const TARGET_FLOOR_CYCLES_PER_SECOND: u64 = 100_000_000_000;
pub const WAVEFORM_STEP_COUNT: usize = 864;

const _: () = assert!(INITIAL_FUNDING_STEP_COUNT <= PRE_ROLL_STEP_COUNT);

const HEIGHT_SCALE: u128 = 1_000_000;
const PLAN_DIGEST_DOMAIN: &[u8] = b"canic-saltz-global-executable-plan-v1";
const WEIGHT_FULL_SECONDS: u128 = 100;
const WEIGHT_REMAINDER_SECONDS: u128 = 1;

///
/// ExecutablePlan
///
/// Exact immutable schedule generated from the checked-in waveform authority.
///
pub struct ExecutablePlan {
    pub burn_cycles: Vec<u128>,
    pub digest: [u8; 32],
    pub initial_funding_cycles: u128,
    pub pre_roll_cycles: u128,
    pub run_cycles: u128,
    pub total_cycles: u128,
}

///
/// ExecutablePlanError
///
/// Exact reason the repository waveform cannot become the fixed executable schedule.
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutablePlanError {
    Arithmetic,

    Duration,

    Rate,

    Waveform,
}

impl Display for ExecutablePlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arithmetic => formatter.write_str("executable plan arithmetic overflowed"),
            Self::Duration => formatter.write_str("waveform duration does not match the plan"),
            Self::Rate => formatter.write_str("executable control rate exceeds the hard ceiling"),
            Self::Waveform => formatter.write_str("waveform has too few authoring points"),
        }
    }
}

impl Error for ExecutablePlanError {}

/// Compile the fixed global-homepage schedule without floating-point execution authority.
pub fn compile_executable_plan(waveform: &Waveform) -> Result<ExecutablePlan, ExecutablePlanError> {
    let expected_duration_ns = u64::try_from(WAVEFORM_STEP_COUNT)
        .ok()
        .and_then(|count| count.checked_mul(CONTROL_STEP_SECONDS))
        .and_then(|seconds| seconds.checked_mul(1_000_000_000))
        .ok_or(ExecutablePlanError::Arithmetic)?;
    if waveform.duration_ns != expected_duration_ns {
        return Err(ExecutablePlanError::Duration);
    }
    if waveform.heights_ppm.len() < 2 {
        return Err(ExecutablePlanError::Waveform);
    }

    let target = resample_control_target(waveform)?;
    let mut rates = vec![target[0]; PRE_ROLL_STEP_COUNT];
    for desired in target {
        let past_weighted = past_weighted_rate(&rates)?;
        let desired_weighted = u128::from(desired)
            .checked_mul(u128::from(KERNEL_WINDOW_SECONDS))
            .ok_or(ExecutablePlanError::Arithmetic)?;
        let requested_weighted = desired_weighted.saturating_sub(past_weighted);
        let requested_rate = requested_weighted
            .checked_add(WEIGHT_FULL_SECONDS - 1)
            .ok_or(ExecutablePlanError::Arithmetic)?
            / WEIGHT_FULL_SECONDS;
        let requested_rate =
            u64::try_from(requested_rate).map_err(|_| ExecutablePlanError::Arithmetic)?;
        if requested_rate > MAX_BURN_RATE_CYCLES_PER_SECOND {
            return Err(ExecutablePlanError::Rate);
        }
        rates.push(requested_rate);
    }

    let burn_cycles = rates
        .iter()
        .map(|rate| {
            u128::from(*rate)
                .checked_mul(u128::from(CONTROL_STEP_SECONDS))
                .ok_or(ExecutablePlanError::Arithmetic)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let pre_roll_cycles = checked_sum(&burn_cycles[..PRE_ROLL_STEP_COUNT])?;
    let initial_funding_cycles = checked_sum(&burn_cycles[..INITIAL_FUNDING_STEP_COUNT])?;
    let run_cycles = checked_sum(&burn_cycles[PRE_ROLL_STEP_COUNT..])?;
    let total_cycles = pre_roll_cycles
        .checked_add(run_cycles)
        .ok_or(ExecutablePlanError::Arithmetic)?;
    if total_cycles > MAX_TOTAL_BURN_CYCLES {
        return Err(ExecutablePlanError::Rate);
    }
    let digest = plan_digest(&burn_cycles, total_cycles);

    Ok(ExecutablePlan {
        burn_cycles,
        digest,
        initial_funding_cycles,
        pre_roll_cycles,
        run_cycles,
        total_cycles,
    })
}

fn resample_control_target(waveform: &Waveform) -> Result<Vec<u64>, ExecutablePlanError> {
    let source_last = waveform.heights_ppm.len() - 1;
    let denominator = u128::try_from(WAVEFORM_STEP_COUNT)
        .map_err(|_| ExecutablePlanError::Arithmetic)?
        .checked_mul(2)
        .ok_or(ExecutablePlanError::Arithmetic)?;

    (0..WAVEFORM_STEP_COUNT)
        .map(|index| {
            let midpoint = u128::try_from(index)
                .map_err(|_| ExecutablePlanError::Arithmetic)?
                .checked_mul(2)
                .and_then(|value| value.checked_add(1))
                .ok_or(ExecutablePlanError::Arithmetic)?;
            let scaled = midpoint
                .checked_mul(
                    u128::try_from(source_last).map_err(|_| ExecutablePlanError::Arithmetic)?,
                )
                .ok_or(ExecutablePlanError::Arithmetic)?;
            let left = usize::try_from(scaled / denominator)
                .map_err(|_| ExecutablePlanError::Arithmetic)?;
            let remainder = scaled % denominator;
            let right = (left + 1).min(source_last);
            let left_height = u128::from(waveform.heights_ppm[left]);
            let height_numerator = left_height
                .checked_mul(denominator - remainder)
                .and_then(|value| {
                    u128::from(waveform.heights_ppm[right])
                        .checked_mul(remainder)
                        .and_then(|right| value.checked_add(right))
                })
                .ok_or(ExecutablePlanError::Arithmetic)?;
            let height_ppm = height_numerator / denominator;
            let visible_rate = u128::from(TARGET_FLOOR_CYCLES_PER_SECOND)
                .checked_add(
                    u128::from(TARGET_AMPLITUDE_CYCLES_PER_SECOND)
                        .checked_mul(height_ppm)
                        .ok_or(ExecutablePlanError::Arithmetic)?
                        / HEIGHT_SCALE,
                )
                .ok_or(ExecutablePlanError::Arithmetic)?;
            let control_rate =
                visible_rate.saturating_sub(u128::from(BACKGROUND_CYCLES_PER_SECOND));
            u64::try_from(control_rate).map_err(|_| ExecutablePlanError::Arithmetic)
        })
        .collect()
}

fn past_weighted_rate(rates: &[u64]) -> Result<u128, ExecutablePlanError> {
    let history_end = rates.len();
    if history_end < PRE_ROLL_STEP_COUNT {
        return Err(ExecutablePlanError::Duration);
    }

    let full = (1..PRE_ROLL_STEP_COUNT).try_fold(0_u128, |sum, lag| {
        u128::from(rates[history_end - lag])
            .checked_mul(WEIGHT_FULL_SECONDS)
            .and_then(|value| sum.checked_add(value))
            .ok_or(ExecutablePlanError::Arithmetic)
    })?;
    u128::from(rates[history_end - PRE_ROLL_STEP_COUNT])
        .checked_mul(WEIGHT_REMAINDER_SECONDS)
        .and_then(|value| full.checked_add(value))
        .ok_or(ExecutablePlanError::Arithmetic)
}

fn checked_sum(values: &[u128]) -> Result<u128, ExecutablePlanError> {
    values.iter().try_fold(0_u128, |sum, value| {
        sum.checked_add(*value)
            .ok_or(ExecutablePlanError::Arithmetic)
    })
}

fn plan_digest(burn_cycles: &[u128], total_cycles: u128) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(PLAN_DIGEST_DOMAIN);
    for value in [
        u128::from(BACKGROUND_CYCLES_PER_SECOND),
        u128::from(CHART_STEP_SECONDS),
        u128::from(CONTROL_STEP_SECONDS),
        EXECUTION_ALLOWANCE_CYCLES,
        u128::try_from(INITIAL_FUNDING_STEP_COUNT).expect("bounded constant"),
        u128::from(KERNEL_WINDOW_SECONDS),
        u128::from(MAX_BURN_RATE_CYCLES_PER_SECOND),
        MAX_TOTAL_BURN_CYCLES,
        MIN_RETAINED_CYCLES,
        u128::try_from(PRE_ROLL_STEP_COUNT).expect("bounded constant"),
        u128::from(TARGET_AMPLITUDE_CYCLES_PER_SECOND),
        u128::from(TARGET_FLOOR_CYCLES_PER_SECOND),
        u128::try_from(WAVEFORM_STEP_COUNT).expect("bounded constant"),
        total_cycles,
    ] {
        hasher.update(value.to_be_bytes());
    }
    for amount in burn_cycles {
        hasher.update(amount.to_be_bytes());
    }
    hasher.finalize().into()
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waveform;

    #[test]
    fn executable_plan_is_exact_bounded_and_repeatable() {
        let waveform = waveform().expect("waveform");
        let first = compile_executable_plan(&waveform).expect("plan");
        let second = compile_executable_plan(&waveform).expect("plan");

        assert_eq!(
            first.burn_cycles.len(),
            PRE_ROLL_STEP_COUNT + WAVEFORM_STEP_COUNT
        );
        assert_eq!(first.digest, second.digest);
        assert_eq!(first.burn_cycles, second.burn_cycles);
        assert_eq!(
            first.initial_funding_cycles,
            first.burn_cycles[..INITIAL_FUNDING_STEP_COUNT]
                .iter()
                .sum::<u128>()
        );
        assert_eq!(first.total_cycles, first.pre_roll_cycles + first.run_cycles);
        assert!(first.total_cycles <= MAX_TOTAL_BURN_CYCLES);
        assert!(first.burn_cycles.iter().all(|amount| {
            *amount
                <= u128::from(MAX_BURN_RATE_CYCLES_PER_SECOND) * u128::from(CONTROL_STEP_SECONDS)
        }));
    }
}
