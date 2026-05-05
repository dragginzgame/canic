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
use canic_testing_internal::canister::SCALE;
use canic_testkit::pic::Pic;

/// Create a worker canister via the given hub canister.
pub fn create_worker(pic: &Pic, hub_pid: Principal) -> Result<Principal, Error> {
    let result: Result<Result<Principal, Error>, Error> =
        pic.update_call(hub_pid, "create_worker", ());

    let worker_pid = result
        .map_err(|err| Error::internal(format!("create_worker transport failed: {err}")))??;
    wait_for_worker_sync(pic, hub_pid, worker_pid);
    Ok(worker_pid)
}

/// Count worker canisters registered under a given parent.
pub fn count_workers(pic: &Pic, root_id: Principal, parent_pid: Principal) -> usize {
    let registry: Result<SubnetRegistryResponse, Error> = pic
        .query_call(root_id, protocol::CANIC_SUBNET_REGISTRY, ())
        .expect("query subnet registry transport");
    let SubnetRegistryResponse(registry): SubnetRegistryResponse =
        registry.expect("query subnet registry application");

    registry
        .iter()
        .filter(|entry: &&SubnetRegistryEntry| {
            entry.role == SCALE && entry.record.parent_pid == Some(parent_pid)
        })
        .count()
}

/// Wait until the parent's local child view includes the newly created worker.
fn wait_for_worker_sync(pic: &Pic, hub_pid: Principal, worker_pid: Principal) {
    pic.wait_for_ready(worker_pid, 50, "scale worker bootstrap");

    for _ in 0..50 {
        pic.tick();

        let children: Result<Page<CanisterInfo>, Error> = pic
            .query_call(
                hub_pid,
                protocol::CANIC_CANISTER_CHILDREN,
                (PageRequest {
                    limit: 100,
                    offset: 0,
                },),
            )
            .expect("query child list transport");

        if children
            .expect("query child list application")
            .entries
            .into_iter()
            .any(|entry| entry.pid == worker_pid)
        {
            return;
        }
    }

    pic.dump_canister_debug(hub_pid, "scale worker sync");
    panic!("parent {hub_pid} did not observe worker {worker_pid} in time");
}
