use candid::{CandidType, Deserialize, Principal, encode_args, encode_one};
use canic::{Error, ids::CanisterRole};
use canic_host::candid_endpoints::{
    EndpointType, IngressPayloadBasis, parse_candid_service_endpoints,
};
use canic_testing_internal::pic::{
    CanicWasmBuildProfile, install_standalone_canister, install_standalone_canister_on_pic,
    standalone_canister_wasm,
};
use ic_testkit::pic::{
    CachedStandaloneCanisterFixtureGuard, CachedStandaloneCanisterFixturePool, CandidCallErrorKind,
    CandidCallExt, CanisterInstallExt, ErrorCode, RejectCode, RetryPolicy, SnapshotRestoreFunding,
    StandaloneCanisterFixture,
};

const PROBE_CRATE: &str = "payload_limit_probe";
const PROBE_ROLE: CanisterRole = CanisterRole::new("test");
const EXPLICIT_ECHO_MAX_BYTES: usize = 32 * 1024;
const SNAPSHOT_RESTORE_MINIMUM_CYCLES: u128 = 10_000_000_000_000;

#[derive(CandidType, Debug, Deserialize, PartialEq)]
struct PlainReply {
    committed: u64,
    predicates: u64,
}

fn counts(fixture: &StandaloneCanisterFixture) -> PlainReply {
    fixture.query_candid_or_panic("dispatch_counts", ())
}

fn denied_count(fixture: &StandaloneCanisterFixture, method: &str) -> u64 {
    let metrics: Result<Vec<canic::dto::metrics::MetricEntry>, Error> =
        fixture.query_candid_or_panic("access_counts", ());
    metrics
        .unwrap()
        .iter()
        .filter(|entry| {
            entry.labels.first().is_some_and(|label| label == "access")
                && entry.labels.get(1).is_some_and(|label| label == method)
        })
        .map(|entry| match entry.value {
            canic::dto::metrics::MetricValue::Count(count) => count,
            _ => panic!("access metrics must count denials"),
        })
        .sum()
}

#[test]
fn guarded_plain_reply_and_result_reply_keep_denial_metrics_and_short_circuiting() {
    let fixture = acquire_probe_fixture();
    assert_eq!(
        counts(&fixture),
        PlainReply {
            committed: 0,
            predicates: 0
        }
    );
    let wire = fixture
        .pocket_ic()
        .update_call(
            fixture.canister_id(),
            Principal::anonymous(),
            "plain_commit",
            encode_args((true,)).unwrap(),
        )
        .unwrap();
    assert_eq!(
        wire,
        encode_one(PlainReply {
            committed: 1,
            predicates: 2
        })
        .unwrap()
    );
    assert_eq!(denied_count(&fixture, "plain_commit"), 0);
    let refusal = fixture
        .pocket_ic()
        .update_call(
            fixture.canister_id(),
            Principal::anonymous(),
            "plain_commit",
            encode_args((false,)).unwrap(),
        )
        .unwrap_err();
    assert_eq!(refusal.reject_code, RejectCode::CanisterReject);
    assert_eq!(
        counts(&fixture),
        PlainReply {
            committed: 1,
            predicates: 3
        }
    );
    assert_eq!(denied_count(&fixture, "plain_commit"), 1);

    let accepted: Result<PlainReply, Error> =
        fixture.update_candid_or_panic("result_commit", (true,));
    assert_eq!(
        accepted.unwrap(),
        PlainReply {
            committed: 2,
            predicates: 5
        }
    );
    assert_eq!(denied_count(&fixture, "result_commit"), 0);
    let denied: Result<PlainReply, Error> =
        fixture.update_candid_or_panic("result_commit", (false,));
    assert!(denied.is_err());
    assert_eq!(
        counts(&fixture),
        PlainReply {
            committed: 2,
            predicates: 6
        }
    );
    assert_eq!(denied_count(&fixture, "result_commit"), 1);
    drop(fixture);
}

fn handler_log_count(fixture: &StandaloneCanisterFixture) -> usize {
    fixture
        .pocket_ic()
        .fetch_canister_logs(fixture.canister_id(), Principal::anonymous())
        .unwrap()
        .iter()
        .filter(|record| record.content == b"bounded handler dispatched")
        .count()
}

#[test]
fn bounded_queries_and_inter_canister_updates_refuse_before_dispatch() {
    let fixture = acquire_probe_fixture();
    let relay = install_standalone_canister_on_pic(
        fixture.pocket_ic(),
        PROBE_CRATE,
        PROBE_ROLE,
        CanicWasmBuildProfile::Fast,
        "bounded-relay",
    );
    let query: PlainReply = fixture.query_candid_or_panic("bounded_query", ("valid",));
    assert_eq!(query.committed, 1);
    let before = handler_log_count(&fixture);
    for bytes in [
        encode_args(("x".repeat(1024),)).unwrap(),
        b"invalid".to_vec(),
    ] {
        let error = fixture
            .pocket_ic()
            .query_call(
                fixture.canister_id(),
                Principal::anonymous(),
                "bounded_query",
                bytes,
            )
            .unwrap_err();
        assert_eq!(error.reject_code, RejectCode::CanisterError, "{error:?}");
        assert_eq!(error.error_code, ErrorCode::CanisterCalledTrap);
    }
    assert_eq!(handler_log_count(&fixture), before);
    let accepted: Result<bool, Error> = fixture.pocket_ic().update_candid_or_panic(
        relay,
        "relay_bounded",
        (
            fixture.canister_id(),
            "bounded_update",
            encode_args(("valid",)).unwrap(),
        ),
    );
    assert!(accepted.unwrap());
    for (method, bytes) in [
        ("bounded_work", encode_args((Vec::<()>::new(),)).unwrap()),
        ("bounded_skip", encode_args(()).unwrap()),
        ("bounded_types", encode_args((Vec::<u8>::new(),)).unwrap()),
        ("bounded_header", encode_args((0_u64,)).unwrap()),
    ] {
        let accepted: Result<bool, Error> = fixture.pocket_ic().update_candid_or_panic(
            relay,
            "relay_bounded",
            (fixture.canister_id(), method, bytes),
        );
        assert!(
            accepted.unwrap(),
            "{method} accepts its ordinary bounded input"
        );
    }
    let initial = counts(&fixture);
    let before = handler_log_count(&fixture);
    assert!(
        before > 0,
        "successful updates must emit the dispatch witness"
    );
    for (method, bytes) in [
        ("bounded_update", encode_args(("x".repeat(1024),)).unwrap()),
        ("bounded_update", b"invalid".to_vec()),
        ("bounded_work", encode_args((vec![(); 100],)).unwrap()),
        ("bounded_skip", encode_args((vec![(); 40],)).unwrap()),
        (
            "bounded_types",
            encode_args((Vec::<u8>::new(), Vec::<Vec<u8>>::new())).unwrap(),
        ),
        (
            "bounded_header",
            encode_args((0_u64, Vec::<Vec<u8>>::new())).unwrap(),
        ),
    ] {
        let refused: Result<bool, Error> = fixture.pocket_ic().update_candid_or_panic(
            relay,
            "relay_bounded",
            (fixture.canister_id(), method, bytes),
        );
        assert!(refused.is_err(), "{method} must refuse the bounded input");
        assert_eq!(counts(&fixture), initial);
        assert_eq!(
            handler_log_count(&fixture),
            before,
            "{method} must not enter its handler even before rollback"
        );
    }
    drop(fixture);
}

fn participant_log_count(fixture: &StandaloneCanisterFixture) -> usize {
    fixture
        .pocket_ic()
        .fetch_canister_logs(fixture.canister_id(), Principal::anonymous())
        .unwrap()
        .iter()
        .filter(|record| record.content.starts_with(b"bounded participant "))
        .count()
}

#[test]
fn lifecycle_bounds_run_before_init_and_post_upgrade_participants() {
    let fixture = acquire_probe_fixture();
    let wasm = standalone_canister_wasm(PROBE_CRATE, CanicWasmBuildProfile::Fast);
    let bad_inputs = [
        encode_args((Some(vec![0_u8; 4096]),)).unwrap(),
        b"invalid".to_vec(),
        encode_args((None::<Vec<u8>>, vec![(); 1000])).unwrap(),
        b"DIDL\x01\x6c\xff\xff\xff\xff\x0f".to_vec(),
    ];
    let before = participant_log_count(&fixture);
    assert!(
        before > 0,
        "successful init must emit the participant witness"
    );
    let retry = RetryPolicy::try_new(4, std::time::Duration::from_mins(5)).unwrap();
    let witness = fixture
        .pocket_ic()
        .retry_install_code(retry, || {
            fixture.pocket_ic().reinstall_canister(
                fixture.canister_id(),
                wasm.clone(),
                encode_args((Some(vec![1_u8]),)).unwrap(),
                None,
            )
        })
        .unwrap_err();
    assert_eq!(witness.error_code, ErrorCode::CanisterCalledTrap);
    assert!(
        fixture
            .pocket_ic()
            .fetch_canister_logs(fixture.canister_id(), Principal::anonymous())
            .unwrap()
            .iter()
            .any(|record| record.content == b"bounded participant trap witness")
    );
    for bytes in bad_inputs {
        fixture
            .pocket_ic()
            .wait_out_install_code_rate_limit(std::time::Duration::from_mins(5));
        let error = fixture
            .pocket_ic()
            .retry_install_code(retry, || {
                fixture.pocket_ic().reinstall_canister(
                    fixture.canister_id(),
                    wasm.clone(),
                    bytes.clone(),
                    None,
                )
            })
            .unwrap_err();
        assert_eq!(error.reject_code, RejectCode::CanisterError, "{error:?}");
        assert_eq!(error.error_code, ErrorCode::CanisterCalledTrap);
        // Reinstall clears old logs. A participant reached in this failed
        // execution would still leave a witness, as proved above.
        assert_eq!(participant_log_count(&fixture), 0);
        fixture
            .pocket_ic()
            .wait_out_install_code_rate_limit(std::time::Duration::from_mins(5));
        let error = fixture
            .pocket_ic()
            .retry_install_code(retry, || {
                fixture.pocket_ic().upgrade_canister(
                    fixture.canister_id(),
                    wasm.clone(),
                    bytes.clone(),
                    None,
                )
            })
            .unwrap_err();
        assert_eq!(error.reject_code, RejectCode::CanisterError, "{error:?}");
        assert_eq!(error.error_code, ErrorCode::CanisterCalledTrap);
        assert_eq!(participant_log_count(&fixture), 0);
    }
    fixture
        .pocket_ic()
        .wait_out_install_code_rate_limit(std::time::Duration::from_mins(5));
    fixture
        .pocket_ic()
        .retry_install_code(retry, || {
            fixture.pocket_ic().upgrade_canister(
                fixture.canister_id(),
                wasm.clone(),
                encode_args(()).unwrap(),
                None,
            )
        })
        .unwrap();
    assert_eq!(participant_log_count(&fixture), 1);
    drop(fixture);
}

// Cases observe only the restored target; the relay created by one case is unrelated state.
static PROBE_FIXTURES: CachedStandaloneCanisterFixturePool<1> =
    CachedStandaloneCanisterFixturePool::<1>::new(|| {
        install_standalone_canister(PROBE_CRATE, PROBE_ROLE, CanicWasmBuildProfile::Fast)
    })
    .with_restore_funding(SnapshotRestoreFunding::TopUpTo {
        minimum_cycles: SNAPSHOT_RESTORE_MINIMUM_CYCLES,
    });

// Verify generated inspect-message limits for default, explicit, and named updates.
#[test]
fn inspect_message_enforces_default_explicit_and_named_payload_limits() {
    let fixture = acquire_probe_fixture();

    assert_echo_ok(&fixture, "default_echo", 12 * 1024);
    assert_rejected(&fixture, "default_echo", 20 * 1024);

    assert_echo_ok(&fixture, "explicit_echo", 20 * 1024);
    assert_rejected(&fixture, "explicit_echo", 36 * 1024);

    assert_echo_ok(&fixture, "wire_named_echo", 20 * 1024);
    assert_rejected(&fixture, "wire_named_echo", 28 * 1024);
}

// Verify the raw generated adapter enforces the same bound when
// canister_inspect_message is not part of the call path.
#[test]
fn raw_update_adapter_rejects_oversized_inter_canister_payload_before_decode() {
    let target = acquire_probe_fixture();
    let relay = install_standalone_canister_on_pic(
        target.pocket_ic(),
        PROBE_CRATE,
        PROBE_ROLE,
        CanicWasmBuildProfile::Fast,
        "payload-limit-relay",
    );
    let exact_payload_len = string_len_for_wire_size(EXPLICIT_ECHO_MAX_BYTES);

    let accepted: Result<usize, Error> = target.pocket_ic().update_candid_or_panic(
        relay,
        "relay_explicit_echo",
        (target.canister_id(), exact_payload_len),
    );
    assert_eq!(
        accepted.expect("exact-boundary inter-canister payload"),
        exact_payload_len
    );

    let rejected: Result<usize, Error> = target.pocket_ic().update_candid_or_panic(
        relay,
        "relay_explicit_echo",
        (target.canister_id(), exact_payload_len + 1),
    );
    drop(target);
    assert!(
        rejected.is_err(),
        "oversized inter-canister payload must be rejected by the target"
    );
}

fn acquire_probe_fixture() -> CachedStandaloneCanisterFixtureGuard<'static> {
    let (fixture, outcome) = PROBE_FIXTURES
        .acquire()
        .expect("acquire payload-limit probe fixture");
    eprintln!("[payload-limit-probe] cached standalone fixture {outcome}");
    fixture
}

// Assert one ingress update reaches the canister and returns the echoed length.
fn assert_echo_ok(fixture: &StandaloneCanisterFixture, method: &str, len: usize) {
    let payload = payload(len);
    let response: Result<usize, Error> = fixture.update_candid_or_panic(method, (payload,));

    assert_eq!(response.expect("endpoint should accept payload"), len);
}

// Assert one ingress update is rejected before endpoint execution.
fn assert_rejected(fixture: &StandaloneCanisterFixture, method: &str, len: usize) {
    let payload = payload(len);
    let err = fixture
        .update_candid::<Result<usize, Error>, _>(method, (payload,))
        .expect_err("transport should reject oversized ingress");

    assert_eq!(err.kind(), CandidCallErrorKind::CanisterReject);
    assert!(err.reject_response().is_some());
}

// Build one ASCII string payload with exact byte length.
fn payload(len: usize) -> String {
    "x".repeat(len)
}

// Resolve the String length whose single-argument Candid encoding has one exact wire size.
fn string_len_for_wire_size(wire_size: usize) -> usize {
    (wire_size.saturating_sub(32)..=wire_size)
        .find(|len| {
            candid::encode_args((payload(*len),))
                .expect("encode probe payload")
                .len()
                == wire_size
        })
        .expect("one nearby String length must encode to the requested wire size")
}

#[test]
fn compiled_payload_contract_matches_actual_ingress_boundaries() {
    let candid = canic_testing_internal::pic::standalone_canister_candid(
        PROBE_CRATE,
        CanicWasmBuildProfile::Fast,
    );
    let endpoints = parse_candid_service_endpoints(&candid).unwrap();
    let plain = endpoints
        .iter()
        .find(|endpoint| endpoint.name == "plain_commit")
        .unwrap();
    assert_eq!(plain.returns.len(), 1);
    assert!(matches!(
        resolve_type(&plain.returns[0]),
        EndpointType::Record { .. }
    ));
    let result = endpoints
        .iter()
        .find(|endpoint| endpoint.name == "result_commit")
        .unwrap();
    assert!(matches!(
        resolve_type(&result.returns[0]),
        EndpointType::Variant { .. }
    ));
    let limit = plain.payload_limits.as_ref().unwrap();
    assert_eq!(limit.ingress_max_bytes, Some(1024));
    assert_eq!(limit.update_guard_max_bytes, Some(1024));
    let fixture = acquire_probe_fixture();
    for (method, limit, explicit) in [
        ("default_echo", 16 * 1024, false),
        ("explicit_echo", 32 * 1024, true),
        ("wire_named_echo", 24 * 1024, true),
        ("bare_echo", 16 * 1024, false),
    ] {
        let declaration = endpoints
            .iter()
            .find(|endpoint| endpoint.name == method)
            .unwrap();
        let limits = declaration.payload_limits.as_ref().unwrap();
        assert_eq!(limits.ingress_max_bytes, Some(limit as u64));
        assert_eq!(
            limits.update_guard_max_bytes,
            explicit.then_some(limit as u64)
        );
        assert_eq!(
            limits.ingress_basis,
            if explicit {
                IngressPayloadBasis::ExplicitOverride
            } else {
                IngressPayloadBasis::ManagedDefault
            }
        );
        let length = string_len_for_wire_size(limit);
        if method == "bare_echo" {
            let result: usize = fixture.update_candid_or_panic(method, (payload(length),));
            assert_eq!(result, length);
        } else {
            assert_echo_ok(&fixture, method, length);
        }
        assert_rejected(&fixture, method, length + 1);
    }
    drop(fixture);
}

fn resolve_type(ty: &EndpointType) -> &EndpointType {
    match ty {
        EndpointType::Named {
            resolved: Some(inner),
            ..
        } => resolve_type(inner),
        _ => ty,
    }
}
