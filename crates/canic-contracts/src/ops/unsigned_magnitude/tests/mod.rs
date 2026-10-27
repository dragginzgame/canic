//! Numeric canonicality is separate from opaque-byte codec admission.
use super::*;

#[test]
fn preserves_zero_overflow_and_maximum_admitted_magnitudes() {
    for value in [
        Nat::from(0_u8),
        Nat::from(1_u8),
        Nat::from(u128::MAX),
        Nat(BigUint::from(1_u8) << 128),
        Nat(BigUint::from(1_u8) << 256),
        Nat((BigUint::from(1_u8) << 512) - BigUint::from(1_u8)),
    ] {
        let magnitude = from_nat::<64>(&value).unwrap();
        assert_eq!(to_nat(&magnitude).unwrap(), value);
        assert_ne!(magnitude.as_slice().first(), Some(&0));
    }
    assert!(
        from_nat::<64>(&Nat::from(0_u8))
            .unwrap()
            .as_slice()
            .is_empty()
    );
    assert_eq!(
        from_nat::<64>(&Nat(BigUint::from(1_u8) << 512)),
        Err(Error::Limit)
    );
    assert_eq!(from_nat::<0>(&Nat::from(1_u8)), Err(Error::Limit));
}

#[test]
fn rejects_nonminimal_numeric_evidence_without_normalizing_it() {
    for bytes in [vec![0], vec![0, 1], vec![0, 255]] {
        let magnitude = Bytes::<64>::new(bytes).unwrap();
        assert_eq!(to_nat(&magnitude), Err(Error::Malformed));
    }
}

#[test]
fn optional_evidence_preserves_missing_and_recorded_zero() {
    let limits = ic_wire::Limits {
        bytes: 512,
        allocation: 2048,
        nodes: 32,
        depth: 8,
    };
    let mut frames = Vec::new();
    for value in [
        None,
        Some(Nat::from(0_u8)),
        Some(Nat(BigUint::from(1_u8) << 128)),
    ] {
        let magnitude = value.as_ref().map(from_nat::<64>).transpose().unwrap();
        let frame = ic_wire::native::encode_response(71, &magnitude, limits).unwrap();
        let decoded: Option<Bytes<64>> =
            ic_wire::native::decode_response(71, &frame, limits).unwrap();
        assert_eq!(decoded.as_ref().map(to_nat).transpose().unwrap(), value);
        frames.push(frame);
    }
    assert_ne!(frames[0], frames[1]);
    assert_ne!(frames[1], frames[2]);
}
