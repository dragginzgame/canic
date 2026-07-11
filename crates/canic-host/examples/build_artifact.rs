use canic_host::canister_build::{
    CanisterBuildProfile, WorkspaceBuildContext, build_workspace_canister_artifact,
    copy_icp_wasm_output, print_workspace_build_context_once,
};
use canic_host::icp_config::resolve_icp_build_environment_from_root;
use canic_host::release_set::{config_path, icp_root, workspace_root};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(canister_name) = std::env::args().nth(1) else {
        return Err(
            "usage: cargo run -p canic-host --example build_artifact -- <canister-name>".into(),
        );
    };

    let profile = CanisterBuildProfile::current();
    let workspace_root = workspace_root()?;
    let icp_root = icp_root()?;
    let environment = std::env::var("ICP_ENVIRONMENT").unwrap_or_else(|_| "local".to_string());
    let build_network = resolve_icp_build_environment_from_root(&icp_root, &environment)?;
    let context = WorkspaceBuildContext {
        role: canister_name.clone(),
        profile,
        requested_profile: std::env::var("CANIC_WASM_PROFILE")
            .unwrap_or_else(|_| "unset".to_string()),
        environment,
        build_network: build_network.as_str().to_string(),
        config_path: config_path(&workspace_root),
        workspace_root,
        icp_root,
        local_replica: None,
    };
    print_workspace_build_context_once(&context)?;
    let output = build_workspace_canister_artifact(&context)?;
    copy_icp_wasm_output(&canister_name, &output)?;
    println!("{}", output.wasm_gz_path.display());
    Ok(())
}
