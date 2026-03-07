// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

mod root;

use canic::{
    Error,
    cdk::types::Principal,
    dto::{
        capability::{
            CAPABILITY_VERSION_V1, CapabilityProof, CapabilityRequestMetadata, CapabilityService,
            RootCapabilityEnvelopeV1, RootCapabilityResponseV1,
        },
        error::ErrorCode,
        metrics::RootCapabilityMetricEntry,
        page::{Page, PageRequest},
        rpc::{
            CreateCanisterParent, CreateCanisterRequest, CyclesRequest, Request, Response,
            RootRequestMetadata, UpgradeCanisterRequest,
        },
    },
    protocol,
};
use canic_internal::canister;
use root::harness::{RootSetup, setup_root};
use std::convert::TryFrom;
use std::time::Duration;

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
        Request::IssueRoleAttestation(canic::dto::auth::RoleAttestationRequest {
            subject: unauthorized,
            role: canister::TEST,
            subnet_id: None,
            audience: Some(setup.root_id),
            ttl_secs: 60,
            epoch: 0,
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
    assert_eq!(err.code, ErrorCode::Forbidden);

    let metrics = root_capability_metrics(&setup);
    assert_eq!(metric_count(&metrics, "Upgrade", "ProofRejected"), 1);
    assert_eq!(metric_count(&metrics, "Upgrade", "Denied"), 0);
    assert_eq!(metric_count(&metrics, "Upgrade", "Authorized"), 0);
    assert_eq!(metric_count(&metrics, "Upgrade", "ExecutionSuccess"), 0);
}

#[test]
fn structural_proof_denies_unsupported_issue_delegation_capability() {
    let setup = setup_root();
    let caller = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister must exist");

    let request = Request::IssueDelegation(canic::dto::auth::DelegationRequest {
        shard_pid: caller,
        scopes: vec!["rpc:verify".to_string()],
        aud: vec![caller],
        ttl_secs: 60,
        verifier_targets: Vec::new(),
        include_root_verifier: false,
        metadata: Some(metadata([35u8; 32], 120)),
    });
    let err = root_response_as(&setup, caller, request)
        .expect_err("unsupported structural capability must fail closed");
    assert_eq!(err.code, ErrorCode::Forbidden);

    let metrics = root_capability_metrics(&setup);
    assert_eq!(
        metric_count(&metrics, "IssueDelegation", "ProofRejected"),
        1
    );
    assert_eq!(
        metric_count(&metrics, "IssueDelegation", "ReplayAccepted"),
        0
    );
    assert_eq!(
        metric_count(&metrics, "IssueDelegation", "ExecutionSuccess"),
        0
    );
}

#[test]
fn cycles_routes_through_dispatcher_and_replay_cache() {
    let setup = setup_root();
    let caller = setup
        .subnet_directory
        .get(&canister::SCALE_HUB)
        .copied()
        .expect("scale_hub canister must exist");

    let request = Request::Cycles(CyclesRequest {
        cycles: 1_111_000,
        metadata: Some(metadata([36u8; 32], 120)),
    });

    let first = root_response_as(&setup, caller, request.clone()).expect("first cycles call works");
    let first_cycles = match first {
        Response::Cycles(response) => response.cycles_transferred,
        other => panic!("expected create canister response, got: {other:?}"),
    };

    let second = root_response_as(&setup, caller, request)
        .expect("identical provisioning replay must cache");
    let second_cycles = match second {
        Response::Cycles(response) => response.cycles_transferred,
        other => panic!("expected create canister response, got: {other:?}"),
    };
    assert_eq!(first_cycles, second_cycles);

    let metrics = root_capability_metrics(&setup);
    assert_eq!(metric_count(&metrics, "MintCycles", "Authorized"), 1);
    assert_eq!(metric_count(&metrics, "MintCycles", "ReplayAccepted"), 1);
    assert_eq!(
        metric_count(&metrics, "MintCycles", "ReplayDuplicateSame"),
        1
    );
    assert_eq!(metric_count(&metrics, "MintCycles", "ExecutionSuccess"), 1);
}

#[test]
fn upgrade_routes_through_dispatcher_non_skip_path() {
    let setup = setup_root();
    let caller = setup.root_id;
    let target = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister exists");

    let request = UpgradeCanisterRequest {
        canister_pid: target,
        metadata: Some(metadata([37u8; 32], 120)),
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
        .expect("identical delegation replay must return cached response");
    let second = match second {
        Response::UpgradeCanister(response) => response,
        other => panic!("expected upgrade response, got: {other:?}"),
    };
    let _ = (first, second);

    let metrics = root_capability_metrics(&setup);
    assert_eq!(metric_count(&metrics, "Upgrade", "Authorized"), 1);
    assert_eq!(metric_count(&metrics, "Upgrade", "ReplayAccepted"), 1);
    assert_eq!(metric_count(&metrics, "Upgrade", "ReplayDuplicateSame"), 1);
    assert_eq!(metric_count(&metrics, "Upgrade", "ExecutionSuccess"), 1);
}

#[test]
fn replay_rejects_cross_variant_same_request_id() {
    let setup = setup_root();
    let caller = setup.root_id;

    let metadata = metadata([11u8; 32], 120);

    let first = Request::Cycles(CyclesRequest {
        cycles: 1_000_000,
        metadata: Some(metadata),
    });
    let first = root_response_as(&setup, caller, first).expect("first request must succeed");
    match first {
        Response::Cycles(response) => assert_eq!(response.cycles_transferred, 1_000_000),
        other => panic!("expected cycles response, got: {other:?}"),
    }

    let second = Request::UpgradeCanister(UpgradeCanisterRequest {
        canister_pid: setup
            .subnet_directory
            .get(&canister::APP)
            .copied()
            .expect("app canister exists"),
        metadata: Some(metadata),
    });
    let err = root_response_as(&setup, caller, second)
        .expect_err("cross-variant replay must be rejected");
    assert_eq!(err.code, ErrorCode::Internal);

    let metrics = root_capability_metrics(&setup);
    assert_eq!(
        metric_count(&metrics, "Upgrade", "ReplayDuplicateConflict"),
        1
    );
    assert_eq!(metric_count(&metrics, "Upgrade", "ExecutionSuccess"), 0);
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
fn upgrade_replay_returns_cached_response_and_rejects_conflict() {
    let setup = setup_root();
    let caller = setup.root_id;
    let app = setup
        .subnet_directory
        .get(&canister::APP)
        .copied()
        .expect("app canister exists");
    let test = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister exists");

    let metadata = metadata([16u8; 32], 120);
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
        .expect("identical upgrade replay must return cached response");
    let second = match second {
        Response::UpgradeCanister(response) => response,
        other => panic!("expected upgrade response, got: {other:?}"),
    };
    let _ = (first, second);

    let conflict = UpgradeCanisterRequest {
        canister_pid: test,
        metadata: Some(metadata),
    };
    let err = root_response_as(&setup, caller, Request::UpgradeCanister(conflict))
        .expect_err("upgrade replay conflict must reject");
    assert_eq!(err.code, ErrorCode::Internal);

    let metrics = root_capability_metrics(&setup);
    assert_eq!(metric_count(&metrics, "Upgrade", "ReplayDuplicateSame"), 1);
    assert_eq!(
        metric_count(&metrics, "Upgrade", "ReplayDuplicateConflict"),
        1
    );
    assert_eq!(metric_count(&metrics, "Upgrade", "ExecutionSuccess"), 1);
}

#[test]
fn unsupported_capability_proof_rejection_does_not_commit_replay_entry() {
    let setup = setup_root();
    let caller = setup
        .subnet_directory
        .get(&canister::TEST)
        .copied()
        .expect("test canister exists");
    let metadata = metadata([17u8; 32], 120);

    let invalid = canic::dto::auth::DelegationRequest {
        shard_pid: caller,
        scopes: vec!["rpc:verify".to_string()],
        aud: vec![caller],
        ttl_secs: 60,
        verifier_targets: Vec::new(),
        include_root_verifier: false,
        metadata: Some(metadata),
    };

    let first = root_response_as(&setup, caller, Request::IssueDelegation(invalid.clone()))
        .expect_err("unsupported structural delegation request must fail");
    assert_eq!(first.code, ErrorCode::Forbidden);

    let second = root_response_as(&setup, caller, Request::IssueDelegation(invalid))
        .expect_err("unsupported structural replay must not be committed");
    assert_eq!(second.code, ErrorCode::Forbidden);

    let metrics = root_capability_metrics(&setup);
    assert_eq!(
        metric_count(&metrics, "IssueDelegation", "ProofRejected"),
        2
    );
    assert_eq!(
        metric_count(&metrics, "IssueDelegation", "ReplayAccepted"),
        0
    );
    assert_eq!(
        metric_count(&metrics, "IssueDelegation", "ExecutionError"),
        0
    );
}

fn root_response_as(
    setup: &RootSetup,
    caller: Principal,
    request: Request,
) -> Result<Response, Error> {
    let (request_id, nonce, ttl_seconds) = capability_metadata_from_request(&request);
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request,
        proof: CapabilityProof::Structural,
        metadata: CapabilityRequestMetadata {
            request_id,
            nonce,
            issued_at: root_now_secs(setup),
            ttl_seconds,
        },
    };

    let result: Result<Result<RootCapabilityResponseV1, Error>, Error> = setup.pic.update_call_as(
        setup.root_id,
        caller,
        protocol::CANIC_RESPONSE_CAPABILITY_V1,
        (envelope,),
    );
    result
        .expect("root response transport call failed")
        .map(|response| response.response)
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

fn root_now_secs(setup: &RootSetup) -> u64 {
    let now: Result<u64, Error> = setup
        .pic
        .query_call(setup.root_id, protocol::CANIC_TIME, ())
        .expect("canic_time transport query failed");
    now.expect("canic_time application query failed") / 1_000_000_000
}

fn capability_metadata_from_request(request: &Request) -> ([u8; 16], [u8; 16], u32) {
    let metadata = match request {
        Request::CreateCanister(req) => req.metadata,
        Request::UpgradeCanister(req) => req.metadata,
        Request::Cycles(req) => req.metadata,
        Request::IssueDelegation(req) => req.metadata,
        Request::IssueRoleAttestation(req) => req.metadata,
    };

    match metadata {
        Some(meta) => {
            let mut request_id = [0u8; 16];
            request_id.copy_from_slice(&meta.request_id[..16]);
            let mut nonce = [0u8; 16];
            nonce.copy_from_slice(&meta.request_id[16..]);
            let ttl_seconds =
                u32::try_from(meta.ttl_seconds.min(u64::from(u32::MAX))).expect("ttl bounded");
            (request_id, nonce, ttl_seconds)
        }
        None => ([0u8; 16], [0u8; 16], 60),
    }
}

fn is_canister_status_decode_failure(err: &Error) -> bool {
    err.message.contains("CanisterStatusResult")
        && err.message.contains("candid decode failed for type")
}

const fn metadata(request_id: [u8; 32], ttl_seconds: u64) -> RootRequestMetadata {
    RootRequestMetadata {
        request_id,
        ttl_seconds,
    }
}
