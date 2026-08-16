//! Binary: saltz_simulator
//!
//! Responsibility: report one bounded, offline waveform-control proposal.
//! Does not own: mainnet authority, canister calls, funding, or executable run plans.
//! Boundary: reads compiled repository input and writes analysis only to stdout.

use std::{error::Error, process::ExitCode};

use clap::Parser;
use saltz_simulator::{SimulationConfig, simulate, waveform};

#[derive(Parser)]
#[command(about = "Model a bounded cycle-burn waveform without external effects")]
struct Args {
    /// Assumed unrelated visible burn rate in cycles per second.
    #[arg(long, default_value_t = 625_000_000)]
    background_cycles_per_second: u64,

    /// Dashboard chart sample spacing in seconds.
    #[arg(long, default_value_t = 600)]
    chart_step_seconds: u64,

    /// Proposed burner control spacing in seconds.
    #[arg(long, default_value_t = 100)]
    control_step_seconds: u64,

    /// Print the 1D chart-point comparison after the summary.
    #[arg(long)]
    emit_chart: bool,

    /// Measured normalization denominator for one pulse's displayed gain.
    #[arg(long, default_value_t = 4_201)]
    kernel_gain_seconds: u64,

    /// Measured duration for which one pulse remains visible.
    #[arg(long, default_value_t = 3_600)]
    kernel_support_seconds: u64,

    /// Maximum permitted instantaneous control rate in cycles per second.
    #[arg(long, default_value_t = 20_000_000_000)]
    max_burn_rate_cycles_per_second: u64,

    /// Hard proposal ceiling, including pre-roll, in cycles.
    #[arg(long)]
    max_total_burn_cycles: u128,

    /// Visible waveform relief above its floor in cycles per second.
    #[arg(long, default_value_t = 1_500_000_000)]
    target_amplitude_cycles_per_second: u64,

    /// Visible waveform floor in cycles per second.
    #[arg(long, default_value_t = 1_000_000_000)]
    target_floor_cycles_per_second: u64,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("saltz simulator rejected the proposal: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let waveform = waveform()?;
    let report = simulate(
        &waveform,
        &SimulationConfig {
            background_cycles_per_second: args.background_cycles_per_second,
            chart_step_seconds: args.chart_step_seconds,
            control_step_seconds: args.control_step_seconds,
            kernel_gain_seconds: args.kernel_gain_seconds,
            kernel_support_seconds: args.kernel_support_seconds,
            max_burn_rate_cycles_per_second: args.max_burn_rate_cycles_per_second,
            max_total_burn_cycles: args.max_total_burn_cycles,
            target_amplitude_cycles_per_second: args.target_amplitude_cycles_per_second,
            target_floor_cycles_per_second: args.target_floor_cycles_per_second,
        },
    )?;

    println!("model=measured_gain_and_support_kernel");
    println!("waveform_sha256={}", waveform.sha256);
    println!("waveform_points={}", waveform.heights_ppm.len());
    println!("control_points={}", report.control_points.len());
    println!("chart_points={}", report.chart_points.len());
    println!("kernel_gain_seconds={}", report.kernel_gain_seconds);
    println!("kernel_support_seconds={}", report.kernel_support_seconds);
    println!("pre_roll_cycles={}", report.pre_roll_cycles);
    println!("run_cycles={}", report.run_cycles);
    println!("total_cycles={}", report.total_cycles);
    println!("max_total_burn_cycles={}", report.max_total_burn_cycles);
    println!("within_total_cap={}", report.within_total_cap);
    println!(
        "nonnegative_constraint_steps={}",
        report.nonnegative_constraint_steps
    );
    println!(
        "peak_control_cycles_per_second={:.3}",
        report.peak_control_cycles_per_second
    );
    println!(
        "rate_cap_constraint_steps={}",
        report.rate_cap_constraint_steps
    );
    println!(
        "chart_mae_cycles_per_second={:.3}",
        report.chart_mae_cycles_per_second
    );
    println!(
        "chart_max_error_cycles_per_second={:.3}",
        report.chart_max_error_cycles_per_second
    );
    println!("chart_correlation={:.6}", report.chart_correlation);
    println!("proposal_exact={}", report.proposal_exact());
    println!("executable_authority=false");

    if args.emit_chart {
        println!(
            "index,offset_seconds,target_visible_cycles_per_second,predicted_visible_cycles_per_second,control_cycles_per_second"
        );
        for point in &report.chart_points {
            println!(
                "{},{},{:.3},{:.3},{:.3}",
                point.index,
                point.offset_seconds,
                point.target_visible_cycles_per_second,
                point.predicted_visible_cycles_per_second,
                point.control_cycles_per_second,
            );
        }
    }

    Ok(())
}
