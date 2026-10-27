//! Exact compiled metadata framing and fail-without-consuming admission.

use super::*;

const LIMITS: MethodLimits = MethodLimits {
    bytes: 128,
    allocation: 256,
    nodes: 16,
    depth: 8,
};
const METHOD: CompiledMethod<'_> = CompiledMethod {
    name: "echo",
    kind: MethodKind::CompositeQuery,
    id: 9001,
    request: LIMITS,
    response: LIMITS,
};
const BYTES: [u8; HEADER_BYTES + 4] = METHOD.encode();

#[test]
fn fixed_record_round_trip_and_concatenation() {
    assert_eq!(&BYTES[..4], &[1, 3, 4, 0]);
    assert_eq!(&BYTES[4..12], &9001_u64.to_le_bytes());
    assert_eq!(&BYTES[12..20], &128_u64.to_le_bytes());
    assert_eq!(&BYTES[HEADER_BYTES..], b"echo");
    let both = [BYTES, BYTES].concat();
    let mut cursor = both.as_slice();
    assert_eq!(CompiledMethod::decode(&mut cursor), Ok(METHOD));
    assert_eq!(CompiledMethod::decode(&mut cursor), Ok(METHOD));
    assert!(cursor.is_empty());
}

#[test]
fn malformed_record_does_not_advance_cursor() {
    for end in 0..BYTES.len() {
        let mut cursor = &BYTES[..end];
        assert_eq!(
            CompiledMethod::decode(&mut cursor),
            Err(DeclarationError::Framing)
        );
        assert_eq!(cursor, &BYTES[..end]);
    }
    for (offset, byte, error) in [
        (0, 2, DeclarationError::Version),
        (1, 9, DeclarationError::Kind),
        (HEADER_BYTES, 255, DeclarationError::Name),
        (HEADER_BYTES, 0, DeclarationError::Name),
    ] {
        let mut bytes = BYTES;
        bytes[offset] = byte;
        let mut cursor = bytes.as_slice();
        assert_eq!(CompiledMethod::decode(&mut cursor), Err(error));
        assert_eq!(cursor, bytes);
    }
}
