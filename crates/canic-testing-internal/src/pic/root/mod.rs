//! Root-specific cached baseline and topology helpers for internal test suites.

use canic::{cdk::types::Principal, ids::CanisterRole};
use canic_testkit::{artifacts::WasmBuildProfile, pic::Pic};
use std::{collections::HashMap, io::Write, path::PathBuf, time::Instant};

mod artifacts;
mod baseline;
mod topology;

use artifacts::stage_managed_release_set;
pub use artifacts::{ensure_root_release_artifacts_built, load_root_wasm};
pub use baseline::{build_root_cached_baseline, restore_root_cached_baseline};
pub use topology::setup_root_topology;

///
/// RootBaselineSpec
///

#[derive(Clone)]
pub struct RootBaselineSpec<'a> {
    pub progress_prefix: &'a str,
    pub workspace_root: PathBuf,
    pub root_wasm_relative: &'a str,
    pub root_wasm_artifact_relative: &'a str,
    pub root_release_artifacts_relative: &'a str,
    pub artifact_watch_paths: &'a [&'a str],
    pub release_roles: &'a [&'a str],
    pub dfx_build_lock_relative: &'a str,
    pub build_network: &'a str,
    pub build_profile: WasmBuildProfile,
    pub build_extra_env: Vec<(String, String)>,
    pub bootstrap_tick_limit: usize,
    pub root_setup_max_attempts: usize,
    pub pocket_ic_wasm_chunk_store_limit_bytes: usize,
    pub root_release_chunk_bytes: usize,
    pub package_version: &'a str,
}

///
/// RootBaselineMetadata
///

pub struct RootBaselineMetadata {
    pub root_id: Principal,
    pub subnet_index: HashMap<CanisterRole, Principal>,
    pub snapshot_pids: Vec<Principal>,
    pub managed_store_pids: Vec<Principal>,
}

// Print one progress line for a root-test setup phase and flush immediately.
fn progress(spec: &RootBaselineSpec<'_>, phase: &str) {
    eprintln!("[{}] {phase}", spec.progress_prefix);
    let _ = std::io::stderr().flush();
}

// Print one completed phase with wall-clock timing.
fn progress_elapsed(spec: &RootBaselineSpec<'_>, phase: &str, started_at: Instant) {
    progress(
        spec,
        &format!("{phase} in {:.2}s", started_at.elapsed().as_secs_f32()),
    );
}

///
/// InitializedRootTopology
///

pub struct InitializedRootTopology {
    pub pic: Pic,
    pub metadata: RootBaselineMetadata,
}
