//! Generate the supported User Directory Wire manifests for baseline review.
use clap::Parser;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Parser)]
struct Args {
    /// Empty candidate directory supplied by the baseline checker.
    output: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = Args::parse().output;
    let config =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/examples/user-directory/canic.toml");
    let (_, candidate) = canic_host::wire_manifest(&config, None)?;
    fs::write(output.join("contracts.json"), candidate)?;
    Ok(())
}
