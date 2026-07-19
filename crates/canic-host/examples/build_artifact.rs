use canic_host::canister_build::{
    CanisterBuildProfile, WorkspaceBuildContext, build_workspace_canister_artifact,
    copy_icp_wasm_output, print_workspace_build_context_once,
};
use canic_host::icp_config::resolve_icp_build_network_from_root;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let (
        Some(canister_name),
        Some(profile),
        Some(workspace_root),
        Some(icp_root),
        Some(config_path),
    ) = (
        args.next(),
        args.next(),
        args.next(),
        args.next(),
        args.next(),
    )
    else {
        return Err(
            "usage: cargo run -p canic-host --example build_artifact -- <canister-name> <debug|fast|release> <workspace-root> <icp-root> <config-path> [--refresh-wasm-store-did]"
                .into(),
        );
    };
    let refresh_canonical_wasm_store_did = match args.next().as_deref() {
        None => false,
        Some("--refresh-wasm-store-did") if canister_name == "wasm_store" => true,
        Some("--refresh-wasm-store-did") => {
            return Err("--refresh-wasm-store-did requires canister-name wasm_store".into());
        }
        Some(_) => return Err("unknown build_artifact argument".into()),
    };
    if args.next().is_some() {
        return Err("build_artifact accepts at most six arguments".into());
    }
    let profile = profile.parse::<CanisterBuildProfile>()?;

    let workspace_root = PathBuf::from(workspace_root).canonicalize()?;
    let icp_root = PathBuf::from(icp_root).canonicalize()?;
    let config_path = resolve_path(&workspace_root, &config_path).canonicalize()?;
    let environment = std::env::var("ICP_ENVIRONMENT").unwrap_or_else(|_| "local".to_string());
    let build_network = resolve_icp_build_network_from_root(&icp_root, &environment)?;
    let context = WorkspaceBuildContext {
        role: canister_name.clone(),
        profile,
        environment,
        build_network,
        config_path,
        workspace_root,
        icp_root,
        local_replica: None,
        refresh_canonical_wasm_store_did,
    };
    print_workspace_build_context_once(&context)?;
    let output = build_workspace_canister_artifact(&context)?;
    copy_icp_wasm_output(&canister_name, &output)?;
    println!("{}", output.wasm_gz_path.display());
    Ok(())
}

fn resolve_path(root: &Path, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}
