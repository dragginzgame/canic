use super::{
    InitializedRootTopology, RootBaselineMetadata, RootBaselineSpec, progress, progress_elapsed,
};
use canic::{cdk::types::Principal, dto::topology::SubnetRegistryResponse, ids::CanisterRole};
use canic_control_plane::dto::template::WasmStoreOverviewResponse;
use canic_testkit::pic::{Pic, PicBuilder, PicStartError};
use std::{
    collections::{BTreeSet, HashMap},
    time::Instant,
};

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
        super::stage_managed_release_set(spec, &pic, root_id);
        progress_elapsed(spec, "staged managed release set", stage_started_at);

        progress(spec, "resuming root bootstrap");
        let resume_started_at = Instant::now();
        super::artifacts::resume_root_bootstrap(&pic, root_id);
        progress_elapsed(spec, "resumed root bootstrap", resume_started_at);

        progress(spec, "waiting for root bootstrap");
        let root_wait_started_at = Instant::now();
        wait_for_bootstrap(spec, &pic, root_id);
        progress_elapsed(spec, "root bootstrap ready", root_wait_started_at);

        progress(spec, "fetching subnet index");
        let index_started_at = Instant::now();
        let subnet_index = fetch_subnet_index(&pic, root_id);
        progress_elapsed(spec, "fetched subnet index", index_started_at);

        progress(spec, "waiting for child canisters ready");
        let child_wait_started_at = Instant::now();
        wait_for_children_ready(spec, &pic, &subnet_index);
        progress_elapsed(spec, "child canisters ready", child_wait_started_at);

        progress(spec, "fetching registered child snapshots");
        let snapshot_started_at = Instant::now();
        let snapshot_pids = fetch_snapshot_pids(&pic, root_id);
        wait_for_snapshot_pids_ready(spec, &pic, &snapshot_pids);
        progress_elapsed(
            spec,
            "registered child snapshots ready",
            snapshot_started_at,
        );

        let managed_store_pids = fetch_managed_store_pids(&pic, root_id);

        return InitializedRootTopology {
            pic,
            metadata: RootBaselineMetadata {
                root_id,
                subnet_index,
                snapshot_pids,
                managed_store_pids,
            },
        };
    }

    unreachable!("setup_root must return or panic")
}

// Wait until root reports `canic_ready`.
pub fn wait_for_bootstrap(spec: &RootBaselineSpec<'_>, pic: &Pic, root_id: Principal) {
    pic.wait_for_ready(root_id, spec.bootstrap_tick_limit, "root bootstrap");
}

// Wait until every child canister reports `canic_ready`.
pub fn wait_for_children_ready(
    spec: &RootBaselineSpec<'_>,
    pic: &Pic,
    subnet_index: &HashMap<CanisterRole, Principal>,
) {
    pic.wait_for_all_ready(
        subnet_index
            .iter()
            .filter(|(role, _)| !role.is_root())
            .map(|(_, pid)| *pid),
        spec.bootstrap_tick_limit,
        "root children bootstrap",
    );
}

// Wait until every registered child PID that will be snapshotted is ready.
pub fn wait_for_snapshot_pids_ready(
    spec: &RootBaselineSpec<'_>,
    pic: &Pic,
    snapshot_pids: &[Principal],
) {
    pic.wait_for_all_ready(
        snapshot_pids.iter().copied(),
        spec.bootstrap_tick_limit,
        "root registered child bootstrap",
    );
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

// Fetch the authoritative subnet registry from root and project it into the
// role → principal map used by the root harness metadata.
fn fetch_subnet_index(pic: &Pic, root_id: Principal) -> HashMap<CanisterRole, Principal> {
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

// Fetch every non-root PID in root's registry so cached baselines include replicas.
fn fetch_snapshot_pids(pic: &Pic, root_id: Principal) -> Vec<Principal> {
    let registry: Result<SubnetRegistryResponse, canic::Error> = pic
        .query_call(root_id, canic::protocol::CANIC_SUBNET_REGISTRY, ())
        .expect("query subnet registry transport");

    registry
        .expect("query subnet registry application")
        .0
        .into_iter()
        .filter(|entry| !entry.role.is_root())
        .map(|entry| entry.pid)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

// Fetch the currently tracked managed wasm_store canister ids from root-owned state.
fn fetch_managed_store_pids(pic: &Pic, root_id: Principal) -> Vec<Principal> {
    let overview: Result<WasmStoreOverviewResponse, canic::Error> = pic
        .query_call(root_id, canic::protocol::CANIC_WASM_STORE_OVERVIEW, ())
        .expect("query wasm_store overview transport");

    overview
        .expect("query wasm_store overview application")
        .stores
        .into_iter()
        .map(|store| store.pid)
        .collect()
}
