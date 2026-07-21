use std::time::Duration;

use canic::{
    Error,
    cdk::types::Principal,
    dto::{
        placement::scaling::ScalingRegistryResponse,
        runtime::{
            CanicRuntimeStatus, CanicTimerStatus, TimerProcessCondition, TimerRegistrationStatus,
        },
    },
    protocol,
};
use canic_testing_internal::canister;
use canic_tests::root::{
    RootSetupProfile,
    harness::{RootSetup, setup_root},
    workers::{count_workers, create_worker},
};

#[test]
fn scale_hub_bootstraps_initial_worker_then_manual_create_reaches_min() {
    let setup = setup_root(RootSetupProfile::Scaling);

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

    let registry: Result<Result<ScalingRegistryResponse, Error>, _> =
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
    let after = count_workers(&setup.pic, setup.root_id, scale_hub_pid);
    let acknowledgement = wait_for_placement_acknowledgement(&setup, scale_hub_pid);
    assert_eq!(
        acknowledgement.registration,
        TimerRegistrationStatus::Unregistered
    );
    assert_eq!(acknowledgement.condition, TimerProcessCondition::Idle);
    assert!(
        acknowledgement.executions_since_runtime_start >= 1,
        "placement creation should drain its exact root receipt"
    );
    drop(setup);

    assert_eq!(after, before + 1);
}

fn wait_for_placement_acknowledgement(
    setup: &RootSetup,
    scale_hub_pid: Principal,
) -> CanicTimerStatus {
    for _ in 0..50 {
        let runtime_status: Result<Result<CanicRuntimeStatus, Error>, _> = setup.pic.query_call_as(
            scale_hub_pid,
            setup.root_id,
            protocol::CANIC_RUNTIME_STATUS,
            (),
        );
        let runtime_status = runtime_status
            .expect("scale_hub runtime status transport failed")
            .expect("scale_hub runtime status application failed");
        let acknowledgement = runtime_status
            .timers
            .into_iter()
            .find(|timer| timer.subsystem == "placement" && timer.name == "receipt_ack")
            .expect("scale_hub should expose placement acknowledgement ownership");
        if acknowledgement.registration == TimerRegistrationStatus::Unregistered
            && acknowledgement.condition == TimerProcessCondition::Idle
            && acknowledgement.executions_since_runtime_start >= 1
        {
            return acknowledgement;
        }
        setup.pic.advance_time(Duration::from_secs(1));
        setup.pic.tick();
    }

    panic!("placement acknowledgement did not drain within 50 ticks");
}

fn is_threshold_key_unavailable(err: &canic::Error) -> bool {
    err.message.contains("Requested unknown threshold key")
        || err.message.contains("existing keys: []")
}
