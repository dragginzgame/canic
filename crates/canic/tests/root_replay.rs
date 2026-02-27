// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

mod root;

use canic::{
    Error,
    cdk::types::Principal,
    dto::{
        auth::DelegationRequest,
        error::ErrorCode,
        metrics::RootCapabilityMetricEntry,
        page::{Page, PageRequest},
        rpc::{
            CreateCanisterParent, CreateCanisterRequest, CyclesRequest, Request, Response,
            RootRequestMetadata, UpgradeCanisterRequest,
        },
    },
    ids::cap,
    protocol,
};
use canic_internal::canister;
use root::harness::{RootSetup, setup_root};
use std::{env, time::Duration};

const REQUIRE_THRESHOLD_KEYS_ENV: &str = "CANIC_REQUIRE_THRESHOLD_KEYS";

#[test]
fn unauthorized_caller_is_denied_for_each_root_capability_variant() {
    let setup = setup_root();
    let test_pid = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");
    let unauthorized = Principal::from_slice(&[250; 29]);

    let cases = vec![
        Request::CreateCanister(CreateCanisterRequest {
            canister_role: canister::SCALE,
            parent: CreateCanisterParent::ThisCanister,
            extra_arg: None,
            metadata: Some(metadata([30u8; 32], 120)),
        }),
        Request::UpgradeCanister(UpgradeCanisterRequest {
            canister_pid: test_pid,
            metadata: Some(metadata([31u8; 32], 120)),
        }),
        Request::Cycles(CyclesRequest {
            cycles: 1_000_000,
            metadata: Some(metadata([32u8; 32], 120)),
        }),
        Request::IssueDelegation(DelegationRequest {
            shard_pid: test_pid,
            scopes: vec![cap::VERIFY.to_string()],
            aud: vec![test_pid],
            ttl_secs: 60,
            verifier_targets: Vec::new(),
            include_root_verifier: false,
            metadata: Some(metadata([33u8; 32], 120)),
        }),
    ];

    for request in cases {
        let err = root_response_as(&setup, unauthorized, request)
            .expect_err("unregistered caller must be rejected at endpoint boundary");
        assert_eq!(err.code, ErrorCode::Unauthorized);
    }

    let metrics = root_capability_metrics(&setup);
    assert!(
        metrics.is_empty(),
        "root capability metrics must stay empty when endpoint auth rejects calls before dispatch"
    );
}

#[test]
fn upgrade_policy_denies_registered_non_parent_caller() {
    let setup = setup_root();
    let caller = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");
    let app_pid = setup
        .subnet_directory
        .get(&canister::APP)
        .copied()
        .expect("app canister must exist");

    let request = Request::UpgradeCanister(UpgradeCanisterRequest {
        canister_pid: app_pid,
        metadata: Some(metadata([34u8; 32], 120)),
    });
    let err = root_response_as(&setup, caller, request)
        .expect_err("non-parent caller must be denied by upgrade policy");
    assert_eq!(err.code, ErrorCode::Internal);

    let metrics = root_capability_metrics(&setup);
    assert_eq!(metric_count(&metrics, "Upgrade", "Denied"), 1);
    assert_eq!(metric_count(&metrics, "Upgrade", "Authorized"), 0);
    assert_eq!(metric_count(&metrics, "Upgrade", "ExecutionSuccess"), 0);
}

#[test]
fn delegation_policy_denies_caller_shard_mismatch() {
    let setup = setup_root();
    let caller = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");
    let mismatched_shard = setup
        .subnet_directory
        .get(&canister::SCALE_HUB)
        .copied()
        .expect("scale_hub canister must exist");

    let request = Request::IssueDelegation(DelegationRequest {
        shard_pid: mismatched_shard,
        scopes: vec![cap::VERIFY.to_string()],
        aud: vec![caller],
        ttl_secs: 60,
        verifier_targets: Vec::new(),
        include_root_verifier: false,
        metadata: Some(metadata([35u8; 32], 120)),
    });
    let err = root_response_as(&setup, caller, request)
        .expect_err("mismatched shard_pid must be denied by delegation policy");
    assert_eq!(err.code, ErrorCode::Internal);

    let metrics = root_capability_metrics(&setup);
    assert_eq!(metric_count(&metrics, "IssueDelegation", "Denied"), 1);
    assert_eq!(metric_count(&metrics, "IssueDelegation", "Authorized"), 0);
    assert_eq!(
        metric_count(&metrics, "IssueDelegation", "ExecutionSuccess"),
        0
    );
}

#[test]
fn provisioning_routes_through_dispatcher_and_replay_cache() {
    let setup = setup_root();
    let caller = setup
        .subnet_directory
        .get(&canister::SCALE_HUB)
        .copied()
        .expect("scale_hub canister must exist");

    let request = Request::CreateCanister(CreateCanisterRequest {
        canister_role: canister::SCALE,
        parent: CreateCanisterParent::ThisCanister,
        extra_arg: None,
        metadata: Some(metadata([36u8; 32], 120)),
    });

    let first = root_response_as(&setup, caller, request.clone())
        .expect("first provisioning request must succeed");
    let first_pid = match first {
        Response::CreateCanister(response) => response.new_canister_pid,
        other => panic!("expected create canister response, got: {other:?}"),
    };

    let second = root_response_as(&setup, caller, request)
        .expect("identical provisioning replay must cache");
    let second_pid = match second {
        Response::CreateCanister(response) => response.new_canister_pid,
        other => panic!("expected create canister response, got: {other:?}"),
    };
    assert_eq!(first_pid, second_pid);

    let metrics = root_capability_metrics(&setup);
    assert_eq!(metric_count(&metrics, "Provision", "Authorized"), 2);
    assert_eq!(metric_count(&metrics, "Provision", "ReplayAccepted"), 1);
    assert_eq!(
        metric_count(&metrics, "Provision", "ReplayDuplicateSame"),
        1
    );
    assert_eq!(metric_count(&metrics, "Provision", "ExecutionSuccess"), 1);
}

#[test]
fn delegation_issuance_routes_through_dispatcher_non_skip_path() {
    let setup = setup_root();
    let caller = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");

    let request = DelegationRequest {
        shard_pid: caller,
        scopes: vec![cap::VERIFY.to_string()],
        aud: vec![caller],
        ttl_secs: 60,
        verifier_targets: Vec::new(),
        include_root_verifier: false,
        metadata: Some(metadata([37u8; 32], 120)),
    };

    let first = match root_response_as(&setup, caller, Request::IssueDelegation(request.clone())) {
        Ok(response) => response,
        Err(err) if is_threshold_key_unavailable(&err) => {
            assert!(
                !require_threshold_keys(),
                "threshold key unavailable while {REQUIRE_THRESHOLD_KEYS_ENV}=1: {}",
                err.message
            );
            eprintln!(
                "skipping non-skip delegation dispatcher assertions: threshold key unavailable: {}",
                err.message
            );
            return;
        }
        Err(err) => panic!("delegation issuance through dispatcher must succeed: {err:?}"),
    };
    let first = match first {
        Response::DelegationIssued(response) => response,
        other => panic!("expected delegation response, got: {other:?}"),
    };
    assert!(!first.proof.cert_sig.is_empty());

    let second = root_response_as(&setup, caller, Request::IssueDelegation(request))
        .expect("identical delegation replay must return cached response");
    let second = match second {
        Response::DelegationIssued(response) => response,
        other => panic!("expected delegation response, got: {other:?}"),
    };
    assert_eq!(first.proof.cert_sig, second.proof.cert_sig);

    let metrics = root_capability_metrics(&setup);
    assert_eq!(metric_count(&metrics, "IssueDelegation", "Authorized"), 2);
    assert_eq!(
        metric_count(&metrics, "IssueDelegation", "ReplayAccepted"),
        1
    );
    assert_eq!(
        metric_count(&metrics, "IssueDelegation", "ReplayDuplicateSame"),
        1
    );
    assert_eq!(
        metric_count(&metrics, "IssueDelegation", "ExecutionSuccess"),
        1
    );
}

#[test]
fn replay_rejects_cross_variant_same_request_id() {
    let setup = setup_root();
    let test_pid = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");

    let metadata = metadata([11u8; 32], 120);

    let first = Request::UpgradeCanister(UpgradeCanisterRequest {
        canister_pid: test_pid,
        metadata: Some(metadata),
    });
    let first = root_response_as(&setup, setup.root_id, first).expect("first request must succeed");
    match first {
        Response::UpgradeCanister(_) => {}
        other => panic!("expected upgrade response, got: {other:?}"),
    }

    let second = Request::Cycles(CyclesRequest {
        cycles: 1_000_000,
        metadata: Some(metadata),
    });
    let err = root_response_as(&setup, setup.root_id, second)
        .expect_err("cross-variant replay must be rejected");
    assert_eq!(err.code, ErrorCode::Internal);

    let metrics = root_capability_metrics(&setup);
    assert_eq!(
        metric_count(&metrics, "MintCycles", "ReplayDuplicateConflict"),
        1
    );
    assert_eq!(metric_count(&metrics, "MintCycles", "ExecutionSuccess"), 0);
}

#[test]
fn replay_rejects_same_variant_mutated_payload() {
    let setup = setup_root();
    let caller = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");

    let metadata = metadata([12u8; 32], 120);

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
    assert_eq!(
        metric_count(&metrics, "MintCycles", "ReplayDuplicateConflict"),
        1
    );
    assert_eq!(metric_count(&metrics, "MintCycles", "ExecutionSuccess"), 1);
}

#[test]
fn replay_returns_cached_response_for_identical_request() {
    let setup = setup_root();
    let caller = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");

    let metadata = metadata([13u8; 32], 120);
    let request = Request::Cycles(CyclesRequest {
        cycles: 999,
        metadata: Some(metadata),
    });

    let first =
        root_response_as(&setup, caller, request.clone()).expect("first request must succeed");
    let second =
        root_response_as(&setup, caller, request).expect("identical replay must return cached");

    let first_cycles = match first {
        Response::Cycles(response) => response.cycles_transferred,
        other => panic!("expected cycles response, got: {other:?}"),
    };
    let second_cycles = match second {
        Response::Cycles(response) => response.cycles_transferred,
        other => panic!("expected cycles response, got: {other:?}"),
    };
    assert_eq!(first_cycles, second_cycles);

    let metrics = root_capability_metrics(&setup);
    assert_eq!(metric_count(&metrics, "MintCycles", "ReplayAccepted"), 1);
    assert_eq!(
        metric_count(&metrics, "MintCycles", "ReplayDuplicateSame"),
        1
    );
    assert_eq!(metric_count(&metrics, "MintCycles", "ExecutionSuccess"), 1);
}

#[test]
fn replay_rejects_ttl_above_max() {
    let setup = setup_root();
    let caller = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");

    let request = Request::Cycles(CyclesRequest {
        cycles: 1,
        metadata: Some(metadata([14u8; 32], 301)),
    });

    let err = root_response_as(&setup, caller, request).expect_err("ttl above max must reject");
    assert_eq!(err.code, ErrorCode::Internal);

    let metrics = root_capability_metrics(&setup);
    assert_eq!(metric_count(&metrics, "MintCycles", "ReplayTtlExceeded"), 1);
    assert_eq!(metric_count(&metrics, "MintCycles", "ReplayAccepted"), 0);
    assert_eq!(metric_count(&metrics, "MintCycles", "ExecutionSuccess"), 0);
}

#[test]
fn replay_rejects_expired_request() {
    let setup = setup_root();
    let caller = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");

    let metadata = metadata([15u8; 32], 1);
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
    assert_eq!(metric_count(&metrics, "MintCycles", "ReplayExpired"), 1);
    assert_eq!(metric_count(&metrics, "MintCycles", "ExecutionSuccess"), 1);
}

#[test]
fn delegation_replay_returns_same_signature_and_rejects_conflict() {
    let setup = setup_root();
    let caller = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");

    let metadata = metadata([16u8; 32], 120);
    let request = DelegationRequest {
        shard_pid: caller,
        scopes: vec![cap::VERIFY.to_string()],
        aud: vec![caller],
        ttl_secs: 60,
        verifier_targets: Vec::new(),
        include_root_verifier: false,
        metadata: Some(metadata),
    };

    let first = match root_response_as(&setup, caller, Request::IssueDelegation(request.clone())) {
        Ok(response) => response,
        Err(err) if is_threshold_key_unavailable(&err) => {
            eprintln!(
                "skipping delegation replay assertions: threshold key unavailable: {}",
                err.message
            );
            return;
        }
        Err(err) => panic!("first delegation request must succeed: {err:?}"),
    };
    let first = match first {
        Response::DelegationIssued(response) => response,
        other => panic!("expected delegation response, got: {other:?}"),
    };

    let second = root_response_as(&setup, caller, Request::IssueDelegation(request))
        .expect("identical delegation replay must return cached response");
    let second = match second {
        Response::DelegationIssued(response) => response,
        other => panic!("expected delegation response, got: {other:?}"),
    };
    assert_eq!(first.proof.cert_sig, second.proof.cert_sig);

    let conflict = DelegationRequest {
        shard_pid: caller,
        scopes: vec![cap::VERIFY.to_string(), cap::READ.to_string()],
        aud: vec![caller],
        ttl_secs: 60,
        verifier_targets: Vec::new(),
        include_root_verifier: false,
        metadata: Some(metadata),
    };
    let err = root_response_as(&setup, caller, Request::IssueDelegation(conflict))
        .expect_err("delegation replay conflict must reject");
    assert_eq!(err.code, ErrorCode::Internal);

    let metrics = root_capability_metrics(&setup);
    assert_eq!(
        metric_count(&metrics, "IssueDelegation", "ReplayDuplicateSame"),
        1
    );
    assert_eq!(
        metric_count(&metrics, "IssueDelegation", "ReplayDuplicateConflict"),
        1
    );
    assert_eq!(
        metric_count(&metrics, "IssueDelegation", "ExecutionSuccess"),
        1
    );
}

#[test]
fn delegation_execution_error_does_not_commit_replay_entry() {
    let setup = setup_root();
    let caller = setup.root_id;
    let metadata = metadata([17u8; 32], 120);

    let invalid = DelegationRequest {
        shard_pid: caller,
        scopes: vec![cap::VERIFY.to_string()],
        aud: vec![caller],
        ttl_secs: 60,
        verifier_targets: Vec::new(),
        include_root_verifier: false,
        metadata: Some(metadata),
    };

    let first = root_response_as(&setup, caller, Request::IssueDelegation(invalid.clone()))
        .expect_err("invalid delegation request must fail");
    assert_eq!(first.code, ErrorCode::Internal);

    let second = root_response_as(&setup, caller, Request::IssueDelegation(invalid))
        .expect_err("failed delegation replay must not be committed");
    assert_eq!(second.code, ErrorCode::Internal);

    let metrics = root_capability_metrics(&setup);
    assert_eq!(
        metric_count(&metrics, "IssueDelegation", "ReplayAccepted"),
        2
    );
    assert_eq!(
        metric_count(&metrics, "IssueDelegation", "ExecutionError"),
        2
    );
    assert_eq!(
        metric_count(&metrics, "IssueDelegation", "ReplayDuplicateSame"),
        0
    );
    assert_eq!(
        metric_count(&metrics, "IssueDelegation", "ReplayDuplicateConflict"),
        0
    );
}

fn root_response_as(
    setup: &RootSetup,
    caller: Principal,
    request: Request,
) -> Result<Response, Error> {
    let result: Result<Result<Response, Error>, Error> =
        setup
            .pic
            .update_call_as(setup.root_id, caller, protocol::CANIC_RESPONSE, (request,));
    result.expect("root response transport call failed")
}

fn root_capability_metrics(setup: &RootSetup) -> Vec<RootCapabilityMetricEntry> {
    let page: Result<Page<RootCapabilityMetricEntry>, Error> = setup
        .pic
        .query_call(
            setup.root_id,
            protocol::CANIC_METRICS_ROOT_CAPABILITY,
            (PageRequest {
                limit: 256,
                offset: 0,
            },),
        )
        .expect("root capability metrics transport query failed");
    page.expect("root capability metrics application query failed")
        .entries
}

fn metric_count(entries: &[RootCapabilityMetricEntry], capability: &str, event: &str) -> u64 {
    entries
        .iter()
        .filter(|entry| entry.capability == capability && entry.event == event)
        .map(|entry| entry.count)
        .sum()
}

fn is_threshold_key_unavailable(err: &Error) -> bool {
    err.message.contains("Requested unknown threshold key")
        || err.message.contains("existing keys: []")
}

fn require_threshold_keys() -> bool {
    env::var(REQUIRE_THRESHOLD_KEYS_ENV)
        .map(|value| {
            value.eq_ignore_ascii_case("1")
                || value.eq_ignore_ascii_case("true")
                || value.eq_ignore_ascii_case("yes")
        })
        .unwrap_or(false)
}

const fn metadata(request_id: [u8; 32], ttl_seconds: u64) -> RootRequestMetadata {
    RootRequestMetadata {
        request_id,
        ttl_seconds,
    }
}
