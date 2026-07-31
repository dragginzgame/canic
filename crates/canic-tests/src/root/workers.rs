// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use std::time::Duration;

use candid::Principal;
use canic::{
    Error,
    dto::{
        canister::CanisterInfo,
        page::{Page, PageRequest},
    },
    protocol,
};
use canic_testing_internal::pic::CanicPicExt;
use ic_testkit::pic::Pic;

const TC: u128 = 1_000_000_000_000;
const DEFAULT_FUNDING_COOLDOWN_SECS: u64 = 60;

/// Create a worker canister via the given hub canister.
pub fn create_worker(pic: &Pic, hub_pid: Principal) -> Result<Principal, Error> {
    let worker_pid: Result<Principal, Error> =
        pic.update_call_or_panic(hub_pid, "create_worker", ());
    let worker_pid = worker_pid?;
    wait_for_worker_sync(pic, hub_pid, worker_pid);
    Ok(worker_pid)
}

/// Move a configured worker beyond autonomous funding before an explicit
/// child-to-parent funding call is measured.
pub fn prepare_worker_for_explicit_parent_funding(pic: &Pic, worker_pid: Principal) {
    pic.add_cycles(worker_pid, 20 * TC);
    pic.advance_time(Duration::from_secs(DEFAULT_FUNDING_COOLDOWN_SECS + 1));
    pic.tick();
}

/// Wait until the parent's local child view includes the newly created worker.
fn wait_for_worker_sync(pic: &Pic, hub_pid: Principal, worker_pid: Principal) {
    pic.wait_for_ready(worker_pid, 50, "scale replica bootstrap");

    for _ in 0..50 {
        pic.tick();

        let children: Result<Page<CanisterInfo>, Error> = pic.query_call_or_panic(
            hub_pid,
            protocol::CANIC_CANISTER_CHILDREN,
            (PageRequest {
                limit: 100,
                offset: 0,
            },),
        );

        if children
            .expect("query child list application")
            .entries
            .into_iter()
            .any(|entry| entry.pid == worker_pid)
        {
            return;
        }
    }

    pic.dump_canister_debug(hub_pid, "scale replica sync");
    panic!("parent {hub_pid} did not observe worker {worker_pid} in time");
}
