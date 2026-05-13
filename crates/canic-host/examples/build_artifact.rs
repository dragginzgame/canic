use canic_host::canister_build::{
    CanisterBuildProfile, build_current_workspace_canister_artifact, copy_icp_wasm_output,
    print_current_workspace_build_context_once,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(canister_name) = std::env::args().nth(1) else {
        return Err(
            "usage: cargo run -p canic-host --example build_artifact -- <canister-name>".into(),
        );
    };

    let profile = CanisterBuildProfile::current();
    print_current_workspace_build_context_once(profile)?;
    let output = build_current_workspace_canister_artifact(&canister_name, profile)?;
    copy_icp_wasm_output(&canister_name, &output)?;
    println!("{}", output.wasm_gz_path.display());
    Ok(())
}
