use canic::{Error, ids::CanisterRole};
use canic_testing_internal::pic::{CanicWasmBuildProfile, install_standalone_canister};
use ic_testkit::pic::{PicCallErrorKind, StandaloneCanisterFixture};

const PROBE_CRATE: &str = "payload_limit_probe";
const PROBE_ROLE: CanisterRole = CanisterRole::new("test");

// Verify generated inspect-message limits for default, explicit, and named updates.
#[test]
fn inspect_message_enforces_default_explicit_and_named_payload_limits() {
    let fixture = install_standalone_canister(PROBE_CRATE, PROBE_ROLE, CanicWasmBuildProfile::Fast);

    assert_echo_ok(&fixture, "default_echo", 12 * 1024);
    assert_rejected(&fixture, "default_echo", 20 * 1024);

    assert_echo_ok(&fixture, "explicit_echo", 20 * 1024);
    assert_rejected(&fixture, "explicit_echo", 36 * 1024);

    assert_echo_ok(&fixture, "wire_named_echo", 20 * 1024);
    assert_rejected(&fixture, "wire_named_echo", 28 * 1024);
}

// Assert one ingress update reaches the canister and returns the echoed length.
fn assert_echo_ok(fixture: &StandaloneCanisterFixture, method: &str, len: usize) {
    let payload = payload(len);
    let response: Result<usize, Error> = fixture.update_call_or_panic(method, (payload,));

    assert_eq!(response.expect("endpoint should accept payload"), len);
}

// Assert one ingress update is rejected before endpoint execution.
fn assert_rejected(fixture: &StandaloneCanisterFixture, method: &str, len: usize) {
    let payload = payload(len);
    let err = fixture
        .update_call::<Result<usize, Error>, _>(method, (payload,))
        .expect_err("transport should reject oversized ingress");

    assert_eq!(err.kind(), PicCallErrorKind::Transport);
}

// Build one ASCII string payload with exact byte length.
fn payload(len: usize) -> String {
    "x".repeat(len)
}
