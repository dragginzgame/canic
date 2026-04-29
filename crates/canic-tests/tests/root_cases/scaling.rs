use crate::root::{
    RootSetupProfile,
    harness::setup_cached_root,
    workers::{count_workers, create_worker},
};
use canic::{Error, dto::placement::scaling::ScalingRegistryResponse};
use canic_reference_support::canister;

#[test]
fn scale_hub_bootstraps_initial_worker_then_manual_create_reaches_min() {
    let setup = setup_cached_root(RootSetupProfile::Scaling);

    let scale_hub_pid = setup
        .subnet_index
        .get(&canister::SCALE_HUB)
        .copied()
        .expect("scale_hub must exist in subnet index");

    let before = count_workers(&setup.pic, setup.root_id, scale_hub_pid);
    assert_eq!(
        before, 1,
        "scale_hub should bootstrap policy.initial_workers before manual create_worker",
    );

    let registry: Result<Result<ScalingRegistryResponse, Error>, Error> =
        setup
            .pic
            .query_call_as(scale_hub_pid, setup.root_id, "canic_scaling_registry", ());
    let registry = registry
        .expect("scaling registry query transport failed")
        .expect("scaling registry query application failed");
    assert_eq!(
        registry.0.len(),
        before,
        "scale_hub local scaling registry should track startup workers",
    );

    match create_worker(&setup.pic, scale_hub_pid) {
        Ok(_) => {}
        Err(err) if is_threshold_key_unavailable(&err) => {
            eprintln!(
                "skipping scale_hub_bootstraps_initial_worker_then_manual_create_reaches_min: threshold key unavailable: {err}"
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
