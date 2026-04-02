use canic_installer::canister_build::{
    CanisterBuildProfile, build_current_workspace_canister_artifact,
};

// Run the public visible-canister build entrypoint and print the `.wasm.gz` path.
fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

// Build one visible Canic canister artifact for the current workspace.
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let canister_name = std::env::args()
        .nth(1)
        .ok_or_else(|| "usage: canic-build-canister-artifact <canister_name>".to_string())?;
    let output =
        build_current_workspace_canister_artifact(&canister_name, CanisterBuildProfile::current())?;
    println!("{}", output.wasm_gz_path.display());
    Ok(())
}
