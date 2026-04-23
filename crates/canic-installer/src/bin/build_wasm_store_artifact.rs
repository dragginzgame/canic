use canic_installer::bootstrap_store::{
    BootstrapWasmStoreBuildProfile, build_current_workspace_bootstrap_wasm_store_artifact,
};

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

// Build the implicit bootstrap `wasm_store` artifact for the current workspace.
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let output = build_current_workspace_bootstrap_wasm_store_artifact(
        BootstrapWasmStoreBuildProfile::current(),
    )?;
    println!("{}", output.wasm_gz_path.display());
    Ok(())
}
