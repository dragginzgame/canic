use super::{
    InitializedRootTopology, RootBaselineMetadata, RootBaselineSpec, progress, progress_elapsed,
};
use crate::pic::{CanicPicExt, startup::try_start_pocket_ic};
use ic_platform::principal::ExternalPrincipal as Principal;

use canic::{
    Error,
    dto::{canister::CanisterInfo, page::PageRequest},
    ids::CanisterRole,
    protocol,
};

use ic_testkit::pic::{PocketIc, PocketIcBuilder, PocketIcStartupError};
use std::{collections::HashMap, time::Instant};

use canic_contracts::dto::wire::projection::fixture_root_topology::RootStatusRequest;

use canic_contracts::dto::wire::projection::fixture_root_topology::RootStatusResponse;

/// Install the configured managed Fleet and fetch its ready Root children.
///
/// # Panics
///
/// Panics if PocketIC cannot be started after the configured retry attempts, if
/// root install/bootstrap fails, if release staging or bootstrap resume fails,
/// or if required root child queries fail.
#[must_use]
pub fn setup_root_topology(
    spec: &RootBaselineSpec<'_>,
    artifacts: &ic_build::artifacts::ArtifactCacheRecord,
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

        progress(spec, "installing configured managed Fleet");
        let install_started_at = Instant::now();
        let authority =
            crate::pic::fleet_registry::install_configured_root(&pic, spec, artifacts, root_wasm);
        let root_id = authority.root;
        progress_elapsed(spec, "managed Fleet activated", install_started_at);

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
                coordinator_id: authority.coordinator,
                pool_assets: authority.pool_assets,
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
fn try_start_root_pic() -> Result<PocketIc, PocketIcStartupError> {
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
        let page: Result<RootStatusResponse, Error> =
            (|| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
                let request_bytes = ic_platform::candid::contract::encode_args((
                    RootStatusRequest::Children(PageRequest {
                        limit: PAGE_LIMIT,
                        offset,
                    }),
                ))?;
                let reply = ic_testkit::pic::try_pocket_ic_operation(|| {
                    (pic).query_call(
                        root_id,
                        ic_platform::principal::ExternalPrincipal::anonymous(),
                        protocol::CANIC_PUBLIC_STATUS,
                        request_bytes,
                    )
                })
                .map_err(ic_testkit::pic::PocketIcCallFailure::Operation)?
                .map_err(ic_testkit::pic::PocketIcCallFailure::Rejected)?;
                ic_platform::candid::decode_args(&reply, canic::testing::CANDID_REPLY_LIMITS)
                    .map_err(Into::into)
            })()
            .map(|(reply,)| reply)
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
    let overview: Result<RootStatusResponse, canic::Error> =
        (|| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
            let request_bytes =
                ic_platform::candid::contract::encode_args((RootStatusRequest::StoreOverview,))?;
            let reply = ic_testkit::pic::try_pocket_ic_operation(|| {
                (pic).query_call(
                    root_id,
                    ic_platform::principal::ExternalPrincipal::anonymous(),
                    canic::protocol::CANIC_ROOT_STATUS,
                    request_bytes,
                )
            })
            .map_err(ic_testkit::pic::PocketIcCallFailure::Operation)?
            .map_err(ic_testkit::pic::PocketIcCallFailure::Rejected)?;
            ic_platform::candid::decode_args(&reply, canic::testing::CANDID_REPLY_LIMITS)
                .map_err(Into::into)
        })()
        .map(|(reply,)| reply)
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
