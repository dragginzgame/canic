use canic::{Error, ids::CanisterRole};
use canic_host::candid_endpoints::{IngressPayloadBasis, parse_candid_service_endpoints};
use canic_testing_internal::pic::{
    CanicWasmBuildProfile, install_standalone_canister, install_standalone_canister_on_pic,
};
use ic_testkit::pic::{
    CachedStandaloneCanisterFixtureGuard, CachedStandaloneCanisterFixturePool, CandidCallErrorKind,
    CandidCallExt, SnapshotRestoreFunding, StandaloneCanisterFixture,
};

const PROBE_CRATE: &str = "payload_limit_probe";
const PROBE_ROLE: CanisterRole = CanisterRole::new("test");
const EXPLICIT_ECHO_MAX_BYTES: usize = 32 * 1024;
const SNAPSHOT_RESTORE_MINIMUM_CYCLES: u128 = 10_000_000_000_000;

// Cases observe only the restored target; the relay created by one case is unrelated state.
static PROBE_FIXTURES: CachedStandaloneCanisterFixturePool<1> =
    CachedStandaloneCanisterFixturePool::new().with_restore_funding(
        SnapshotRestoreFunding::TopUpTo {
            minimum_cycles: SNAPSHOT_RESTORE_MINIMUM_CYCLES,
        },
    );

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
        .acquire(|| {
            install_standalone_canister(PROBE_CRATE, PROBE_ROLE, CanicWasmBuildProfile::Fast)
        })
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
