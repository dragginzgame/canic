//! Byte-shape coverage for the funding probe's explicit Candid hooks.

use super::*;
use crate::{GrantRequest, Principal, ProbeInit};
use ic_platform::candid::contract::encode_args;

#[test]
fn codecs_preserve_installation_unit_scalar_and_multi_argument_contracts() {
    let init = ProbeInit::Root {
        coordinator: Principal::anonymous(),
    };
    assert_eq!(decode_one::<ProbeInit>(encode_one(&init).unwrap()), init);
    let (): () = decode_args(encode_args(()).unwrap());

    let request = GrantRequest {
        operation_id: [7; 32],
        amount: 123,
    };
    assert_eq!(
        decode_one::<GrantRequest>(encode_one(&request).unwrap()),
        request
    );
    let args = (Principal::anonymous(), [7_u8; 32], 123_u128);
    assert_eq!(
        decode_args::<(Principal, [u8; 32], u128)>(encode_args(args).unwrap()),
        args
    );
    assert_eq!(
        encode_reply(Some(request.clone())),
        encode_args((Some(request),)).unwrap()
    );
}
