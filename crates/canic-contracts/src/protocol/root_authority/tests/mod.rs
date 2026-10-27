//! Native authority requests cannot alias Store reads or accept malformed frames.
use super::*;
#[test]
fn authority_request_requires_exact_identity_and_complete_frame() {
    let frame = ic_wire::native::encode_request(METHOD_ID, &(), REQUEST_LIMITS).unwrap();
    ic_wire::native::decode_request::<()>(METHOD_ID, &frame, REQUEST_LIMITS).unwrap();
    assert!(matches!(
        ic_wire::native::decode_request::<()>(
            crate::protocol::store_status::authority::METHOD_ID,
            &frame,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
    let mut trailing = frame.clone();
    trailing.push(0);
    assert!(ic_wire::native::decode_request::<()>(METHOD_ID, &trailing, REQUEST_LIMITS).is_err());
    assert!(
        ic_wire::native::decode_request::<()>(METHOD_ID, &frame[..frame.len() - 1], REQUEST_LIMITS)
            .is_err()
    );
}
