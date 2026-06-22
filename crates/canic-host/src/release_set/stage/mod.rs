mod artifact;
mod call;
mod candid;
mod entry;
mod progress;

use super::{RootReleaseSetManifest, root_time_secs};
use call::icp_call_on_network;
use entry::stage_release_entry;
use progress::StageProgress;

pub(super) use artifact::build_release_set_entry;

#[cfg(test)]
pub(super) use artifact::read_release_artifact;

// Stage one emitted release-set manifest into root and resume bootstrap-ready state.
pub fn stage_root_release_set(
    icp_root: &std::path::Path,
    network: &str,
    root_canister: &str,
    manifest: &RootReleaseSetManifest,
) -> Result<(), Box<dyn std::error::Error>> {
    let now_secs = root_time_secs()?;
    println!("Stage release set:");
    let mut progress = StageProgress::new();
    progress.print_header();

    for entry in &manifest.entries {
        stage_release_entry(
            icp_root,
            network,
            root_canister,
            &manifest.release_version,
            entry,
            now_secs,
            &mut progress,
        )?;
    }

    println!();
    Ok(())
}

// Trigger root bootstrap resume after the ordinary release set is fully staged.
pub fn resume_root_bootstrap(
    network: &str,
    root_canister: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = icp_call_on_network(
        network,
        root_canister,
        canic_core::protocol::CANIC_WASM_STORE_BOOTSTRAP_RESUME_ROOT_ADMIN,
        None,
        None,
    )?;
    Ok(())
}

// Run one query-only `icp canister call` and return stdout, preserving stderr on failure.
pub fn icp_query_on_network(
    network: &str,
    canister: &str,
    method: &str,
    argument: Option<&str>,
    output: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    call::icp_query_on_network(network, canister, method, argument, output)
}
