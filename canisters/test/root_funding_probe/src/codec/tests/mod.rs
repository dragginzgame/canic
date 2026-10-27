//! Installation/provider codecs and owned grant frames remain distinct.

use super::*;
use crate::{GrantRequest, Principal, ProbeInit, protocol};
use ic_platform::candid::contract::encode_args;

#[test]
fn external_codecs_preserve_installation_and_provider_contracts() {
    let init = ProbeInit::Root {
        coordinator: Principal::anonymous(),
    };
    assert_eq!(decode_one::<ProbeInit>(encode_one(&init).unwrap()), init);
    let (): () = decode_args(encode_args(()).unwrap());
    assert_eq!(
        encode_reply(10_000_u64),
        encode_args((10_000_u64,)).unwrap()
    );
}

#[test]
fn native_grant_frames_bind_the_exact_method_and_direction() {
    let request = GrantRequest {
        operation_id: [7; 32],
        amount: u128::MAX,
    };
    let bytes =
        ic_wire::native::encode_request(protocol::ACCEPT, &request, protocol::LIMITS).unwrap();
    assert_eq!(accept::decode(bytes.clone()), request);
    assert_eq!(
        ic_wire::native::decode_request::<GrantRequest>(
            protocol::ACCEPT_THEN_TRAP,
            &bytes,
            protocol::LIMITS
        ),
        Err(ic_wire::Error::Identity)
    );
    assert_eq!(
        ic_wire::native::decode_response::<GrantRequest>(
            protocol::ACCEPT,
            &bytes,
            protocol::LIMITS
        ),
        Err(ic_wire::Error::Identity)
    );
    let reply = crate::RootAcceptResult::CallerDenied;
    let bytes = accept::encode(reply.clone());
    assert_eq!(
        ic_wire::native::decode_response::<crate::RootAcceptResult>(
            protocol::ACCEPT,
            &bytes,
            protocol::LIMITS
        )
        .unwrap(),
        reply
    );
}

#[test]
fn native_prepare_preserves_target_and_maximum_amount() {
    let request = protocol::PrepareRequest {
        root: ic_platform::Principal::self_authenticating([7]),
        operation_id: [9; 32],
        amount: u128::MAX,
    };
    let bytes =
        ic_wire::native::encode_request(protocol::PREPARE, &request, protocol::LIMITS).unwrap();
    assert_eq!(prepare::decode(bytes), request);
}
