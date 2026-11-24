//! Native byte and direction compatibility for application-declared contracts.
use canic::method::{Contract, LIMITS, wire};

#[canic::method_descriptor]
fn echo(text: String) -> String {
    text
}

#[test]
fn default_contract_accepts_a_full_frame_in_each_direction() {
    for limits in [echo::REQUEST, echo::REPLY] {
        assert_eq!(
            (limits.bytes, limits.allocation, limits.nodes, limits.depth),
            (16_384, 393_216, 4_096, 16)
        );
    }
    // The frame header takes 11 bytes and this string's length takes two.
    let mut text = "x".repeat(16_384 - 13);
    let request = echo::encode_request(&text).unwrap();
    let reply = echo::encode_reply(&echo::decode_request(&request).unwrap()).unwrap();
    assert_eq!(request.len(), 16_384);
    assert_eq!(reply.len(), 16_384);
    assert_eq!(echo::decode_reply(&reply).unwrap(), text);

    text.push('x');
    assert!(echo::encode_request(&text).is_err());
    assert!(echo::encode_reply(&text).is_err());
}

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

#[test]
fn borrowed_preflight_matches_request_encoding_on_the_host() {
    fn check<M: Contract<Input = String>>(text: &String) {
        let encoded = M::encode_request(text).map(|frame| frame.len());
        assert_eq!(M::request_len(text), encoded);
        let borrowed = M::request_len(text.as_str());
        if text.len() == 16_384 {
            // Wire meters the inline size of str during traversal. String holds a
            // fixed-size header, so its traversal reaches the frame byte check.
            assert_eq!(
                borrowed,
                Err(wire::Error::Limit(wire::LimitKind::Allocation))
            );
            assert_eq!(encoded, Err(wire::Error::Limit(wire::LimitKind::Bytes)));
        } else {
            assert_eq!(borrowed, encoded);
        }
    }

    for size in [1, 64, 16_384] {
        let text = "x".repeat(size);
        check::<echo>(&text);
        check::<Example>(&text);
    }
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

// The caller has neither the receiver's guard nor its implementation in scope.
#[canic::method_descriptor(
    name = "guarded_lookup",
    guard = require_parent,
    limits = Example::REQUEST,
    reply_limits = Example::REPLY,
)]
async fn lookup(key: String) -> String {
    receiver::lookup(key).await
}

#[test]
fn passive_descriptor_preserves_export_name_and_directional_limits() {
    assert_eq!(lookup::NAME, "guarded_lookup");
    assert_eq!(lookup::ID, wire::method_id("guarded_lookup"));
    let value = "a".repeat(96);
    assert!(lookup::encode_request(&value).is_err());
    assert_eq!(
        lookup::decode_reply(&lookup::encode_reply(&value).unwrap()).unwrap(),
        value
    );
}

#[test]
fn outbound_call_refuses_oversized_request_before_dispatch() {
    use canic::method::{CallError, call, call_unbounded};
    use ic_platform_runtime as runtime;
    use ic_platform_types::Principal;
    use std::{
        future::Future,
        pin::pin,
        task::{Context, Poll, Waker},
    };

    let input = "a".repeat(96);
    let target = Principal::management_canister();
    let handler = call::<Example>(target, 1, &input);
    assert!(matches!(
        pin!(handler).poll(&mut Context::from_waker(Waker::noop())),
        Poll::Ready(Err(CallError::Request(_)))
    ));

    assert!(matches!(
        pin!(call_unbounded::<lookup>(target, &input))
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Ready(Err(CallError::Request(_)))
    ));

    // The same public runtime import used by application guards type-checks here.
    let _: fn() -> Principal = runtime::system::caller;
}
