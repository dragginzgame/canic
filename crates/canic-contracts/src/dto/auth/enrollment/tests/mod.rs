//! Candid transport preserves Root enrollment authority and freshness fields.

use super::*;
use crate::test::support::fleet_key;

fn binding() -> ApplicationEnrollmentBinding {
    ApplicationEnrollmentBinding {
        fleet: fleet_key(7),
        root: Principal::from_slice(&[1]),
        root_installation: [2; 32],
        receiver: Principal::from_slice(&[3]),
        receiver_installation: [4; 32],
        role: CanisterRole::new("app"),
    }
}

#[test]
fn enrollment_request_preserves_installations_and_challenge() {
    let request = ApplicationEnrollmentRequest {
        binding: binding(),
        challenge: [5; 32],
    };
    let encoded = ic_platform::candid::contract::encode_one(&request).unwrap();
    let decoded: ApplicationEnrollmentRequest =
        ic_platform::candid::decoder::decode_one(&encoded).unwrap();
    assert_eq!(decoded, request);
}

#[test]
fn complete_enrollment_preserves_both_mechanisms_and_revocation_marks() {
    let snapshot = ApplicationEnrollmentSnapshot {
        binding: binding(),
        challenge: [5; 32],
        revision: u64::MAX,
        generation: 23,
        issued_at_ns: 1_000_000_000,
        expires_at_ns: 2_000_000_000,
        entries: vec![
            ApplicationEnrollmentEntry {
                issuer: Principal::from_slice(&[6]),
                key_id: [7; 32],
                key: ApplicationEnrollmentKey::Certified,
                scopes: vec!["read".into()],
                not_before_ns: 1_000_000_000,
                expires_at_ns: 2_000_000_000,
                approval_digest: [8; 32],
            },
            ApplicationEnrollmentEntry {
                issuer: Principal::from_slice(&[9]),
                key_id: [10; 32],
                key: ApplicationEnrollmentKey::Ed25519([11; 32]),
                scopes: vec!["read".into(), "write".into()],
                not_before_ns: 1_000_000_000,
                expires_at_ns: 2_000_000_000,
                approval_digest: [12; 32],
            },
        ],
        revoked_keys: vec![[13; 32], [14; 32]],
    };
    let encoded = ic_platform::candid::contract::encode_one(&snapshot).unwrap();
    let decoded: ApplicationEnrollmentSnapshot =
        ic_platform::candid::decoder::decode_one(&encoded).unwrap();
    assert_eq!(decoded, snapshot);
}
