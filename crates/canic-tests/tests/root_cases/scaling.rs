use crate::root::{
    RootSetupProfile,
    harness::setup_cached_root,
    workers::{count_workers, create_worker},
};
use canic_internal::canister;

#[test]
fn worker_topology_cascades_through_parent() {
    let setup = setup_cached_root(RootSetupProfile::Scaling);

    let scale_hub_pid = setup
        .subnet_directory
        .get(&canister::SCALE_HUB)
        .copied()
        .expect("scale_hub must exist in subnet directory");

    let before = count_workers(&setup.pic, setup.root_id, scale_hub_pid);

    match create_worker(&setup.pic, scale_hub_pid) {
        Ok(_) => {}
        Err(err) if is_threshold_key_unavailable(&err) => {
            eprintln!(
                "skipping worker_topology_cascades_through_parent: threshold key unavailable: {err}"
            );
            return;
        }
        Err(err) => panic!("create_worker application failed: {err:?}"),
    }
    setup.pic.tick_n(10);

    let after = count_workers(&setup.pic, setup.root_id, scale_hub_pid);

    assert_eq!(after, before + 1);
}

fn is_threshold_key_unavailable(err: &canic::Error) -> bool {
    err.message.contains("Requested unknown threshold key")
        || err.message.contains("existing keys: []")
}
