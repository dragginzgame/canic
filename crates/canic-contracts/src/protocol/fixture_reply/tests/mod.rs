//! Barriers retain separate method identities and typed, bounded native frames.
use super::*;
use crate::dto::fixture_reply::ReplyKind;

#[test]
fn arm_preserves_each_checkpoint_and_rejects_other_control_frames() {
    for kind in [ReplyKind::Grant, ReplyKind::Revoke, ReplyKind::Chunk] {
        let frame = ic_wire::native::encode_request(arm::METHOD_ID, &kind, LIMITS).unwrap();
        assert_eq!(
            ic_wire::native::decode_request::<ReplyKind>(arm::METHOD_ID, &frame, LIMITS).unwrap(),
            kind
        );
        assert!(matches!(
            ic_wire::native::decode_request::<()>(release::METHOD_ID, &frame, LIMITS),
            Err(ic_wire::Error::Identity)
        ));
        let reply = ic_wire::native::encode_response(arm::METHOD_ID, &true, LIMITS).unwrap();
        assert!(ic_wire::native::decode_response::<bool>(arm::METHOD_ID, &reply, LIMITS).unwrap());
    }
}

#[test]
fn unit_controls_keep_distinct_requests_and_response_shapes() {
    let release_frame = ic_wire::native::encode_request(release::METHOD_ID, &(), LIMITS).unwrap();
    ic_wire::native::decode_request::<()>(release::METHOD_ID, &release_frame, LIMITS).unwrap();
    let reply = ic_wire::native::encode_response(release::METHOD_ID, &(), LIMITS).unwrap();
    ic_wire::native::decode_response::<()>(release::METHOD_ID, &reply, LIMITS).unwrap();
    let mut waiting_frame =
        ic_wire::native::encode_request(waiting::METHOD_ID, &(), LIMITS).unwrap();
    ic_wire::native::decode_request::<()>(waiting::METHOD_ID, &waiting_frame, LIMITS).unwrap();
    assert!(matches!(
        ic_wire::native::decode_request::<()>(waiting::METHOD_ID, &release_frame, LIMITS),
        Err(ic_wire::Error::Identity)
    ));
    let reply = ic_wire::native::encode_response(waiting::METHOD_ID, &u32::MAX, LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_response::<u32>(waiting::METHOD_ID, &reply, LIMITS).unwrap(),
        u32::MAX
    );
    waiting_frame.push(0);
    assert!(matches!(
        ic_wire::native::decode_request::<()>(waiting::METHOD_ID, &waiting_frame, LIMITS),
        Err(ic_wire::Error::Trailing)
    ));
}
