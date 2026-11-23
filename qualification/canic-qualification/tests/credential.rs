//! A local credential signed by the qualification issuer's public fixture seed.
use canic_auth_witness::{
    KEY,
    protocol::{
        SessionInput,
        model::{PortableCredential, PortableProof},
        wire as auth_wire,
    },
};
use ic_auth::{
    claims::encode_claims,
    verification::{Ed25519Key, browser_principal},
};
use ic_auth_protocol_types::{DelegatedRoleGrant, PortableClaims, PortableMechanism, Principal};
use ic_platform_crypto::ed25519::{Signer, SigningKey};
use ic_testkit::pocket_ic::CanisterId;
use sha2::{Digest, Sha256};

pub fn signed(
    now: u64,
    device: u8,
    slot: u8,
    issuer: Principal,
    subject: Principal,
    scopes: &[&str],
) -> (CanisterId, SessionInput) {
    signed_with_binding(
        now,
        device,
        slot,
        issuer,
        subject,
        scopes,
        Some(b"qualification-binding"),
    )
}

pub fn signed_with_binding(
    now: u64,
    device: u8,
    slot: u8,
    issuer: Principal,
    subject: Principal,
    scopes: &[&str],
    binding: Option<&[u8]>,
) -> (CanisterId, SessionInput) {
    signed_for_key(now, [device; 32], slot, issuer, subject, scopes, binding)
}

pub fn signed_for_key(
    now: u64,
    device: [u8; 32],
    slot: u8,
    issuer: Principal,
    subject: Principal,
    scopes: &[&str],
    binding: Option<&[u8]>,
) -> (CanisterId, SessionInput) {
    let key = SigningKey::from_bytes(&[7; 32]);
    assert_eq!(key.verifying_key().to_bytes(), KEY);
    let browser = SigningKey::from_bytes(&device).verifying_key().to_bytes();
    let claims = encode_claims(&PortableClaims {
        ext: binding.map(<[u8]>::to_vec),
        mechanism: PortableMechanism::Ed25519,
        audience: canic_auth_witness::audience(),
        issuer,
        key_binding_id: Ed25519Key::new(KEY).unwrap().binding(),
        browser_public_key: browser,
        account_subject: subject,
        device_slot: slot,
        device_generation: 1,
        login_started_at_ns: now,
        login_deadline_ns: now + 300 * 1_000_000_000,
        issued_at_ns: now,
        admission_expires_at_ns: now + 60 * 1_000_000_000,
        nonce: [slot; 16],
        grants: vec![DelegatedRoleGrant {
            target: "user".parse().unwrap(),
            scopes: scopes.iter().map(|s| s.parse().unwrap()).collect(),
        }],
    })
    .unwrap();
    let signature = key.sign(&Sha256::digest(&claims)).to_bytes();
    (
        CanisterId::from_slice(browser_principal(&browser).as_slice()),
        SessionInput {
            credential: PortableCredential {
                claims: auth_wire::Bytes::new(claims).unwrap(),
                proof: PortableProof::Ed25519(auth_wire::Bytes::new(signature.to_vec()).unwrap()),
            },
            scopes: auth_wire::List::new(
                scopes
                    .iter()
                    .map(|s| auth_wire::Text::new((*s).to_owned()).unwrap())
                    .collect(),
            )
            .unwrap(),
            ttl_secs: Some(60),
        },
    )
}
