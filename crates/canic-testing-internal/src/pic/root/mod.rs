use canic::{cdk::types::Principal, dto::topology::SubnetRegistryResponse, ids::CanisterRole};
use canic_testkit::{
    artifacts::WasmBuildProfile,
    pic::{CachedPicBaseline, Pic, PicBuilder, PicStartError},
};
use std::{collections::HashMap, io::Write, path::PathBuf, time::Instant};

mod artifacts;

use artifacts::stage_managed_release_set;
pub use artifacts::{ensure_root_release_artifacts_built, load_root_wasm};

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
    pub subnet_directory: HashMap<CanisterRole, Principal>,
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

/// Build one fresh root topology and capture immutable controller snapshots for cache reuse.
#[must_use]
pub fn build_root_cached_baseline(
    spec: &RootBaselineSpec<'_>,
    root_wasm: Vec<u8>,
) -> CachedPicBaseline<RootBaselineMetadata> {
    let initialized = setup_root_topology(spec, root_wasm);
    let controller_ids = std::iter::once(initialized.metadata.root_id)
        .chain(initialized.metadata.subnet_directory.values().copied())
        .collect::<Vec<_>>();

    progress(spec, "capturing cached root snapshots");
    let started_at = Instant::now();
    let baseline = CachedPicBaseline::capture(
        initialized.pic,
        initialized.metadata.root_id,
        controller_ids,
        initialized.metadata,
    )
    .expect("cached root snapshots must be available");
    progress_elapsed(spec, "captured cached root snapshots", started_at);
    baseline
}

/// Restore one cached root topology and wait until root plus children are ready again.
pub fn restore_root_cached_baseline(
    spec: &RootBaselineSpec<'_>,
    baseline: &CachedPicBaseline<RootBaselineMetadata>,
) {
    progress(spec, "restoring cached root snapshots");
    let restore_started_at = Instant::now();
    baseline.restore(baseline.metadata().root_id);
    progress_elapsed(spec, "restored cached root snapshots", restore_started_at);

    progress(spec, "waiting for restored root bootstrap");
    let root_wait_started_at = Instant::now();
    wait_for_bootstrap(spec, baseline.pic(), baseline.metadata().root_id);
    progress_elapsed(spec, "restored root bootstrap ready", root_wait_started_at);

    progress(spec, "waiting for restored child canisters ready");
    let child_wait_started_at = Instant::now();
    wait_for_children_ready(spec, baseline.pic(), &baseline.metadata().subnet_directory);
    progress_elapsed(
        spec,
        "restored child canisters ready",
        child_wait_started_at,
    );
}

/// Install root, stage one ordinary release profile, resume bootstrap, and fetch the subnet map.
#[must_use]
pub fn setup_root_topology(
    spec: &RootBaselineSpec<'_>,
    root_wasm: Vec<u8>,
) -> InitializedRootTopology {
    for attempt in 1..=spec.root_setup_max_attempts {
        progress(
            spec,
            &format!(
                "initialize root setup attempt {attempt}/{}",
                spec.root_setup_max_attempts
            ),
        );
        let pic_started_at = Instant::now();
        let pic = match try_start_root_pic(spec) {
            Ok(pic) => {
                progress_elapsed(spec, "PocketIC instance ready", pic_started_at);
                pic
            }
            Err(err) if should_retry_root_pic_start(&err, spec, attempt) => {
                eprintln!(
                    "setup_root startup attempt {attempt}/{} failed; retrying: {err}",
                    spec.root_setup_max_attempts
                );
                continue;
            }
            Err(err) => {
                panic!(
                    "failed to start PocketIC instance for root baseline on attempt {attempt}/{}: {err}",
                    spec.root_setup_max_attempts
                );
            }
        };

        progress(spec, "installing root canister");
        let root_install_started_at = Instant::now();
        let root_id = pic
            .create_and_install_root_canister(root_wasm)
            .expect("install root canister");
        progress_elapsed(spec, "root canister installed", root_install_started_at);

        progress(spec, "staging managed release set");
        let stage_started_at = Instant::now();
        stage_managed_release_set(spec, &pic, root_id);
        progress_elapsed(spec, "staged managed release set", stage_started_at);

        progress(spec, "resuming root bootstrap");
        let resume_started_at = Instant::now();
        artifacts::resume_root_bootstrap(&pic, root_id);
        progress_elapsed(spec, "resumed root bootstrap", resume_started_at);

        progress(spec, "waiting for root bootstrap");
        let root_wait_started_at = Instant::now();
        wait_for_bootstrap(spec, &pic, root_id);
        progress_elapsed(spec, "root bootstrap ready", root_wait_started_at);

        progress(spec, "fetching subnet directory");
        let directory_started_at = Instant::now();
        let subnet_directory = fetch_subnet_directory(&pic, root_id);
        progress_elapsed(spec, "fetched subnet directory", directory_started_at);

        progress(spec, "waiting for child canisters ready");
        let child_wait_started_at = Instant::now();
        wait_for_children_ready(spec, &pic, &subnet_directory);
        progress_elapsed(spec, "child canisters ready", child_wait_started_at);

        return InitializedRootTopology {
            pic,
            metadata: RootBaselineMetadata {
                root_id,
                subnet_directory,
            },
        };
    }

    unreachable!("setup_root must return or panic")
}

// Start the PocketIC instance for one root baseline and retry only on the
// typed startup failures we explicitly treat as transient.
fn try_start_root_pic(spec: &RootBaselineSpec<'_>) -> Result<Pic, PicStartError> {
    progress(spec, "starting PocketIC instance");

    PicBuilder::new()
        .with_ii_subnet()
        .with_application_subnet()
        .try_build()
}

const fn should_retry_root_pic_start(
    err: &PicStartError,
    spec: &RootBaselineSpec<'_>,
    attempt: usize,
) -> bool {
    attempt < spec.root_setup_max_attempts
        && matches!(
            err,
            PicStartError::ServerStartFailed { .. } | PicStartError::StartupTimedOut { .. }
        )
}

///
/// InitializedRootTopology
///

pub struct InitializedRootTopology {
    pub pic: Pic,
    pub metadata: RootBaselineMetadata,
}

// Wait until root reports `canic_ready`.
fn wait_for_bootstrap(spec: &RootBaselineSpec<'_>, pic: &Pic, root_id: Principal) {
    pic.wait_for_ready(root_id, spec.bootstrap_tick_limit, "root bootstrap");
}

// Wait until every child canister reports `canic_ready`.
fn wait_for_children_ready(
    spec: &RootBaselineSpec<'_>,
    pic: &Pic,
    subnet_directory: &HashMap<CanisterRole, Principal>,
) {
    pic.wait_for_all_ready(
        subnet_directory
            .iter()
            .filter(|(role, _)| !role.is_root())
            .map(|(_, pid)| *pid),
        spec.bootstrap_tick_limit,
        "root children bootstrap",
    );
}

// Fetch the authoritative subnet registry from root and project it into the
// role → principal map used by the root harness metadata.
fn fetch_subnet_directory(pic: &Pic, root_id: Principal) -> HashMap<CanisterRole, Principal> {
    let registry: Result<SubnetRegistryResponse, canic::Error> = pic
        .query_call(root_id, canic::protocol::CANIC_SUBNET_REGISTRY, ())
        .expect("query subnet registry transport");

    let registry = registry.expect("query subnet registry application");

    registry
        .0
        .into_iter()
        .filter(|entry| !entry.role.is_root() && !entry.role.is_wasm_store())
        .map(|entry| (entry.role, entry.pid))
        .collect()
}
