//! Native byte and direction compatibility for application-declared contracts.
use canic::method::{Contract, LIMITS, wire};

struct Example;
impl Contract for Example {
    const NAME: &'static str = "provision_user";
    const ID: u64 = 0x544f_4b4f_0001;
    const REQUEST: wire::Limits = wire::Limits {
        bytes: 64,
        ..LIMITS
    };
    const REPLY: wire::Limits = wire::Limits {
        bytes: 256,
        ..LIMITS
    };
    type Input = String;
    type Output = String;
}

/// Return the submitted name using the application's published contract.
#[canic::update(contract = Example)]
fn provision_user(name: String) -> String {
    name
}

#[test]
fn named_frames_follow_each_direction_and_budget() {
    let short = "name".to_owned();
    let long = "a".repeat(96);
    let request = Example::encode_request(&short).unwrap();
    let reply = Example::encode_reply(&long).unwrap();
    let frame = |method, direction| wire::Encoding::Frame { method, direction };
    assert_eq!(
        request,
        wire::encode(
            &short,
            frame(Example::ID, wire::Direction::Request),
            Example::REQUEST
        )
        .unwrap()
    );
    assert_eq!(
        reply,
        wire::encode(
            &long,
            frame(Example::ID, wire::Direction::Response),
            Example::REPLY
        )
        .unwrap()
    );
    assert_eq!(
        provision_user(Example::decode_request(&request).unwrap()),
        short
    );
    assert_eq!(Example::decode_reply(&reply).unwrap(), long);
    assert!(Example::encode_request(&long).is_err());
    let too_large = "b".repeat(300);
    assert!(Example::encode_reply(&too_large).is_err());
    for (direction, decode, oversized) in [
        (
            wire::Direction::Request,
            Example::decode_request as fn(&[u8]) -> Result<String, wire::Error>,
            &long,
        ),
        (
            wire::Direction::Response,
            Example::decode_reply as fn(&[u8]) -> Result<String, wire::Error>,
            &too_large,
        ),
    ] {
        let wrong = wire::encode(&short, frame(Example::ID + 1, direction), LIMITS).unwrap();
        assert!(decode(&wrong).is_err());
        let oversized = wire::encode(oversized, frame(Example::ID, direction), LIMITS).unwrap();
        assert!(decode(&oversized).is_err());
    }
    assert!(Example::decode_request(&Example::encode_reply(&short).unwrap()).is_err());
    assert!(Example::decode_reply(&request).is_err());
    assert!(Example::decode_request(&request[..request.len() - 1]).is_err());
    assert!(Example::decode_reply(&reply[..reply.len() - 1]).is_err());
}

#[test]
fn outbound_call_refuses_oversized_request_before_dispatch() {
    use canic::method::{CallError, call, runtime};
    use ic_platform_types::Principal;
    use std::{
        future::Future,
        pin::pin,
        task::{Context, Poll, Waker},
    };

    let input = "a".repeat(96);
    let handler = async { call::<Example>(Principal::management_canister(), 1, &input).await };
    assert!(matches!(
        pin!(handler).poll(&mut Context::from_waker(Waker::noop())),
        Poll::Ready(Err(CallError::Request(_)))
    ));

    // The same public runtime import used by application guards type-checks here.
    let _: fn() -> Principal = runtime::system::caller;
}
