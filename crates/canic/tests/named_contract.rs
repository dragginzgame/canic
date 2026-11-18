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
    assert_eq!(Example::decode_request(&request).unwrap(), short);
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
