//! Shared Wire codecs and pre-decode admission; exports are mounted by each canister.
#[cfg(feature = "fault-injection")]
use canic_management_protocol::INJECT;
use canic_management_protocol::{
    LIMITS, RECONCILE, Refusal, STAGE, STAGE_LIMITS, STATUS, Stage, Status,
};
use ic_platform_runtime::cdk::api;
fn guard(limit: usize, private: bool) -> Result<(), String> {
    if private && !api::is_controller(&api::msg_caller()) {
        return Err("controller required".into());
    }
    if ic_platform_runtime::ic0::msg_arg_data_size() > limit {
        return Err("growth frame too large".into());
    }
    Ok(())
}
/// Bound stage input and require a current controller before decoding.
///
/// # Errors
/// Refuses oversized frames or missing controller authority.
pub fn uploader() -> Result<(), String> {
    guard(STAGE_LIMITS.bytes, true)
}
/// Bound control input and require a current controller before decoding.
///
/// # Errors
/// Refuses oversized frames or missing controller authority.
pub fn controller() -> Result<(), String> {
    guard(LIMITS.bytes, true)
}
/// Bound public status input before decoding.
///
/// # Errors
/// Refuses oversized frames or missing controller authority.
pub fn public() -> Result<(), String> {
    guard(LIMITS.bytes, false)
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns decoder bytes"
)]
/// Decode the bounded staging frame.
///
/// # Panics
/// Traps if the frame or result cannot satisfy the protocol bounds.
#[must_use]
pub fn decode_stage(bytes: Vec<u8>) -> Stage {
    ic_wire::native::decode_request(STAGE, &bytes, STAGE_LIMITS).expect("bounded stage")
}
#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns reply")]
/// Encode admission evidence.
///
/// # Panics
/// Traps if the frame or result cannot satisfy the protocol bounds.
#[must_use]
pub fn encode_stage(value: Result<Status, Refusal>) -> Vec<u8> {
    ic_wire::native::encode_response(STAGE, &value, LIMITS).expect("bounded status")
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns decoder bytes"
)]
/// Decode a status request.
///
/// # Panics
/// Traps if the frame or result cannot satisfy the protocol bounds.
pub fn decode_status(bytes: Vec<u8>) {
    ic_wire::native::decode_request::<()>(STATUS, &bytes, LIMITS).expect("status frame");
}
#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns reply")]
/// Encode status evidence.
///
/// # Panics
/// Traps if the frame or result cannot satisfy the protocol bounds.
#[must_use]
pub fn encode_status(value: Status) -> Vec<u8> {
    ic_wire::native::encode_response(STATUS, &value, LIMITS).expect("bounded status")
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns decoder bytes"
)]
/// Decode a reconciliation request.
///
/// # Panics
/// Traps if the frame or result cannot satisfy the protocol bounds.
pub fn decode_reconcile(bytes: Vec<u8>) {
    ic_wire::native::decode_request::<()>(RECONCILE, &bytes, LIMITS).expect("reconcile frame");
}
#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns reply")]
/// Encode reconciled evidence.
///
/// # Panics
/// Traps if the frame or result cannot satisfy the protocol bounds.
#[must_use]
pub fn encode_reconcile(value: Status) -> Vec<u8> {
    ic_wire::native::encode_response(RECONCILE, &value, LIMITS).expect("bounded status")
}

#[cfg(feature = "fault-injection")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns decoder bytes"
)]
/// Decode a test-only fault request.
///
/// # Panics
/// Traps if the frame or result cannot satisfy the protocol bounds.
#[must_use]
pub fn decode_inject(bytes: Vec<u8>) -> u8 {
    ic_wire::native::decode_request(INJECT, &bytes, LIMITS).expect("fault frame")
}
#[cfg(feature = "fault-injection")]
/// Encode the test-only fault acknowledgement.
///
/// # Panics
/// Traps if the frame or result cannot satisfy the protocol bounds.
#[must_use]
pub fn encode_inject() -> Vec<u8> {
    ic_wire::native::encode_response(INJECT, &(), LIMITS).expect("fault reply")
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns decoder bytes"
)]
/// Decode a bounded child upgrade envelope.
/// # Panics
/// Traps if the request exceeds its protocol contract.
#[must_use]
pub fn decode_upgrade(bytes: Vec<u8>) -> canic_management_protocol::upgrade::Stage {
    ic_wire::native::decode_request(
        canic_management_protocol::upgrade::STAGE,
        &bytes,
        STAGE_LIMITS,
    )
    .expect("upgrade frame")
}
#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns reply")]
/// Encode upgrade admission or refusal.
/// # Panics
/// Traps if evidence exceeds the protocol bounds.
#[must_use]
pub fn encode_upgrade(
    value: Result<canic_management_protocol::upgrade::Status, Refusal>,
) -> Vec<u8> {
    ic_wire::native::encode_response(canic_management_protocol::upgrade::STAGE, &value, LIMITS)
        .expect("upgrade reply")
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns decoder bytes"
)]
/// Decode an upgrade status request.
/// # Panics
/// Traps if the request exceeds its protocol contract.
pub fn decode_upgrade_status(bytes: Vec<u8>) {
    ic_wire::native::decode_request::<()>(
        canic_management_protocol::upgrade::STATUS,
        &bytes,
        LIMITS,
    )
    .expect("upgrade status frame");
}
#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns reply")]
/// Encode committed upgrade evidence.
/// # Panics
/// Traps if evidence exceeds protocol bounds.
#[must_use]
pub fn encode_upgrade_status(value: canic_management_protocol::upgrade::Status) -> Vec<u8> {
    ic_wire::native::encode_response(canic_management_protocol::upgrade::STATUS, &value, LIMITS)
        .expect("upgrade status reply")
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns decoder bytes"
)]
/// Decode an upgrade reconciliation request.
/// # Panics
/// Traps if the request exceeds its protocol contract.
pub fn decode_upgrade_reconcile(bytes: Vec<u8>) {
    ic_wire::native::decode_request::<()>(
        canic_management_protocol::upgrade::RECONCILE,
        &bytes,
        LIMITS,
    )
    .expect("upgrade reconciliation frame");
}
#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns reply")]
/// Encode reconciled upgrade evidence.
/// # Panics
/// Traps if evidence exceeds protocol bounds.
#[must_use]
pub fn encode_upgrade_reconcile(value: canic_management_protocol::upgrade::Status) -> Vec<u8> {
    ic_wire::native::encode_response(
        canic_management_protocol::upgrade::RECONCILE,
        &value,
        LIMITS,
    )
    .expect("upgrade reconciliation reply")
}
