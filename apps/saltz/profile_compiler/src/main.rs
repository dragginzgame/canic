//! Module: saltz_profile_compiler binary
//!
//! Responsibility: write explicitly requested Saltz review artifacts to host paths.
//! Does not own: deployment, external observation, authorization, or cycle effects.
//! Boundary: delegates all compilation semantics to the deterministic library.

use std::{fs, path::PathBuf};

use clap::Parser;
use saltz_profile_compiler::{compile_selected_reference, render_csv, render_preview_svg};

///
/// Args
///
/// Explicit host paths for the selected source and generated review artifacts.
///

#[derive(Debug, Parser)]
#[command(name = "saltz-profile")]
struct Args {
    #[arg(long)]
    csv: PathBuf,

    #[arg(long)]
    preview: PathBuf,

    #[arg(long)]
    source: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let bytes = fs::read(&args.source)?;
    let compilation = compile_selected_reference(&bytes)?;
    fs::write(&args.csv, render_csv(&compilation))?;
    fs::write(
        &args.preview,
        render_preview_svg(&compilation, &args.source),
    )?;

    println!("source_sha256={}", compilation.source_sha256);
    println!("trace_sha256={}", compilation.trace.points_sha256);
    println!("points={}", compilation.trace.points.len());
    println!(
        "source_y_millipx={}..={}",
        compilation.trace.highest_y_millipx, compilation.trace.lowest_y_millipx
    );
    println!(
        "zero_background_cycles={}",
        compilation.zero_background_cycles
    );
    Ok(())
}
