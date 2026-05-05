use canic::{Error, ids::CanisterRole};
use canic_testkit::{artifacts::WasmBuildProfile, pic::install_standalone_canister};

const PROBE_CRATE: &str = "payload_limit_probe";
const PROBE_ROLE: CanisterRole = CanisterRole::new("test");

// Verify generated inspect-message limits for default, explicit, and named updates.
#[test]
fn inspect_message_enforces_default_explicit_and_named_payload_limits() {
    let fixture = install_standalone_canister(PROBE_CRATE, PROBE_ROLE, WasmBuildProfile::Fast);
    let pic = fixture.pic();
    let canister_id = fixture.canister_id();

    assert_echo_ok(pic, canister_id, "default_echo", 12 * 1024);
    assert_rejected(pic, canister_id, "default_echo", 20 * 1024);

    assert_echo_ok(pic, canister_id, "explicit_echo", 20 * 1024);
    assert_rejected(pic, canister_id, "explicit_echo", 36 * 1024);

    assert_echo_ok(pic, canister_id, "wire_named_echo", 20 * 1024);
    assert_rejected(pic, canister_id, "wire_named_echo", 28 * 1024);
}

// Assert one ingress update reaches the canister and returns the echoed length.
fn assert_echo_ok(
    pic: &canic_testkit::pic::Pic,
    canister_id: canic::cdk::types::Principal,
    method: &str,
    len: usize,
) {
    let payload = payload(len);
    let response: Result<usize, Error> = pic
        .update_call(canister_id, method, (payload,))
        .expect("transport should accept payload");

    assert_eq!(response.expect("endpoint should accept payload"), len);
}

// Assert one ingress update is rejected before endpoint execution.
fn assert_rejected(
    pic: &canic_testkit::pic::Pic,
    canister_id: canic::cdk::types::Principal,
    method: &str,
    len: usize,
) {
    let payload = payload(len);
    let err = pic
        .update_call::<Result<usize, Error>, _>(canister_id, method, (payload,))
        .expect_err("transport should reject oversized ingress");

    assert!(
        err.message.contains("pocket_ic update_call failed"),
        "unexpected rejection error: {err}"
    );
}

// Build one ASCII string payload with exact byte length.
fn payload(len: usize) -> String {
    "x".repeat(len)
}
