use super::{
    InitializedRootTopology, RootBaselineMetadata, RootBaselineSpec, progress, progress_elapsed,
};
use candid::{CandidType, Deserialize, Principal};
use canic::{
    Error,
    dto::{
        canister::CanisterInfo,
        page::{Page, PageRequest},
    },
    ids::CanisterRole,
    protocol,
};
use canic_control_plane::dto::template::WasmStoreOverviewResponse;
use ic_testkit::pic::{PocketIc, PocketIcBuilder, prelude::*};
use std::{collections::HashMap, fs, time::Instant};

use crate::pic::{
    CanicPicExt,
    canic::{adopt_sibling_wasm_store, create_and_install_pre_adoption_root},
    startup::{PocketIcHarnessStartupError, try_start_pocket_ic},
};

#[derive(CandidType)]
enum RootStatusRequest {
    Children(PageRequest),
    StoreOverview,
}

#[derive(CandidType, Deserialize)]
enum RootStatusResponse {
    Children(Page<CanisterInfo>),
    StoreOverview(WasmStoreOverviewResponse),
}

/// Install root, stage one ordinary release profile, resume bootstrap, and fetch root children.
///
/// # Panics
///
/// Panics if PocketIC cannot be started after the configured retry attempts, if
/// root install/bootstrap fails, if release staging or bootstrap resume fails,
/// or if required root child queries fail.
#[must_use]
pub fn setup_root_topology(
    spec: &RootBaselineSpec<'_>,
    root_wasm: Vec<u8>,
) -> InitializedRootTopology {
    let wasm_store_wasm = fs::read(
        spec.root_release_artifacts_dir
            .join("wasm_store/wasm_store.wasm.gz"),
    )
    .expect("read sibling Wasm Store artifact");
    for attempt in 1..=spec.root_setup_max_attempts {
        progress(
            spec,
            &format!(
                "initialize root setup attempt {attempt}/{}",
                spec.root_setup_max_attempts
            ),
        );
        let pic_started_at = Instant::now();
        let pic = match try_start_root_pic() {
            Ok(pic) => {
                progress_elapsed(spec, "PocketIC instance ready", pic_started_at);
                pic
            }
            Err(err) if should_retry_root_pic_start(spec, attempt) => {
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
        let prepared = create_and_install_pre_adoption_root(
            &pic,
            root_wasm,
            wasm_store_wasm,
            &spec.build_config_path,
        )
        .expect("install root canister");
        let root_id = prepared.root_id;
        progress_elapsed(spec, "root canister installed", root_install_started_at);

        progress(spec, "staging managed release set");
        let stage_started_at = Instant::now();
        super::stage_managed_release_set(
            spec,
            &pic,
            prepared.wasm_store,
            prepared.installation_controller,
        );
        progress_elapsed(spec, "staged managed release set", stage_started_at);

        adopt_sibling_wasm_store(&pic, root_id, &prepared.root_args);

        progress(spec, "activating managed Fleet");
        let resume_started_at = Instant::now();
        crate::pic::canic::activate_managed_fleet(&pic, root_id);
        progress_elapsed(spec, "activated managed Fleet", resume_started_at);

        progress(spec, "waiting for root bootstrap");
        let root_wait_started_at = Instant::now();
        wait_for_bootstrap(spec, &pic, root_id);
        progress_elapsed(spec, "root bootstrap ready", root_wait_started_at);

        progress(spec, "fetching root child inventory");
        let directory_started_at = Instant::now();
        let root_children = fetch_root_children(&pic, root_id);
        let component_canisters = root_children
            .iter()
            .filter(|entry| !entry.role.is_wasm_store())
            .map(|entry| (entry.role.clone(), entry.pid))
            .collect();
        progress_elapsed(spec, "fetched root child inventory", directory_started_at);

        progress(spec, "waiting for child canisters ready");
        let child_wait_started_at = Instant::now();
        wait_for_children_ready(spec, &pic, root_id, &component_canisters);
        progress_elapsed(spec, "child canisters ready", child_wait_started_at);

        progress(spec, "fetching root child snapshots");
        let snapshot_started_at = Instant::now();
        let snapshot_pids = root_children
            .iter()
            .map(|entry| entry.pid)
            .collect::<Vec<_>>();
        wait_for_snapshot_pids_ready(spec, &pic, root_id, &snapshot_pids);
        progress_elapsed(spec, "root child snapshots ready", snapshot_started_at);

        let managed_store_pids = fetch_managed_store_pids(&pic, root_id);

        return InitializedRootTopology {
            pic,
            metadata: RootBaselineMetadata {
                root_id,
                component_canisters,
                snapshot_pids,
                managed_store_pids,
            },
        };
    }

    unreachable!("setup_root must return or panic")
}

// Wait until root reports `canic_root_status(Readiness)`.
pub(super) fn wait_for_bootstrap(spec: &RootBaselineSpec<'_>, pic: &PocketIc, root_id: Principal) {
    pic.wait_for_root_ready(
        root_id,
        Principal::anonymous(),
        spec.bootstrap_tick_limit,
        "root bootstrap",
    );
}

// Wait until every child canister reports its role-owned readiness status.
pub(super) fn wait_for_children_ready(
    spec: &RootBaselineSpec<'_>,
    pic: &PocketIc,
    root_id: Principal,
    component_canisters: &HashMap<CanisterRole, Principal>,
) {
    pic.wait_for_all_ready(
        component_canisters
            .values()
            .copied()
            .map(|canister_id| (canister_id, root_id)),
        spec.bootstrap_tick_limit,
        "root children bootstrap",
    );
}

// Wait until every registered child PID that will be snapshotted is ready.
pub(super) fn wait_for_snapshot_pids_ready(
    spec: &RootBaselineSpec<'_>,
    pic: &PocketIc,
    root_id: Principal,
    snapshot_pids: &[Principal],
) {
    pic.wait_for_all_ready(
        snapshot_pids
            .iter()
            .copied()
            .map(|canister_id| (canister_id, root_id)),
        spec.bootstrap_tick_limit,
        "root registered child bootstrap",
    );
}

// Start one root baseline through the testkit's typed fallible builder boundary.
fn try_start_root_pic() -> Result<PocketIc, PocketIcHarnessStartupError> {
    try_start_pocket_ic(
        PocketIcBuilder::new()
            .with_ii_subnet()
            .with_application_subnet(),
    )
}

const fn should_retry_root_pic_start(spec: &RootBaselineSpec<'_>, attempt: usize) -> bool {
    attempt < spec.root_setup_max_attempts
}

// Read every direct root child through the maintained bounded child view.
fn fetch_root_children(pic: &PocketIc, root_id: Principal) -> Vec<CanisterInfo> {
    const PAGE_LIMIT: u64 = 1_000;

    let mut entries = Vec::new();
    let mut offset = 0;
    loop {
        let page: Result<RootStatusResponse, Error> = pic
            .query_candid(
                root_id,
                protocol::CANIC_ROOT_STATUS,
                (RootStatusRequest::Children(PageRequest {
                    limit: PAGE_LIMIT,
                    offset,
                }),),
            )
            .expect("query root children transport");
        let page = match page.expect("query root children application") {
            RootStatusResponse::Children(page) => page,
            RootStatusResponse::StoreOverview(_) => panic!("unexpected Root status response"),
        };
        assert!(
            !page.entries.is_empty() || offset >= page.total,
            "root child pagination made no progress at offset {offset}"
        );
        entries.extend(page.entries);
        offset = u64::try_from(entries.len()).expect("root child inventory exceeds u64");
        if offset >= page.total {
            return entries;
        }
    }
}

// Fetch the currently tracked managed wasm_store canister ids from root-owned state.
fn fetch_managed_store_pids(pic: &PocketIc, root_id: Principal) -> Vec<Principal> {
    let overview: Result<RootStatusResponse, canic::Error> = pic
        .query_candid(
            root_id,
            canic::protocol::CANIC_ROOT_STATUS,
            (RootStatusRequest::StoreOverview,),
        )
        .expect("query wasm_store overview transport");

    match overview.expect("query wasm_store overview application") {
        RootStatusResponse::StoreOverview(overview) => overview,
        RootStatusResponse::Children(_) => panic!("unexpected Root status response"),
    }
    .stores
    .into_iter()
    .map(|store| store.pid)
    .collect()
}
