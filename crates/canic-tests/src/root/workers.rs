// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use canic::{
    Error,
    cdk::types::Principal,
    dto::{
        canister::CanisterInfo,
        page::{Page, PageRequest},
        topology::SubnetRegistryEntry,
        topology::SubnetRegistryResponse,
    },
    protocol,
};
use canic_testing_internal::canister::SCALE_REPLICA;
use canic_testing_internal::pic::CanicPicExt;
use ic_testkit::pic::Pic;

/// Create a worker canister via the given hub canister.
pub fn create_worker(pic: &Pic, hub_pid: Principal) -> Result<Principal, Error> {
    let worker_pid: Result<Principal, Error> =
        pic.update_call_or_panic(hub_pid, "create_worker", ());
    let worker_pid = worker_pid?;
    wait_for_worker_sync(pic, hub_pid, worker_pid);
    Ok(worker_pid)
}

/// Count worker canisters registered under a given parent.
#[must_use]
pub fn count_workers(pic: &Pic, root_id: Principal, parent_pid: Principal) -> usize {
    let registry: Result<SubnetRegistryResponse, Error> =
        pic.query_call_or_panic(root_id, protocol::CANIC_SUBNET_REGISTRY, ());
    let SubnetRegistryResponse(registry): SubnetRegistryResponse =
        registry.expect("query subnet registry application");

    registry
        .iter()
        .filter(|entry: &&SubnetRegistryEntry| {
            entry.role == SCALE_REPLICA && entry.record.parent_pid == Some(parent_pid)
        })
        .count()
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
