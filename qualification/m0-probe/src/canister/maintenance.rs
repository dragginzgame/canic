//! Bounded fixture checkpoint; no general storage engine or schema migration.

use super::{controller, creation, funding};
use crate::{EXECUTABLE, FUNDING_STATUS, LIMITS, STATUS, UPGRADE};
use ic_platform_runtime::cdk::{self, api, stable};

const FUNDING_OFFSET: u64 = LIMITS.bytes as u64 + 4;

#[cdk::pre_upgrade]
fn pre_upgrade() {
    let creation = ic_wire::native::encode_response(STATUS, &creation::status(), LIMITS)
        .expect("bounded creation checkpoint");
    let funding =
        ic_wire::native::encode_response(FUNDING_STATUS, &funding::funding_status(), LIMITS)
            .expect("bounded funding checkpoint");
    // One 64 KiB page holds two 4 KiB frames and their lengths. This fixture
    // exclusively owns stable memory; no application data shares this layout.
    if api::stable_size() == 0 {
        stable::stable_grow(1).expect("checkpoint page allocation");
    }
    write_frame(0, &creation);
    write_frame(FUNDING_OFFSET, &funding);
}

fn write_frame(offset: u64, bytes: &[u8]) {
    let length = u32::try_from(bytes.len()).expect("bounded checkpoint length");
    stable::stable_write(offset, &length.to_le_bytes());
    stable::stable_write(offset + 4, bytes);
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns argument bytes"
)]
fn decode_upgrade(bytes: Vec<u8>) -> bool {
    assert!(bytes.len() <= LIMITS.bytes, "upgrade argument too large");
    ic_wire::native::decode_request(UPGRADE, &bytes, LIMITS).expect("valid upgrade argument")
}

#[cdk::post_upgrade(decode_with = "decode_upgrade")]
fn post_upgrade(trap_after_restore: bool) {
    assert!(api::stable_size() > 0, "maintenance checkpoint missing");
    let creation = ic_wire::native::decode_response(STATUS, &read_frame(0), LIMITS)
        .expect("valid creation checkpoint");
    let funding =
        ic_wire::native::decode_response(FUNDING_STATUS, &read_frame(FUNDING_OFFSET), LIMITS)
            .expect("valid funding checkpoint");
    creation::restore(creation);
    funding::restore(funding);
    if trap_after_restore {
        // Check rollback of stable writes as well as the new executable/heap.
        stable::stable_write(0, &[0; 4]);
        api::trap("injected maintenance trap after restore");
    }
}

fn read_frame(offset: u64) -> Vec<u8> {
    let mut length = [0; 4];
    stable::stable_read(offset, &mut length);
    let length = u32::from_le_bytes(length) as usize;
    assert!(
        length > 0 && length <= LIMITS.bytes,
        "invalid checkpoint length"
    );
    let mut bytes = vec![0; length];
    stable::stable_read(offset + 4, &mut bytes);
    bytes
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns argument bytes"
)]
fn decode_executable(bytes: Vec<u8>) {
    ic_wire::native::decode_request::<()>(EXECUTABLE, &bytes, LIMITS)
        .expect("valid executable request");
}

fn encode_executable(value: bool) -> Vec<u8> {
    ic_wire::native::encode_response(EXECUTABLE, &value, LIMITS).expect("bounded executable reply")
}

#[cdk::query(
    guard = "controller",
    decode_with = "decode_executable",
    encode_with = "encode_executable"
)]
fn executable() -> bool {
    cfg!(feature = "maintenance-candidate")
}
