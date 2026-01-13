use canic::{
    Error,
    cdk::types::Principal,
    core::{
        dto::{topology::SubnetRegistryEntryView, topology::SubnetRegistryView},
        protocol,
    },
};
use canic_internal::canister::SCALE;
use canic_testkit::pic::Pic;

/// Create a worker canister via the given hub canister.
///
/// Panics on transport or application failure.
pub fn create_worker(pic: &Pic, hub_pid: Principal) -> Principal {
    let result: Result<Result<Principal, Error>, Error> =
        pic.update_call(hub_pid, "create_worker", ());

    result
        .expect("create_worker transport failed")
        .expect("create_worker application failed")
}

/// Count worker canisters registered under a given parent.
pub fn count_workers(pic: &Pic, root_id: Principal, parent_pid: Principal) -> usize {
    let SubnetRegistryView(registry): SubnetRegistryView = pic
        .query_call(root_id, protocol::CANIC_SUBNET_REGISTRY, ())
        .expect("query subnet registry");

    registry
        .iter()
        .filter(|entry: &&SubnetRegistryEntryView| {
            entry.role == SCALE && entry.record.parent_pid == Some(parent_pid)
        })
        .count()
}
