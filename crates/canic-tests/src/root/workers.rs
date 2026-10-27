// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use canic::{Error, dto::page::PageRequest, protocol};
use canic_contracts::dto::{canister::CanisterInfo, page::Page};
use canic_testing_internal::pic::{CanicPicExt, report_canister_diagnostics};
use ic_platform::pocket_ic::PocketIc;
use ic_platform::principal::ExternalPrincipal as Principal;
use std::time::Duration;

const TC: u128 = 1_000_000_000_000;
const DEFAULT_FUNDING_COOLDOWN_SECS: u64 = 60;

/// Create a worker canister via the given hub canister.
///
/// # Panics
/// Panics on a Wire harness failure or if the worker fails readiness checks.
pub fn create_worker(
    pic: &PocketIc,
    root_pid: Principal,
    hub_pid: Principal,
) -> Result<Principal, Error> {
    let worker_pid: Result<ic_platform::Principal, Error> = ic_testkit::pic::update_wire(
        pic,
        hub_pid,
        Principal::anonymous(),
        canic_testing_internal::pic::application_probe::CREATE_WORKER,
        &(),
    )
    .expect("create worker Wire transport");
    let worker_pid: Principal = worker_pid?.into();
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

        let children: Result<Page<CanisterInfo>, Error> = ic_testkit::pic::query_wire(
            pic,
            hub_pid,
            Principal::anonymous(),
            ic_testkit::pic::WireCallSpec {
                method: protocol::CANIC_PUBLIC_CHILDREN,
                method_id: protocol::public_children::METHOD_ID,
                request_limits: protocol::public_children::REQUEST_LIMITS,
                response_limits: protocol::public_children::RESPONSE_LIMITS,
            },
            &PageRequest {
                limit: 100,
                offset: 0,
            },
        )
        .expect("query child inventory Wire frame");
        let children = children.expect("query child inventory application");
        if children
            .entries
            .into_iter()
            .any(|entry| entry.pid == worker_pid.into())
        {
            return;
        }
    }

    report_canister_diagnostics(pic, hub_pid, root_pid, "scale replica sync");
    panic!("parent {hub_pid} did not observe worker {worker_pid} in time");
}
