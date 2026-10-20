// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use canic::{Error, dto::page::PageRequest, protocol};
use canic_contracts::dto::wire::projection::current_inventory::{
    ChildrenStatusRequest as CanisterStatusRequest,
    ChildrenStatusResponse as CanisterStatusResponse,
};
use canic_testing_internal::pic::{CanicPicExt, report_canister_diagnostics};
use ic_platform::principal::ExternalPrincipal as Principal;
use ic_testkit::pic::PocketIc;
use std::time::Duration;

const TC: u128 = 1_000_000_000_000;
const DEFAULT_FUNDING_COOLDOWN_SECS: u64 = 60;

/// Create a worker canister via the given hub canister.
///
/// # Panics
/// Panics on a Candid harness failure or if the worker fails readiness checks.
pub fn create_worker(
    pic: &PocketIc,
    root_pid: Principal,
    hub_pid: Principal,
) -> Result<Principal, Error> {
    let worker_pid: Result<Principal, Error> = ic_testkit::pic::update_candid_args(
        pic,
        hub_pid,
        ic_platform::principal::ExternalPrincipal::anonymous(),
        "create_worker",
        (),
        canic::testing::CANDID_REPLY_LIMITS,
    )
    .map(|(reply,)| reply)
    .expect("update Candid harness call");
    let worker_pid = worker_pid?;
    wait_for_worker_sync(pic, root_pid, hub_pid, worker_pid);
    Ok(worker_pid)
}

/// Move a configured worker beyond autonomous funding before an explicit
/// child-to-parent funding call is measured.
pub fn prepare_worker_for_explicit_parent_funding(pic: &PocketIc, worker_pid: Principal) {
    pic.add_cycles(worker_pid, 20 * TC);
    pic.advance_time(Duration::from_secs(DEFAULT_FUNDING_COOLDOWN_SECS + 1));
    pic.tick();
}

/// Wait until the parent's local child view includes the newly created worker.
fn wait_for_worker_sync(
    pic: &PocketIc,
    root_pid: Principal,
    hub_pid: Principal,
    worker_pid: Principal,
) {
    pic.wait_for_ready(worker_pid, hub_pid, 50, "scale replica bootstrap");

    for _ in 0..50 {
        pic.tick();

        let children: Result<CanisterStatusResponse, Error> = ic_testkit::pic::query_candid_args(
            pic,
            hub_pid,
            ic_platform::principal::ExternalPrincipal::anonymous(),
            protocol::CANIC_PUBLIC_STATUS,
            (CanisterStatusRequest::Children(PageRequest {
                limit: 100,
                offset: 0,
            }),),
            canic::testing::CANDID_REPLY_LIMITS,
        )
        .map(|(reply,)| reply)
        .expect("query Candid harness call");

        let CanisterStatusResponse::Children(children) =
            children.expect("query child list application");
        if children
            .entries
            .into_iter()
            .any(|entry| entry.pid == worker_pid)
        {
            return;
        }
    }

    report_canister_diagnostics(pic, hub_pid, root_pid, "scale replica sync");
    panic!("parent {hub_pid} did not observe worker {worker_pid} in time");
}
