use candid::Nat;
use canic::{Error, ids::CanisterRole};
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
const QUALIFICATION_FIXTURE_WASM_SHA256: [u8; 32] = [
    0xe9, 0x6e, 0x05, 0x38, 0x2a, 0x8a, 0xcc, 0xfa, 0x13, 0xec, 0xf6, 0x7b, 0x24, 0xf9, 0xcb, 0x77,
    0x11, 0x17, 0xbc, 0xa7, 0x12, 0xa9, 0x39, 0x5b, 0xf7, 0xd8, 0x27, 0x9c, 0x06, 0x48, 0x42, 0x06,
];
const SNAPSHOT_RESTORE_MINIMUM_CYCLES: u128 = 10_000_000_000_000;

// Both cases observe only the restored target; the relay created by one case is unrelated state.
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

// Freeze the initialized predecessor-built workload used by the 0.106 Q3 protocol.
#[test]
fn estate_qualification_fixture_has_exact_initialized_memory_identity() {
    let fixture = install_standalone_canister(PROBE_CRATE, PROBE_ROLE, CanicWasmBuildProfile::Fast);
    let status = fixture
        .pocket_ic()
        .canister_status(fixture.canister_id(), None)
        .expect("query initialized qualification fixture");

    assert_eq!(
        status.module_hash.as_deref(),
        Some(QUALIFICATION_FIXTURE_WASM_SHA256.as_slice())
    );
    assert_eq!(status.memory_size, Nat::from(208_937_103_u64));
    assert_eq!(
        status.memory_metrics.wasm_memory_size,
        Nat::from(1_376_256_u64)
    );
    assert_eq!(
        status.memory_metrics.stable_memory_size,
        Nat::from(201_392_128_u64)
    );
    assert_eq!(status.memory_metrics.global_memory_size, Nat::from(64_u64));
    assert_eq!(
        status.memory_metrics.wasm_binary_size,
        Nat::from(3_010_225_u64)
    );
    assert_eq!(status.memory_metrics.custom_sections_size, Nat::from(0_u64));
    assert_eq!(
        status.memory_metrics.canister_history_size,
        Nat::from(414_u64)
    );
    assert_eq!(
        status.memory_metrics.wasm_chunk_store_size,
        Nat::from(3_145_728_u64)
    );
    assert_eq!(status.memory_metrics.snapshots_size, Nat::from(0_u64));
    eprintln!(
        "[estate-qualification-fixture] memory_size={} wasm_memory={} stable_memory={} global_memory={} wasm_binary={} custom_sections={} history={} chunk_store={} snapshots={}",
        status.memory_size,
        status.memory_metrics.wasm_memory_size,
        status.memory_metrics.stable_memory_size,
        status.memory_metrics.global_memory_size,
        status.memory_metrics.wasm_binary_size,
        status.memory_metrics.custom_sections_size,
        status.memory_metrics.canister_history_size,
        status.memory_metrics.wasm_chunk_store_size,
        status.memory_metrics.snapshots_size,
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
