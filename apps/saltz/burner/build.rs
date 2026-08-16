//! Build script: saltz_burner
//!
//! Responsibility: embed the one digest-bound executable schedule as Rust constants.
//! Does not own: runtime authorization, funding, scheduling, or burn execution.
//! Boundary: compilation fails if the checked-in waveform cannot satisfy the fixed envelope.

use std::{env, error::Error, fmt::Write as _, fs, path::PathBuf};

use saltz_simulator::{
    BACKGROUND_CYCLES_PER_SECOND, CHART_STEP_SECONDS, CONTROL_STEP_SECONDS,
    INITIAL_FUNDING_STEP_COUNT, KERNEL_WINDOW_SECONDS, MAX_BURN_RATE_CYCLES_PER_SECOND,
    MAX_TOTAL_BURN_CYCLES, PRE_ROLL_STEP_COUNT, TARGET_AMPLITUDE_CYCLES_PER_SECOND,
    TARGET_FLOOR_CYCLES_PER_SECOND, WAVEFORM_STEP_COUNT, compile_executable_plan, waveform,
};

fn main() -> Result<(), Box<dyn Error>> {
    let waveform = waveform()?;
    let plan = compile_executable_plan(&waveform)?;
    let mut generated = String::new();

    writeln!(
        generated,
        "pub const BACKGROUND_CYCLES_PER_SECOND: u64 = {BACKGROUND_CYCLES_PER_SECOND};"
    )?;
    writeln!(
        generated,
        "pub const BURN_CYCLES: [u128; {}] = {:?};",
        plan.burn_cycles.len(),
        plan.burn_cycles
    )?;
    writeln!(
        generated,
        "pub const CHART_STEP_SECONDS: u64 = {CHART_STEP_SECONDS};"
    )?;
    writeln!(
        generated,
        "pub const CONTROL_STEP_SECONDS: u64 = {CONTROL_STEP_SECONDS};"
    )?;
    writeln!(
        generated,
        "pub const INITIAL_FUNDING_CYCLES: u128 = {};",
        plan.initial_funding_cycles
    )?;
    writeln!(
        generated,
        "pub const INITIAL_FUNDING_STEP_COUNT: u32 = {INITIAL_FUNDING_STEP_COUNT};"
    )?;
    writeln!(
        generated,
        "pub const KERNEL_WINDOW_SECONDS: u64 = {KERNEL_WINDOW_SECONDS};"
    )?;
    writeln!(
        generated,
        "pub const MAX_BURN_RATE_CYCLES_PER_SECOND: u64 = {MAX_BURN_RATE_CYCLES_PER_SECOND};"
    )?;
    writeln!(
        generated,
        "pub const MAX_TOTAL_BURN_CYCLES: u128 = {MAX_TOTAL_BURN_CYCLES};"
    )?;
    writeln!(
        generated,
        "pub const PLAN_DIGEST: [u8; 32] = {:?};",
        plan.digest
    )?;
    writeln!(
        generated,
        "pub const PRE_ROLL_CYCLES: u128 = {};",
        plan.pre_roll_cycles
    )?;
    writeln!(
        generated,
        "pub const PRE_ROLL_STEP_COUNT: u32 = {PRE_ROLL_STEP_COUNT};"
    )?;
    writeln!(
        generated,
        "pub const RUN_CYCLES: u128 = {};",
        plan.run_cycles
    )?;
    writeln!(
        generated,
        "pub const TARGET_AMPLITUDE_CYCLES_PER_SECOND: u64 = {TARGET_AMPLITUDE_CYCLES_PER_SECOND};"
    )?;
    writeln!(
        generated,
        "pub const TARGET_FLOOR_CYCLES_PER_SECOND: u64 = {TARGET_FLOOR_CYCLES_PER_SECOND};"
    )?;
    writeln!(
        generated,
        "pub const TOTAL_BURN_CYCLES: u128 = {};",
        plan.total_cycles
    )?;
    writeln!(
        generated,
        "pub const WAVEFORM_STEP_COUNT: u32 = {WAVEFORM_STEP_COUNT};"
    )?;

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    fs::write(out_dir.join("executable_plan.rs"), generated)?;
    println!("cargo:rerun-if-changed=../simulator/src");
    println!(
        "cargo:rerun-if-changed=../../../docs/design/ideas/saltz/saltz_24h_waveform_floor_100B_860.csv"
    );
    Ok(())
}
