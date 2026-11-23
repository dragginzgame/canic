use super::*;

#[test]
fn status_requires_a_complete_v2_response() {
    let response = protocol::Response {
        schema: [7; 32],
        admission: protocol::Admission::Maintenance,
    };
    let frame = |method, direction| {
        ic_wire::encode(&response, Encoding::Frame { method, direction }, RPC_LIMITS).unwrap()
    };
    let bytes = frame(protocol::INSPECT, Direction::Response);
    assert_eq!(decode(&bytes).unwrap(), response);
    assert!(render(&response).contains("Maintenance"));
    assert!(
        decode(&frame(
            ic_wire::method_id("canic.db.inspect.v1"),
            Direction::Response
        ))
        .is_err()
    );
    assert!(decode(&frame(protocol::INSPECT, Direction::Request)).is_err());
    assert!(decode(&bytes[..bytes.len() - 1]).is_err());
    assert!(decode(&vec![0; RPC_LIMITS.bytes + 1]).is_err());
}
