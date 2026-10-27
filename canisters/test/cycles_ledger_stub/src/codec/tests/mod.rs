//! Native control replies preserve unit and lossless numeric evidence.

use crate::{codec, protocol};
use ic_platform::candid::contract::Nat;

#[test]
fn unit_control_reply_is_a_native_frame() {
    let bytes = codec::lose_next_transfer_reply::encode();
    ic_wire::native::decode_response::<()>(
        protocol::LOSE_NEXT_TRANSFER_REPLY,
        &bytes,
        protocol::LIMITS,
    )
    .unwrap();
}

#[test]
fn requested_amount_reply_preserves_zero_and_values_above_u128() {
    let mut wide = Nat::from(1_u8);
    wide.0 <<= 160_usize;
    let expected = vec![Nat::from(0_u8), Nat::from(u128::MAX), wide];
    let native = expected
        .iter()
        .map(canic_contracts::ops::unsigned_magnitude::from_nat)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let bytes = codec::requested_amounts::encode(native);
    let decoded: Vec<ic_wire::Bytes<{ protocol::MAGNITUDE_BYTES }>> =
        ic_wire::native::decode_response(protocol::REQUESTED_AMOUNTS, &bytes, protocol::LIMITS)
            .unwrap();
    assert!(decoded[0].as_slice().is_empty());
    let observed = decoded
        .iter()
        .map(canic_contracts::ops::unsigned_magnitude::to_nat)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(observed, expected);
}
