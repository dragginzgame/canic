use canic_core::ids::ReleaseBuildId;
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
            "usage: cargo run -p canic-host --example build_artifact -- <canister-name> <debug|fast|release> <workspace-root> <icp-root> <config-path> [--refresh-canonical-did] [--release-build-id <id>]"
                .into(),
        );
    };
    let mut refresh_canonical_infrastructure_did = false;
    let mut release_build_id = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--refresh-canonical-did"
                if matches!(canister_name.as_str(), "fleet_coordinator" | "wasm_store") =>
            {
                refresh_canonical_infrastructure_did = true;
            }
            "--refresh-canonical-did" => {
                return Err(
                    "--refresh-canonical-did requires fleet_coordinator or wasm_store".into(),
                );
            }
            "--release-build-id" => {
                if release_build_id.is_some() {
                    return Err("--release-build-id may be supplied only once".into());
                }
                let value = args
                    .next()
                    .ok_or("--release-build-id requires a canonical release-build ID")?;
                release_build_id = Some(value.parse::<ReleaseBuildId>()?);
            }
            _ => return Err("unknown build_artifact argument".into()),
        }
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
        refresh_canonical_infrastructure_did,
        release_build_id,
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
