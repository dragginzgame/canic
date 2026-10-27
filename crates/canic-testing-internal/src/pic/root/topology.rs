use super::{
    InitializedRootTopology, RootBaselineMetadata, RootBaselineSpec, progress, progress_elapsed,
};
use crate::pic::{CanicPicExt, startup::try_start_pocket_ic};
use ic_platform::principal::ExternalPrincipal as Principal;

use canic::ids::CanisterRole;

use std::{collections::HashMap, time::Instant};
use {
    ic_platform::pocket_ic::{PocketIc, PocketIcBuilder},
    ic_testkit::pic::PocketIcStartupError,
};

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

        progress(spec, "retaining provisioned Component inventory");
        let directory_started_at = Instant::now();
        let components = authority
            .components
            .iter()
            .map(|entry| {
                (
                    entry.role.clone(),
                    entry
                        .installation
                        .as_ref()
                        .expect("committed Component installation")
                        .binding
                        .canister_id,
                )
            })
            .collect::<Vec<_>>();
        let component_canisters = components.iter().cloned().collect();
        progress_elapsed(
            spec,
            "retained provisioned Component inventory",
            directory_started_at,
        );

        progress(spec, "waiting for child canisters ready");
        let child_wait_started_at = Instant::now();
        wait_for_children_ready(spec, &pic, root_id, &component_canisters);
        progress_elapsed(spec, "child canisters ready", child_wait_started_at);

        progress(spec, "fetching root child snapshots");
        let snapshot_started_at = Instant::now();
        let managed_store_pids = fetch_managed_store_pids(&pic, root_id);
        let snapshot_pids = components
            .iter()
            .map(|(_, canister_id)| *canister_id)
            .chain(managed_store_pids.iter().copied())
            .collect::<Vec<_>>();
        wait_for_snapshot_pids_ready(spec, &pic, root_id, &snapshot_pids);
        progress_elapsed(spec, "root child snapshots ready", snapshot_started_at);

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

// Wait until root reports `canic_readiness`.
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
    // Store exposes bootstrap through its role overview, not the protected
    // application readiness endpoint. Components were checked separately above.
    for _ in 0..spec.bootstrap_tick_limit {
        pic.tick();
        let mut all_ready = true;
        for canister_id in snapshot_pids {
            let response: Result<canic::dto::role::RoleOverviewResponse, canic::Error> =
                ic_testkit::pic::query_wire(
                    pic,
                    *canister_id,
                    root_id,
                    ic_testkit::pic::WireCallSpec {
                        method: canic::protocol::CANIC_PUBLIC_OVERVIEW,
                        method_id: canic::protocol::public_overview::METHOD_ID,
                        request_limits: canic::protocol::public_overview::REQUEST_LIMITS,
                        response_limits: canic::protocol::public_overview::RESPONSE_LIMITS,
                    },
                    &(),
                )
                .expect("snapshot role overview transport");
            all_ready &= response
                .expect("snapshot role overview application result")
                .bootstrap
                .ready;
        }
        if all_ready {
            return;
        }
    }
    panic!("snapshot roles did not finish bootstrap within the bounded wait");
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

// Fetch the currently tracked managed wasm_store canister ids from root-owned state.
fn fetch_managed_store_pids(pic: &PocketIc, root_id: Principal) -> Vec<Principal> {
    let overview: Result<canic::dto::root_store_reporting::Overview, canic::Error> =
        ic_testkit::pic::query_wire(
            pic,
            root_id,
            Principal::anonymous(),
            ic_testkit::pic::WireCallSpec {
                method: canic::protocol::root_store_reporting::overview::METHOD,
                method_id: canic::protocol::root_store_reporting::overview::METHOD_ID,
                request_limits: canic::protocol::root_store_reporting::REQUEST_LIMITS,
                response_limits: canic::protocol::root_store_reporting::RESPONSE_LIMITS,
            },
            &(),
        )
        .expect("native Root Store overview transport");
    overview
        .expect("Root Store overview application")
        .stores
        .into_iter()
        .map(|store| store.pid.into())
        .collect()
}
