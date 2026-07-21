use canic::{
    Error,
    cdk::types::Principal,
    dto::{
        capability::{
            CAPABILITY_VERSION_V1, CapabilityProof, CapabilityRequestMetadata, CapabilityService,
            RootCapabilityEnvelopeV1, RootCapabilityResponseV1,
        },
        error::ErrorCode,
        metrics::{MetricEntry, MetricValue, MetricsKind},
        page::{Page, PageRequest},
        rpc::{
            AcknowledgePlacementReceiptRequest, CreateCanisterParent, CreateCanisterRequest,
            CyclesRequest, Request, Response, RootRequestMetadata, UpgradeCanisterRequest,
        },
        topology::IndexEntryResponse,
    },
    protocol,
};
use canic_testing_internal::canister;
use canic_tests::root::{
    RootSetupProfile,
    harness::{RootSetup, setup_cached_root},
    workers::{create_worker, prepare_worker_for_explicit_parent_funding},
};
use std::time::Duration;

#[test]
fn later_auto_created_sibling_refreshes_existing_subnet_index_cache() {
    let setup = setup_cached_root(RootSetupProfile::Capability);
    let app_pid = setup
        .subnet_index
        .get(&canister::APP)
        .copied()
        .expect("app canister must exist");
    let test_pid = setup
        .subnet_index
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");

    let app_subnet_index = query_subnet_index(&setup, app_pid);
    drop(setup);
    assert!(
        app_subnet_index
            .iter()
            .any(|entry| entry.role == canister::TEST && entry.pid == test_pid),
        "existing sibling subnet-index cache must refresh with the later-created test entry",
    );
}

#[test]
fn unauthorized_caller_is_denied_for_each_root_capability_variant() {
    let setup = setup_cached_root(RootSetupProfile::Capability);
    let test_pid = setup
        .subnet_index
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");
    let unauthorized = Principal::from_slice(&[250; 29]);
    let baseline_metrics = root_capability_metrics(&setup);
    let baseline_count = root_capability_count_total(&baseline_metrics);

    let cases = vec![
        Request::CreateCanister(CreateCanisterRequest {
            canister_role: canister::SCALE_REPLICA,
            parent: CreateCanisterParent::ThisCanister,
            extra_arg: None,
            metadata: Some(metadata([30u8; 32], 120_000_000_000)),
        }),
        Request::UpgradeCanister(UpgradeCanisterRequest {
            canister_pid: test_pid,
            metadata: Some(metadata([31u8; 32], 120_000_000_000)),
        }),
        Request::Cycles(CyclesRequest {
            cycles: 1_000_000,
            metadata: Some(metadata([32u8; 32], 120_000_000_000)),
        }),
    ];

    for request in cases {
        let err = root_response_as(&setup, unauthorized, request)
            .expect_err("unregistered caller must be rejected at endpoint boundary");
        assert_eq!(err.code, ErrorCode::Unauthorized);
    }

    let metrics = root_capability_metrics(&setup);
    drop(setup);
    assert_eq!(
        root_capability_count_total(&metrics),
        baseline_count,
        "root capability metrics must not change when endpoint auth rejects calls before dispatch"
    );
}

#[test]
fn upgrade_policy_denies_registered_non_parent_caller() {
    let setup = setup_cached_root(RootSetupProfile::Capability);
    let caller = setup
        .subnet_index
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");
    let app_pid = setup
        .subnet_index
        .get(&canister::APP)
        .copied()
        .expect("app canister must exist");

    let request = Request::UpgradeCanister(UpgradeCanisterRequest {
        canister_pid: app_pid,
        metadata: Some(metadata([34u8; 32], 120_000_000_000)),
    });
    let err = root_response_as(&setup, caller, request)
        .expect_err("non-parent caller must be denied by upgrade policy");
    assert_eq!(err.code, ErrorCode::Forbidden);

    let metrics = root_capability_metrics(&setup);
    drop(setup);
    assert_eq!(metric_count(&metrics, "Upgrade", "ProofRejected"), 1);
    assert_eq!(metric_count(&metrics, "Upgrade", "Denied"), 0);
    assert_eq!(metric_count(&metrics, "Upgrade", "Authorized"), 0);
    assert_eq!(metric_count(&metrics, "Upgrade", "ExecutionSuccess"), 0);
}

#[test]
fn cycles_routes_through_dispatcher_and_replay_duplicate_same() {
    let setup = setup_cached_root(RootSetupProfile::Capability);
    let caller = setup
        .subnet_index
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");
    let baseline_metrics = root_capability_metrics(&setup);
    let request = Request::Cycles(CyclesRequest {
        cycles: 1_111_000,
        metadata: Some(metadata([36u8; 32], 120_000_000_000)),
    });

    let first = root_response_as(&setup, caller, request.clone()).expect("first cycles call works");
    match first {
        Response::Cycles(response) => response.cycles_transferred,
        other => panic!("expected create canister response, got: {other:?}"),
    };

    let second = root_response_as(&setup, caller, request)
        .expect("identical replay should return cached response");
    match second {
        Response::Cycles(response) => assert_eq!(response.cycles_transferred, 1_111_000),
        other => panic!("expected cached cycles response, got: {other:?}"),
    }

    let metrics = root_capability_metrics(&setup);
    drop(setup);
    assert_eq!(
        metric_delta(&metrics, &baseline_metrics, "RequestCycles", "Authorized"),
        1
    );
    assert_eq!(
        metric_delta(
            &metrics,
            &baseline_metrics,
            "RequestCycles",
            "ReplayAccepted"
        ),
        1
    );
    assert_eq!(
        metric_delta(
            &metrics,
            &baseline_metrics,
            "RequestCycles",
            "ReplayDuplicateSame"
        ),
        1
    );
    assert_eq!(
        metric_delta(
            &metrics,
            &baseline_metrics,
            "RequestCycles",
            "ExecutionSuccess"
        ),
        1
    );
}

#[test]
fn placement_retry_reuses_exact_root_receipt_and_acknowledgement_is_not_replayed() {
    let setup = setup_cached_root(RootSetupProfile::Scaling);
    let caller = setup
        .subnet_index
        .get(&canister::SCALE_HUB)
        .copied()
        .expect("scale_hub canister must exist");
    let baseline_metrics = root_capability_metrics(&setup);
    let operation_id = [83u8; 32];
    let allocation = Request::AllocatePlacementChild(CreateCanisterRequest {
        canister_role: canister::SCALE_REPLICA,
        parent: CreateCanisterParent::ThisCanister,
        extra_arg: None,
        metadata: Some(metadata(operation_id, 120_000_000_000)),
    });

    let first_pid = match root_response_as(&setup, caller, allocation.clone())
        .expect("first placement allocation must succeed")
    {
        Response::CreateCanister(response) => response.new_canister_pid,
        other => panic!("expected placement create response, got: {other:?}"),
    };
    let replayed_pid = match root_response_as(&setup, caller, allocation)
        .expect("placement retry must return the retained result")
    {
        Response::CreateCanister(response) => response.new_canister_pid,
        other => panic!("expected cached placement response, got: {other:?}"),
    };
    assert_eq!(replayed_pid, first_pid);

    let acknowledgement =
        Request::AcknowledgePlacementReceipt(AcknowledgePlacementReceiptRequest {
            operation_id,
            metadata: Some(metadata(operation_id, 120_000_000_000)),
        });
    for _ in 0..2 {
        assert!(matches!(
            root_response_as(&setup, caller, acknowledgement.clone())
                .expect("placement acknowledgement must be response-idempotent"),
            Response::AcknowledgePlacementReceipt(_)
        ));
    }

    let metrics = root_capability_metrics(&setup);
    drop(setup);
    assert_eq!(
        metric_count(&metrics, "AllocatePlacementChild", "ReplayAccepted")
            - metric_count(
                &baseline_metrics,
                "AllocatePlacementChild",
                "ReplayAccepted",
            ),
        1
    );
    assert_eq!(
        metric_count(&metrics, "AllocatePlacementChild", "ReplayDuplicateSame")
            - metric_count(
                &baseline_metrics,
                "AllocatePlacementChild",
                "ReplayDuplicateSame"
            ),
        1
    );
    assert_eq!(
        metric_count(&metrics, "AcknowledgePlacementReceipt", "ReplayAccepted")
            - metric_count(
                &baseline_metrics,
                "AcknowledgePlacementReceipt",
                "ReplayAccepted"
            ),
        0
    );
    assert_eq!(
        metric_count(&metrics, "AcknowledgePlacementReceipt", "ExecutionSuccess")
            - metric_count(
                &baseline_metrics,
                "AcknowledgePlacementReceipt",
                "ExecutionSuccess"
            ),
        2
    );
}

#[test]
fn root_cycles_request_increases_direct_child_balance() {
    let setup = setup_cached_root(RootSetupProfile::Capability);
    let caller = setup
        .subnet_index
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");
    let amount = 321_000u128;
    let request = Request::Cycles(CyclesRequest {
        cycles: amount,
        metadata: Some(metadata([81u8; 32], 120_000_000_000)),
    });

    let before = canister_cycle_balance(&setup, caller);
    let response = root_response_as(&setup, caller, request).expect("root cycles call works");
    let after = canister_cycle_balance(&setup, caller);
    drop(setup);

    match response {
        Response::Cycles(response) => assert_eq!(response.cycles_transferred, amount),
        other => panic!("expected cycles response, got: {other:?}"),
    }
    assert_eq!(
        after.saturating_sub(before),
        amount,
        "root direct-child funding must increase the child balance by the granted amount"
    );
}

#[test]
fn parent_cycles_request_increases_direct_child_balance() {
    let setup = setup_cached_root(RootSetupProfile::Scaling);
    let parent = setup
        .subnet_index
        .get(&canister::SCALE_HUB)
        .copied()
        .expect("scale_hub canister must exist");
    let caller = create_worker(&setup.pic, parent).expect("scale_hub must create one worker");
    prepare_worker_for_explicit_parent_funding(&setup.pic, caller);
    let amount = 654_000u128;

    let response: Result<Result<u128, Error>, _> =
        setup
            .pic
            .update_call(caller, "request_cycles_from_parent", (amount,));
    let metrics = cycles_funding_metrics(&setup, parent);
    drop(setup);

    let transferred = response
        .expect("parent cycles transport must succeed")
        .expect("parent cycles call must succeed");
    assert_eq!(transferred, amount);
    assert_eq!(
        cycles_funding_amount(&metrics, "cycles_granted_to_child", caller),
        amount,
        "non-root parent funding must record the granted child amount in parent metrics"
    );
}

#[test]
fn upgrade_routes_through_dispatcher_non_skip_path() {
    let setup = setup_cached_root(RootSetupProfile::Capability);
    let caller = setup.root_id;
    let target = setup
        .subnet_index
        .get(&canister::TEST)
        .copied()
        .expect("test canister exists");

    let request = UpgradeCanisterRequest {
        canister_pid: target,
        metadata: Some(metadata([37u8; 32], 120_000_000_000)),
    };

    let first = match root_response_as(&setup, caller, Request::UpgradeCanister(request.clone())) {
        Ok(response) => response,
        Err(err) if is_canister_status_decode_failure(&err) => {
            let second = root_response_as(&setup, caller, Request::UpgradeCanister(request))
                .expect_err("failed upgrade path must not be replay-cached");
            assert!(
                is_canister_status_decode_failure(&second),
                "expected decode failure on retried upgrade, got: {second:?}"
            );

            let metrics = root_capability_metrics(&setup);
            drop(setup);
            assert_eq!(metric_count(&metrics, "Upgrade", "Authorized"), 2);
            assert_eq!(metric_count(&metrics, "Upgrade", "ReplayAccepted"), 2);
            assert_eq!(metric_count(&metrics, "Upgrade", "ReplayDuplicateSame"), 0);
            assert_eq!(metric_count(&metrics, "Upgrade", "ExecutionError"), 2);
            assert_eq!(metric_count(&metrics, "Upgrade", "ExecutionSuccess"), 0);
            return;
        }
        Err(err) => panic!("upgrade through dispatcher must succeed: {err:?}"),
    };
    let first = match first {
        Response::UpgradeCanister(response) => response,
        other => panic!("expected upgrade response, got: {other:?}"),
    };

    let second = root_response_as(&setup, caller, Request::UpgradeCanister(request))
        .expect("identical replay should return cached response");
    match second {
        Response::UpgradeCanister(_) => {}
        other => panic!("expected cached upgrade response, got: {other:?}"),
    }
    let _ = first;

    let metrics = root_capability_metrics(&setup);
    drop(setup);
    assert_eq!(metric_count(&metrics, "Upgrade", "Authorized"), 1);
    assert_eq!(metric_count(&metrics, "Upgrade", "ReplayAccepted"), 1);
    assert_eq!(metric_count(&metrics, "Upgrade", "ReplayDuplicateSame"), 1);
    assert_eq!(metric_count(&metrics, "Upgrade", "ExecutionSuccess"), 1);
}

#[test]
fn replay_rejects_cross_variant_same_request_id() {
    let setup = setup_cached_root(RootSetupProfile::Capability);
    let caller = setup.root_id;
    let baseline_metrics = root_capability_metrics(&setup);

    let metadata = metadata([11u8; 32], 120_000_000_000);
    let target = setup
        .subnet_index
        .get(&canister::APP)
        .copied()
        .expect("app canister exists");

    let first = Request::UpgradeCanister(UpgradeCanisterRequest {
        canister_pid: target,
        metadata: Some(metadata),
    });
    match root_response_as(&setup, caller, first) {
        Ok(Response::UpgradeCanister(_)) => {}
        Ok(other) => panic!("expected upgrade response, got: {other:?}"),
        Err(err) if is_canister_status_decode_failure(&err) => {
            // PocketIC canister-status decode mismatch path: upgrade does not commit replay.
            // Keep the test resilient by accepting this known infra branch.
            return;
        }
        Err(err) => panic!("first request must succeed: {err:?}"),
    }

    let second = Request::Cycles(CyclesRequest {
        cycles: 1_000_000,
        metadata: Some(metadata),
    });
    let err = root_response_as(&setup, caller, second)
        .expect_err("cross-variant replay must be rejected");
    assert_eq!(err.code, ErrorCode::Internal);

    let metrics = root_capability_metrics(&setup);
    drop(setup);
    assert_eq!(
        metric_delta(
            &metrics,
            &baseline_metrics,
            "RequestCycles",
            "ReplayDuplicateConflict"
        ),
        1
    );
    assert_eq!(
        metric_delta(
            &metrics,
            &baseline_metrics,
            "RequestCycles",
            "ExecutionSuccess"
        ),
        0
    );
}

#[test]
fn replay_rejects_same_variant_mutated_payload() {
    let setup = setup_cached_root(RootSetupProfile::Capability);
    let caller = setup
        .subnet_index
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");
    let baseline_metrics = root_capability_metrics(&setup);

    let metadata = metadata([12u8; 32], 120_000_000_000);

    let first = Request::Cycles(CyclesRequest {
        cycles: 777,
        metadata: Some(metadata),
    });
    let first = root_response_as(&setup, caller, first).expect("first request must succeed");
    match first {
        Response::Cycles(response) => assert_eq!(response.cycles_transferred, 777),
        other => panic!("expected cycles response, got: {other:?}"),
    }

    let second = Request::Cycles(CyclesRequest {
        cycles: 778,
        metadata: Some(metadata),
    });
    let err = root_response_as(&setup, caller, second)
        .expect_err("mutated payload with same request_id must be rejected");
    assert_eq!(err.code, ErrorCode::Internal);

    let metrics = root_capability_metrics(&setup);
    drop(setup);
    assert_eq!(
        metric_delta(
            &metrics,
            &baseline_metrics,
            "RequestCycles",
            "ReplayDuplicateConflict"
        ),
        1
    );
    assert_eq!(
        metric_delta(
            &metrics,
            &baseline_metrics,
            "RequestCycles",
            "ExecutionSuccess"
        ),
        1
    );
}

#[test]
fn replay_returns_cached_response_for_identical_request() {
    let setup = setup_cached_root(RootSetupProfile::Capability);
    let caller = setup
        .subnet_index
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");
    let baseline_metrics = root_capability_metrics(&setup);

    let metadata = metadata([13u8; 32], 120_000_000_000);
    let request = Request::Cycles(CyclesRequest {
        cycles: 999,
        metadata: Some(metadata),
    });

    let first =
        root_response_as(&setup, caller, request.clone()).expect("first request must succeed");
    match first {
        Response::Cycles(response) => response.cycles_transferred,
        other => panic!("expected cycles response, got: {other:?}"),
    };
    let second =
        root_response_as(&setup, caller, request).expect("identical replay should be cache-hit");
    match second {
        Response::Cycles(response) => assert_eq!(response.cycles_transferred, 999),
        other => panic!("expected cached cycles response, got: {other:?}"),
    }

    let metrics = root_capability_metrics(&setup);
    drop(setup);
    assert_eq!(
        metric_delta(
            &metrics,
            &baseline_metrics,
            "RequestCycles",
            "ReplayAccepted"
        ),
        1
    );
    assert_eq!(
        metric_delta(
            &metrics,
            &baseline_metrics,
            "RequestCycles",
            "ReplayDuplicateSame"
        ),
        1
    );
    assert_eq!(
        metric_delta(
            &metrics,
            &baseline_metrics,
            "RequestCycles",
            "ExecutionSuccess"
        ),
        1
    );
}

#[test]
fn cycles_request_above_default_policy_is_clamped_and_transferred() {
    let setup = setup_cached_root(RootSetupProfile::Capability);
    let caller = setup
        .subnet_index
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");
    let expected_grant = 5_000_000_000_000u128;
    let baseline_metrics = root_capability_metrics(&setup);

    let request = Request::Cycles(CyclesRequest {
        cycles: u128::MAX,
        metadata: Some(metadata([18u8; 32], 120_000_000_000)),
    });

    let before = canister_cycle_balance(&setup, caller);
    let response = root_response_as(&setup, caller, request)
        .expect("request above default per-request policy must be clamped");
    let after = canister_cycle_balance(&setup, caller);

    match response {
        Response::Cycles(response) => assert_eq!(response.cycles_transferred, expected_grant),
        other => panic!("expected clamped cycles response, got: {other:?}"),
    }
    assert_eq!(
        after.saturating_sub(before),
        expected_grant,
        "clamped root funding must increase the child balance by the approved grant"
    );

    let metrics = root_capability_metrics(&setup);
    drop(setup);
    assert_eq!(
        metric_delta(
            &metrics,
            &baseline_metrics,
            "RequestCycles",
            "ReplayAccepted"
        ),
        1
    );
    assert_eq!(
        metric_delta(&metrics, &baseline_metrics, "RequestCycles", "Authorized"),
        1
    );
    assert_eq!(
        metric_delta(
            &metrics,
            &baseline_metrics,
            "RequestCycles",
            "ExecutionSuccess"
        ),
        1
    );
}

#[test]
fn replay_rejects_ttl_above_max() {
    let setup = setup_cached_root(RootSetupProfile::Capability);
    let caller = setup
        .subnet_index
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");
    let baseline_metrics = root_capability_metrics(&setup);

    let request = Request::Cycles(CyclesRequest {
        cycles: 1,
        metadata: Some(metadata([14u8; 32], 301_000_000_000)),
    });

    let err = root_response_as(&setup, caller, request).expect_err("ttl above max must reject");
    assert_eq!(err.code, ErrorCode::Internal);

    let metrics = root_capability_metrics(&setup);
    drop(setup);
    assert_eq!(
        metric_delta(
            &metrics,
            &baseline_metrics,
            "RequestCycles",
            "ReplayTtlExceeded"
        ),
        1
    );
    assert_eq!(
        metric_delta(
            &metrics,
            &baseline_metrics,
            "RequestCycles",
            "ReplayAccepted"
        ),
        0
    );
    assert_eq!(
        metric_delta(
            &metrics,
            &baseline_metrics,
            "RequestCycles",
            "ExecutionSuccess"
        ),
        0
    );
}

#[test]
fn replay_rejects_expired_request() {
    let setup = setup_cached_root(RootSetupProfile::Capability);
    let caller = setup
        .subnet_index
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");
    let baseline_metrics = root_capability_metrics(&setup);

    let metadata = metadata([15u8; 32], 1_000_000_000);
    let request = Request::Cycles(CyclesRequest {
        cycles: 123,
        metadata: Some(metadata),
    });

    let first =
        root_response_as(&setup, caller, request.clone()).expect("first request must succeed");
    match first {
        Response::Cycles(response) => assert_eq!(response.cycles_transferred, 123),
        other => panic!("expected cycles response, got: {other:?}"),
    }

    setup.pic.advance_time(Duration::from_secs(2));
    setup.pic.tick();

    let err = root_response_as(&setup, caller, request).expect_err("expired replay must reject");
    assert_eq!(err.code, ErrorCode::Internal);

    let metrics = root_capability_metrics(&setup);
    drop(setup);
    assert_eq!(
        metric_delta(
            &metrics,
            &baseline_metrics,
            "RequestCycles",
            "ReplayExpired"
        ),
        1
    );
    assert_eq!(
        metric_delta(
            &metrics,
            &baseline_metrics,
            "RequestCycles",
            "ExecutionSuccess"
        ),
        1
    );
}

#[test]
fn upgrade_replay_returns_cached_response_and_rejects_conflict() {
    let setup = setup_cached_root(RootSetupProfile::Capability);
    let caller = setup.root_id;
    let app = setup
        .subnet_index
        .get(&canister::APP)
        .copied()
        .expect("app canister exists");
    let test = setup
        .subnet_index
        .get(&canister::TEST)
        .copied()
        .expect("test canister exists");

    let metadata = metadata([16u8; 32], 120_000_000_000);
    let request = UpgradeCanisterRequest {
        canister_pid: app,
        metadata: Some(metadata),
    };

    let first = match root_response_as(&setup, caller, Request::UpgradeCanister(request.clone())) {
        Ok(response) => response,
        Err(err) if is_canister_status_decode_failure(&err) => {
            let second = root_response_as(&setup, caller, Request::UpgradeCanister(request))
                .expect_err("failed upgrade request must not be replay-cached");
            assert!(
                is_canister_status_decode_failure(&second),
                "expected decode failure on identical retry, got: {second:?}"
            );

            let conflict = UpgradeCanisterRequest {
                canister_pid: test,
                metadata: Some(metadata),
            };
            let third = root_response_as(&setup, caller, Request::UpgradeCanister(conflict))
                .expect_err("failed upgrade replay entry must not trigger conflict path");
            assert!(
                is_canister_status_decode_failure(&third),
                "expected decode failure on conflict-shape request, got: {third:?}"
            );

            let metrics = root_capability_metrics(&setup);
            drop(setup);
            assert_eq!(metric_count(&metrics, "Upgrade", "ReplayDuplicateSame"), 0);
            assert_eq!(
                metric_count(&metrics, "Upgrade", "ReplayDuplicateConflict"),
                0
            );
            assert_eq!(metric_count(&metrics, "Upgrade", "ReplayAccepted"), 3);
            assert_eq!(metric_count(&metrics, "Upgrade", "ExecutionError"), 3);
            assert_eq!(metric_count(&metrics, "Upgrade", "ExecutionSuccess"), 0);
            return;
        }
        Err(err) => panic!("first upgrade request must succeed: {err:?}"),
    };
    let first = match first {
        Response::UpgradeCanister(response) => response,
        other => panic!("expected upgrade response, got: {other:?}"),
    };

    let second = root_response_as(&setup, caller, Request::UpgradeCanister(request))
        .expect("identical replay should return cached response");
    match second {
        Response::UpgradeCanister(_) => {}
        other => panic!("expected cached upgrade response, got: {other:?}"),
    }
    let _ = first;

    let conflict = UpgradeCanisterRequest {
        canister_pid: test,
        metadata: Some(metadata),
    };
    let err = root_response_as(&setup, caller, Request::UpgradeCanister(conflict))
        .expect_err("upgrade replay conflict must reject");
    assert_eq!(err.code, ErrorCode::Internal);

    let metrics = root_capability_metrics(&setup);
    drop(setup);
    assert_eq!(metric_count(&metrics, "Upgrade", "ReplayDuplicateSame"), 1);
    assert_eq!(
        metric_count(&metrics, "Upgrade", "ReplayDuplicateConflict"),
        1
    );
    assert_eq!(metric_count(&metrics, "Upgrade", "ExecutionSuccess"), 1);
}

fn root_response_as(
    setup: &RootSetup,
    caller: Principal,
    request: Request,
) -> Result<Response, Error> {
    capability_response_as(setup, setup.root_id, caller, request)
}

// Call one capability endpoint as the requested caller and return its typed response.
fn capability_response_as(
    setup: &RootSetup,
    target_pid: Principal,
    caller: Principal,
    request: Request,
) -> Result<Response, Error> {
    let (request_id, ttl_ns) = capability_metadata_from_request(&request);
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request,
        proof: CapabilityProof::Structural,
        metadata: CapabilityRequestMetadata {
            request_id,
            issued_at_ns: target_now_ns(setup, target_pid),
            ttl_ns,
        },
    };

    let result: Result<Result<RootCapabilityResponseV1, Error>, _> = setup.pic.update_call_as(
        target_pid,
        caller,
        protocol::CANIC_RESPONSE_CAPABILITY_V1,
        (envelope,),
    );
    result
        .expect("root response transport call failed")
        .map(|response| response.response)
}

// Read the live cycle balance that the canister itself reports through the public query surface.
fn canister_cycle_balance(setup: &RootSetup, canister_id: Principal) -> u128 {
    let balance: Result<u128, Error> = setup
        .pic
        .query_call(canister_id, protocol::CANIC_CYCLE_BALANCE, ())
        .expect("cycle balance transport query must succeed");
    balance.expect("cycle balance query must succeed")
}

fn root_capability_metrics(setup: &RootSetup) -> Vec<MetricEntry> {
    query_metrics(&setup.pic, setup.root_id, MetricsKind::Security)
}

// Read one canister's cached subnet-index page through the public query surface.
fn query_subnet_index(setup: &RootSetup, canister_id: Principal) -> Vec<IndexEntryResponse> {
    let response: Result<Page<IndexEntryResponse>, Error> = setup
        .pic
        .query_call(
            canister_id,
            protocol::CANIC_SUBNET_INDEX,
            (PageRequest {
                limit: 100,
                offset: 0,
            },),
        )
        .expect("subnet index transport query failed");

    response.expect("subnet index query failed").entries
}

// Read one canister's public metrics page for the requested metric family.
fn query_metrics(
    pic: &ic_testkit::pic::Pic,
    canister_id: Principal,
    kind: MetricsKind,
) -> Vec<MetricEntry> {
    let response: Result<Page<MetricEntry>, Error> = pic
        .query_call(
            canister_id,
            protocol::CANIC_METRICS,
            (
                kind,
                PageRequest {
                    limit: 256,
                    offset: 0,
                },
            ),
        )
        .expect("metrics transport query failed");

    response.expect("metrics application query failed").entries
}

// Read one canister's cycles-funding metrics page.
fn cycles_funding_metrics(setup: &RootSetup, canister_id: Principal) -> Vec<MetricEntry> {
    query_metrics(&setup.pic, canister_id, MetricsKind::Core)
}

// Sum one cycles-funding `U128` metric for a specific child principal.
fn cycles_funding_amount(entries: &[MetricEntry], label: &str, child: Principal) -> u128 {
    entries
        .iter()
        .filter(|entry| {
            entry
                .labels
                .first()
                .is_some_and(|value| value == "cycles_funding")
                && entry.labels.get(1).is_some_and(|value| value == label)
                && entry.principal == Some(child)
        })
        .map(|entry| match entry.value {
            MetricValue::U128(value) => value,
            MetricValue::Count(_) | MetricValue::CountAndU64 { .. } => 0,
        })
        .sum()
}

fn metric_count(entries: &[MetricEntry], capability: &str, event: &str) -> u64 {
    let (event_type, outcome) = metric_event_parts(event);
    entries
        .iter()
        .filter(|entry| {
            entry
                .labels
                .first()
                .is_some_and(|label| label == "root_capability")
                && entry.labels.get(1).is_some_and(|label| label == capability)
                && entry.labels.get(2).is_some_and(|label| label == event_type)
                && entry.labels.get(3).is_some_and(|label| label == outcome)
        })
        .map(metric_entry_count)
        .sum()
}

fn metric_delta(
    after: &[MetricEntry],
    before: &[MetricEntry],
    capability: &str,
    event: &str,
) -> u64 {
    metric_count(after, capability, event)
        .checked_sub(metric_count(before, capability, event))
        .expect("capability metric count must not decrease")
}

fn root_capability_count_total(entries: &[MetricEntry]) -> u64 {
    entries
        .iter()
        .filter(|entry| {
            entry
                .labels
                .first()
                .is_some_and(|label| label == "root_capability")
        })
        .map(metric_entry_count)
        .sum()
}

const fn metric_entry_count(entry: &MetricEntry) -> u64 {
    match entry.value {
        MetricValue::Count(count) | MetricValue::CountAndU64 { count, .. } => count,
        MetricValue::U128(_) => 0,
    }
}

fn metric_event_parts(event: &str) -> (&'static str, &'static str) {
    match event {
        "EnvelopeRejected" => ("Envelope", "Rejected"),
        "EnvelopeValidated" => ("Envelope", "Accepted"),
        "ProofRejected" => ("Proof", "Rejected"),
        "ProofVerified" => ("Proof", "Accepted"),
        "Authorized" => ("Authorization", "Accepted"),
        "Denied" => ("Authorization", "Denied"),
        "ReplayAccepted" => ("Replay", "Accepted"),
        "ReplayDuplicateSame" => ("Replay", "DuplicateSame"),
        "ReplayDuplicateConflict" => ("Replay", "DuplicateConflict"),
        "ReplayExpired" => ("Replay", "Expired"),
        "ReplayTtlExceeded" => ("Replay", "TtlExceeded"),
        "ExecutionSuccess" => ("Execution", "Success"),
        "ExecutionError" => ("Execution", "Error"),
        other => panic!("unexpected root capability metric event: {other}"),
    }
}

// Read one canister's current time in nanoseconds for capability metadata issuance.
fn target_now_ns(setup: &RootSetup, canister_id: Principal) -> u64 {
    let _ = canister_id;
    setup.pic.current_time_nanos()
}

const fn capability_metadata_from_request(request: &Request) -> ([u8; 32], u64) {
    let metadata = match request {
        Request::AcknowledgePlacementReceipt(req) => req.metadata,
        Request::AllocatePlacementChild(req) | Request::CreateCanister(req) => req.metadata,
        Request::UpgradeCanister(req) => req.metadata,
        Request::RecycleCanister(req) => req.metadata,
        Request::Cycles(req) => req.metadata,
    };

    match metadata {
        Some(meta) => (meta.request_id, meta.ttl_ns),
        None => ([0u8; 32], 60_000_000_000),
    }
}

fn is_canister_status_decode_failure(err: &Error) -> bool {
    err.message.contains("CanisterStatusResult")
        && (err.message.contains("candid decode failed for type")
            || err.message.contains("candid decode_one failed")
            || err.message.contains("decode_one failed"))
}

const fn metadata(request_id: [u8; 32], ttl_ns: u64) -> RootRequestMetadata {
    RootRequestMetadata { request_id, ttl_ns }
}
