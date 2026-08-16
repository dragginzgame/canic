//! Module: model
//!
//! Responsibility: forward-model a bounded non-negative controller through one smoothing kernel.
//! Does not own: platform guarantees, background forecasts, or executable cycle amounts.
//! Boundary: every result is explicitly provisional until the complete pulse tail is frozen.

#![expect(
    clippy::cast_precision_loss,
    reason = "the explicitly non-authoritative offline model operates in Bcycles/second"
)]

use std::{
    error::Error,
    fmt::{self, Display},
};

use crate::Waveform;

const BILLION: f64 = 1_000_000_000.0;
const NANOS_PER_SECOND: u64 = 1_000_000_000;

///
/// SimulationConfig
///
/// Exact analysis inputs supplied by the operator for one offline proposal.
///
pub struct SimulationConfig {
    pub background_cycles_per_second: u64,
    pub chart_step_seconds: u64,
    pub control_step_seconds: u64,
    pub kernel_window_seconds: u64,
    pub max_burn_rate_cycles_per_second: u64,
    pub max_total_burn_cycles: u128,
    pub target_amplitude_cycles_per_second: u64,
    pub target_floor_cycles_per_second: u64,
}

///
/// ChartPoint
///
/// One current-frontend 1D sample projected from the control schedule.
///
pub struct ChartPoint {
    pub control_cycles_per_second: f64,
    pub index: usize,
    pub offset_seconds: u64,
    pub predicted_visible_cycles_per_second: f64,
    pub target_visible_cycles_per_second: f64,
}

///
/// SimulationReport
///
/// Bounded fit, cost and constraint evidence for one offline proposal.
///
pub struct SimulationReport {
    pub chart_correlation: f64,
    pub chart_mae_cycles_per_second: f64,
    pub chart_max_error_cycles_per_second: f64,
    pub chart_points: Vec<ChartPoint>,
    pub control_points: Vec<f64>,
    pub kernel_window_seconds: u64,
    pub max_total_burn_cycles: u128,
    pub nonnegative_constraint_steps: usize,
    pub pre_roll_cycles: u128,
    pub rate_cap_constraint_steps: usize,
    pub run_cycles: u128,
    pub total_cycles: u128,
    pub within_total_cap: bool,
}

impl SimulationReport {
    /// Return whether the provisional controller reproduces every chart point within constraints.
    #[must_use]
    pub fn proposal_exact(&self) -> bool {
        self.within_total_cap
            && self.nonnegative_constraint_steps == 0
            && self.rate_cap_constraint_steps == 0
            && self.chart_max_error_cycles_per_second < 1.0
    }
}

///
/// SimulationError
///
/// Bounded input rejection before offline modelling begins.
///
#[derive(Debug, Eq, PartialEq)]
pub enum SimulationError {
    ChartStep,

    ControlStep,

    Duration,

    Kernel,

    Rate,
}

impl Display for SimulationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChartStep => formatter.write_str("chart step must be a multiple of control step"),
            Self::ControlStep => formatter.write_str("control step must be nonzero"),
            Self::Duration => formatter.write_str("waveform duration must divide by control step"),
            Self::Kernel => formatter.write_str("kernel window must be at least one control step"),
            Self::Rate => formatter.write_str("target rate or rate ceiling is invalid"),
        }
    }
}

impl Error for SimulationError {}

/// Simulate a non-negative causal inverse against a rectangular smoothing approximation.
pub fn simulate(
    waveform: &Waveform,
    config: &SimulationConfig,
) -> Result<SimulationReport, SimulationError> {
    validate(waveform, config)?;

    let control_count = usize::try_from(duration_seconds(waveform) / config.control_step_seconds)
        .map_err(|_| SimulationError::Duration)?;
    let target = resample_target(waveform, config, control_count);
    let weights = rectangular_kernel(config);
    let pre_roll = vec![target[0]; weights.len()];
    let (control, predicted, nonnegative_constraint_steps, rate_cap_constraint_steps) =
        invert_nonnegative(&target, &pre_roll, &weights, config);
    let chart_points = chart_points(&target, &predicted, &control, config);
    let (chart_mae, chart_max_error, chart_correlation) = chart_metrics(&chart_points);
    let pre_roll_cycles = integrate_cycles(&pre_roll, config.control_step_seconds);
    let run_cycles = integrate_cycles(&control, config.control_step_seconds);
    let total_cycles = pre_roll_cycles.saturating_add(run_cycles);

    Ok(SimulationReport {
        chart_correlation,
        chart_mae_cycles_per_second: chart_mae * BILLION,
        chart_max_error_cycles_per_second: chart_max_error * BILLION,
        chart_points,
        control_points: control,
        kernel_window_seconds: config.kernel_window_seconds,
        max_total_burn_cycles: config.max_total_burn_cycles,
        nonnegative_constraint_steps,
        pre_roll_cycles,
        rate_cap_constraint_steps,
        run_cycles,
        total_cycles,
        within_total_cap: total_cycles <= config.max_total_burn_cycles,
    })
}

fn validate(waveform: &Waveform, config: &SimulationConfig) -> Result<(), SimulationError> {
    if config.control_step_seconds == 0 {
        return Err(SimulationError::ControlStep);
    }
    if config.chart_step_seconds == 0
        || !config
            .chart_step_seconds
            .is_multiple_of(config.control_step_seconds)
    {
        return Err(SimulationError::ChartStep);
    }
    if config.kernel_window_seconds < config.control_step_seconds {
        return Err(SimulationError::Kernel);
    }
    if usize::try_from(config.kernel_window_seconds / config.control_step_seconds).is_err() {
        return Err(SimulationError::Kernel);
    }
    if !duration_seconds(waveform).is_multiple_of(config.control_step_seconds) {
        return Err(SimulationError::Duration);
    }
    if config.target_amplitude_cycles_per_second == 0
        || config.max_burn_rate_cycles_per_second == 0
        || config
            .target_floor_cycles_per_second
            .checked_add(config.target_amplitude_cycles_per_second)
            .is_none()
    {
        return Err(SimulationError::Rate);
    }
    Ok(())
}

const fn duration_seconds(waveform: &Waveform) -> u64 {
    waveform.duration_ns / NANOS_PER_SECOND
}

fn resample_target(
    waveform: &Waveform,
    config: &SimulationConfig,
    control_count: usize,
) -> Vec<f64> {
    let background = config.background_cycles_per_second as f64 / BILLION;
    let amplitude = config.target_amplitude_cycles_per_second as f64 / BILLION;
    let floor = config.target_floor_cycles_per_second as f64 / BILLION;
    let last_source = waveform.heights_ppm.len() - 1;

    (0..control_count)
        .map(|index| {
            let numerator = index * 2 + 1;
            let denominator = control_count * 2;
            let scaled = numerator * last_source;
            let left = scaled / denominator;
            let remainder = scaled % denominator;
            let right = (left + 1).min(last_source);
            let left_height = f64::from(waveform.heights_ppm[left]);
            let right_height = f64::from(waveform.heights_ppm[right]);
            let height =
                left_height + (right_height - left_height) * remainder as f64 / denominator as f64;
            (floor + amplitude * height / 1_000_000.0 - background).max(0.0)
        })
        .collect()
}

fn rectangular_kernel(config: &SimulationConfig) -> Vec<f64> {
    let full_steps = usize::try_from(config.kernel_window_seconds / config.control_step_seconds)
        .expect("kernel step count was validated");
    let remainder = config.kernel_window_seconds % config.control_step_seconds;
    let mut weights =
        vec![config.control_step_seconds as f64 / config.kernel_window_seconds as f64; full_steps];
    if remainder > 0 {
        weights.push(remainder as f64 / config.kernel_window_seconds as f64);
    }
    weights
}

fn invert_nonnegative(
    target: &[f64],
    pre_roll: &[f64],
    weights: &[f64],
    config: &SimulationConfig,
) -> (Vec<f64>, Vec<f64>, usize, usize) {
    let max_rate = config.max_burn_rate_cycles_per_second as f64 / BILLION;
    let mut history = pre_roll.to_vec();
    let mut control = Vec::with_capacity(target.len());
    let mut predicted = Vec::with_capacity(target.len());
    let mut nonnegative_constraint_steps = 0;
    let mut rate_cap_constraint_steps = 0;

    for desired in target {
        let past = weights
            .iter()
            .enumerate()
            .skip(1)
            .map(|(lag, weight)| weight * history[history.len() - lag])
            .sum::<f64>();
        let requested = (desired - past) / weights[0];
        let bounded = if requested < 0.0 {
            nonnegative_constraint_steps += 1;
            0.0
        } else if requested > max_rate {
            rate_cap_constraint_steps += 1;
            max_rate
        } else {
            requested
        };

        history.push(bounded);
        let actual = weights
            .iter()
            .enumerate()
            .map(|(lag, weight)| weight * history[history.len() - 1 - lag])
            .sum();
        control.push(bounded);
        predicted.push(actual);
    }

    (
        control,
        predicted,
        nonnegative_constraint_steps,
        rate_cap_constraint_steps,
    )
}

fn chart_points(
    target: &[f64],
    predicted: &[f64],
    control: &[f64],
    config: &SimulationConfig,
) -> Vec<ChartPoint> {
    let stride = usize::try_from(config.chart_step_seconds / config.control_step_seconds)
        .expect("chart stride is bounded by the validated waveform duration");
    let background = config.background_cycles_per_second as f64 / BILLION;

    (stride - 1..target.len())
        .step_by(stride)
        .enumerate()
        .map(|(index, control_index)| ChartPoint {
            control_cycles_per_second: control[control_index] * BILLION,
            index,
            offset_seconds: (control_index as u64 + 1) * config.control_step_seconds,
            predicted_visible_cycles_per_second: (predicted[control_index] + background) * BILLION,
            target_visible_cycles_per_second: (target[control_index] + background) * BILLION,
        })
        .collect()
}

fn chart_metrics(points: &[ChartPoint]) -> (f64, f64, f64) {
    let count = points.len() as f64;
    let target_mean = points
        .iter()
        .map(|point| point.target_visible_cycles_per_second / BILLION)
        .sum::<f64>()
        / count;
    let predicted_mean = points
        .iter()
        .map(|point| point.predicted_visible_cycles_per_second / BILLION)
        .sum::<f64>()
        / count;
    let mut absolute_error = 0.0_f64;
    let mut covariance = 0.0_f64;
    let mut max_error = 0.0_f64;
    let mut predicted_variance = 0.0_f64;
    let mut target_variance = 0.0_f64;

    for point in points {
        let target = point.target_visible_cycles_per_second / BILLION;
        let predicted = point.predicted_visible_cycles_per_second / BILLION;
        let error = (target - predicted).abs();
        absolute_error += error;
        max_error = max_error.max(error);
        covariance = (target - target_mean).mul_add(predicted - predicted_mean, covariance);
        target_variance += (target - target_mean).powi(2);
        predicted_variance += (predicted - predicted_mean).powi(2);
    }

    let correlation = if target_variance == 0.0 || predicted_variance == 0.0 {
        0.0
    } else {
        covariance / (target_variance * predicted_variance).sqrt()
    };
    (absolute_error / count, max_error, correlation)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "rates are finite, non-negative, bounded model outputs rounded to whole cycles"
)]
fn integrate_cycles(rates_billions: &[f64], step_seconds: u64) -> u128 {
    rates_billions
        .iter()
        .map(|rate| (rate * BILLION * step_seconds as f64).round() as u128)
        .sum()
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waveform;

    fn config() -> SimulationConfig {
        SimulationConfig {
            background_cycles_per_second: 625_000_000,
            chart_step_seconds: 600,
            control_step_seconds: 100,
            kernel_window_seconds: 4_531,
            max_burn_rate_cycles_per_second: 20_000_000_000,
            max_total_burn_cycles: 200_000_000_000_000,
            target_amplitude_cycles_per_second: 1_500_000_000,
            target_floor_cycles_per_second: 1_000_000_000,
        }
    }

    #[test]
    fn proposal_is_bounded_and_reports_nonnegative_limit() {
        let report = simulate(&waveform().expect("waveform"), &config()).expect("simulation");

        assert!(report.within_total_cap);
        assert!(report.nonnegative_constraint_steps > 0);
        assert_eq!(report.rate_cap_constraint_steps, 0);
        assert!(!report.proposal_exact());
    }

    #[test]
    fn total_cap_is_observation_not_execution_authority() {
        let mut config = config();
        config.max_total_burn_cycles = 1;

        let report = simulate(&waveform().expect("waveform"), &config).expect("simulation");

        assert!(!report.within_total_cap);
        assert!(!report.proposal_exact());
    }

    #[test]
    fn invalid_chart_step_is_rejected() {
        let mut config = config();
        config.chart_step_seconds = 601;

        let error = simulate(&waveform().expect("waveform"), &config)
            .err()
            .expect("invalid step should reject");

        assert_eq!(error, SimulationError::ChartStep);
    }
}
