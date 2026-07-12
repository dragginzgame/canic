//! Module: release_set::stage
//!
//! Responsibility: validate and stage one root release-set manifest.
//! Does not own: artifact construction, root installation, or bootstrap policy.
//! Boundary: rejects invalid manifest authority before issuing ICP mutations.

mod artifact;
mod call;
mod entry;
mod progress;

use super::{RootReleaseSetManifest, root_time_secs, validate_root_release_set_manifest};
use crate::icp::LocalReplicaTarget;

use call::icp_call_on_network;
use entry::stage_release_entry;
use progress::StageProgress;

pub(super) use artifact::build_release_set_entry;
pub use artifact::resolve_release_artifact_path;

#[cfg(test)]
pub(super) use artifact::read_release_artifact;

// Stage one emitted release-set manifest into root and resume bootstrap-ready state.
pub fn stage_root_release_set(
    icp_root: &std::path::Path,
    network: &str,
    local_replica: Option<&LocalReplicaTarget>,
    root_canister: &str,
    manifest: &RootReleaseSetManifest,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_root_release_set_manifest(manifest)?;
    let now_secs = root_time_secs()?;
    println!("Stage release set:");
    let mut progress = StageProgress::new();
    progress.print_header();

    for entry in &manifest.entries {
        stage_release_entry(
            icp_root,
            network,
            local_replica,
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
    icp_root: &std::path::Path,
    network: &str,
    local_replica: Option<&LocalReplicaTarget>,
    root_canister: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = icp_call_on_network(
        icp_root,
        network,
        local_replica,
        root_canister,
        canic_core::protocol::CANIC_WASM_STORE_BOOTSTRAP_RESUME_ROOT_ADMIN,
        None,
        None,
    )?;
    Ok(())
}

// Run one query-only `icp canister call` and return stdout, preserving stderr on failure.
pub fn icp_query_on_network(
    icp_root: &std::path::Path,
    network: &str,
    local_replica: Option<&LocalReplicaTarget>,
    canister: &str,
    method: &str,
    argument: Option<&str>,
    output: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    call::icp_query_on_network(
        icp_root,
        network,
        local_replica,
        canister,
        method,
        argument,
        output,
    )
}
