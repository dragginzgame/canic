//! Genuine Ed25519 possession with retained Root intent and lookup-only recovery.

use super::*;
use crate::{
    model::auth::portable_registration::PortableIssuerPreparationState,
    test::{seams, support::fleet_key},
};
use canic_contracts::dto::portable_enrollment::PortableEnrollmentGrant;
use ic_platform::ed25519::{Signer, SigningKey};

const NOW: u64 = 10_000;

fn maximum_request(operation: u8, seed: u8) -> PreparePortableIssuer {
    let mut request = request(operation, seed);
    let limits = ic_auth_protocol_types::limits::MAX_LABEL_BYTES;
    request.approval.grants = (0..ic_auth_protocol_types::limits::MAX_GRANTS)
        .map(|role| PortableEnrollmentGrant {
            role: format!("role{role:02}{}", "x".repeat(limits - 6)),
            scopes: (0..ic_auth_protocol_types::limits::MAX_SCOPES_PER_GRANT)
                .map(|scope| format!("scope{scope:02}{}", "x".repeat(limits - 7)))
                .collect(),
        })
        .collect();
    request
}

#[test]
fn maximum_grants_register_and_recover_the_original_approval() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let request = maximum_request(1, 42);
    let challenge = finish(ticket(request.clone(), NOW), [9; 32], NOW).unwrap();
    let PortableIssuerPreparationState::Ready(replayed) =
        begin(domain(), request.clone(), NOW).unwrap()
    else {
        panic!("completed preparation must return its retained challenge");
    };
    assert_eq!(replayed, challenge);
    let signed = proof(&challenge, 42);
    let result = register(domain(), request.approval.issuer, signed.clone(), NOW).unwrap();
    assert_eq!(result, enrollment(&request.approval));
    let checkpoint = RootPortableEnrollmentState::read();
    assert_eq!(
        lookup(domain(), request.approval.issuer, request.operation_id, NOW),
        Ok(PortableIssuerRegistrationStatus::Registered(result.clone()))
    );
    assert_eq!(RootPortableEnrollmentState::read(), checkpoint);
    assert_eq!(
        register(
            domain(),
            request.approval.issuer,
            signed,
            challenge.expires_at_ns
        ),
        Ok(result)
    );
}

#[test]
fn root_byte_capacity_refuses_publication_without_losing_authority_or_pending_work() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let first = maximum_request(1, 42);
    let challenge = finish(ticket(first.clone(), NOW), [9; 32], NOW).unwrap();
    let original = register(domain(), first.approval.issuer, proof(&challenge, 42), NOW).unwrap();
    let mut second = maximum_request(2, 43);
    second.approval.issuer = Principal::from_slice(&[3]);
    let challenge = finish(ticket(second.clone(), NOW), [8; 32], NOW).unwrap();
    let checkpoint = RootPortableEnrollmentState::read();
    assert_eq!(
        register(domain(), second.approval.issuer, proof(&challenge, 43), NOW),
        Err(E::Capacity)
    );
    assert_eq!(RootPortableEnrollmentState::read(), checkpoint);
    assert_eq!(
        lookup(domain(), first.approval.issuer, first.operation_id, NOW),
        Ok(PortableIssuerRegistrationStatus::Registered(original))
    );
    assert_eq!(
        lookup(domain(), second.approval.issuer, second.operation_id, NOW),
        Ok(PortableIssuerRegistrationStatus::Challenge(challenge))
    );
}

struct StateGuard(PortableEnrollmentRecord);
impl StateGuard {
    fn empty() -> Self {
        let previous = RootPortableEnrollmentState::read();
        RootPortableEnrollmentState::write(PortableEnrollmentRecord::default());
        Self(previous)
    }
}
impl Drop for StateGuard {
    fn drop(&mut self) {
        RootPortableEnrollmentState::write(self.0.clone());
    }
}

fn domain() -> EnrollmentDomain {
    EnrollmentDomain {
        authority: Principal::from_slice(&[1]),
        fleet: fleet_key(1),
    }
}

fn request(operation: u8, seed: u8) -> PreparePortableIssuer {
    PreparePortableIssuer {
        operation_id: [operation; 32],
        approval: PortableIssuerApproval {
            issuer: Principal::from_slice(&[2]),
            public_key: SigningKey::from_bytes(&[seed; 32])
                .verifying_key()
                .to_bytes(),
            custody: PortableIssuerCustody::TrustedVolatileLocal,
            not_before_ns: NOW - 1,
            expires_at_ns: NOW + wire::REGISTRATION_CHALLENGE_TTL_NS * 2,
            grants: vec![PortableEnrollmentGrant {
                role: "app_service".into(),
                scopes: vec!["read".into()],
            }],
            governance_digest: [7; 32],
            governance_expires_at_ns: NOW + wire::REGISTRATION_CHALLENGE_TTL_NS * 3,
        },
    }
}

fn ticket(request: PreparePortableIssuer, now_ns: u64) -> PortableIssuerPreparation {
    match begin(domain(), request, now_ns).unwrap() {
        PortableIssuerPreparationState::Pending(ticket) => ticket,
        PortableIssuerPreparationState::Ready(_) => panic!("expected pending intent"),
    }
}

fn proof(challenge: &PortableIssuerChallenge, seed: u8) -> RegisterPortableIssuer {
    RegisterPortableIssuer {
        operation_id: challenge.operation_id,
        signature: SigningKey::from_bytes(&[seed; 32])
            .sign(&registration_message(challenge).unwrap())
            .to_bytes()
            .to_vec(),
    }
}

#[test]
fn root_registration_signing_contract_covers_every_authority_field() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let challenge = finish(ticket(request(1, 42), NOW), [9; 32], NOW).unwrap();
    let signature = proof(&challenge, 42);
    let key = VerifyingKey::from_bytes(&challenge.public_key).unwrap();
    let signature = Signature::from_slice(&signature.signature).unwrap();
    let message = registration_message(&challenge).unwrap();
    key.verify_strict(&message, &signature).unwrap();
    assert_eq!(
        <[u8; 32]>::from(Sha256::digest(&message)),
        [
            0xd7, 0x96, 0x79, 0x55, 0xf2, 0x8d, 0x94, 0x3e, 0x62, 0x2f, 0x4a, 0x63, 0xb2, 0xf0,
            0x10, 0xa2, 0xc9, 0xad, 0xc6, 0x81, 0x5b, 0x2f, 0xc2, 0x51, 0x0c, 0x88, 0xb2, 0xcf,
            0x91, 0xfe, 0x81, 0xda
        ]
    );
    assert_eq!(
        signature.to_bytes(),
        [
            0xc0, 0xb4, 0x9c, 0x5d, 0xe6, 0x19, 0x91, 0xad, 0xb2, 0x9c, 0xb2, 0x96, 0xcd, 0x12,
            0x14, 0x30, 0x75, 0x74, 0x99, 0x4c, 0x38, 0x2c, 0xd5, 0x7e, 0x24, 0x7f, 0x8b, 0x6e,
            0xee, 0x4a, 0xfa, 0x14, 0xb7, 0x5a, 0xdf, 0xca, 0xb0, 0x7b, 0xc4, 0xe7, 0xc5, 0xb7,
            0xc2, 0x3f, 0x92, 0xb4, 0x96, 0xb3, 0x1e, 0x63, 0x4a, 0x89, 0xde, 0x29, 0x98, 0xe9,
            0x10, 0xde, 0xd4, 0x81, 0xc6, 0xd9, 0x51, 0x01
        ]
    );
    let mut substitutions = Vec::new();
    let mut changed = challenge.clone();
    changed.authority = Principal::from_slice(&[3]);
    substitutions.push(changed);
    let mut changed = challenge.clone();
    changed.fleet = fleet_key(2);
    substitutions.push(changed);
    let mut changed = challenge.clone();
    changed.issuer = Principal::from_slice(&[3]);
    substitutions.push(changed);
    let mut changed = challenge.clone();
    changed.public_key[0] ^= 1;
    substitutions.push(changed);
    let mut changed = challenge.clone();
    changed.operation_id[0] ^= 1;
    substitutions.push(changed);
    let mut changed = challenge.clone();
    changed.approval_digest[0] ^= 1;
    substitutions.push(changed);
    let mut changed = challenge.clone();
    changed.nonce[0] ^= 1;
    substitutions.push(changed);
    let mut changed = challenge.clone();
    changed.issued_at_ns += 1;
    substitutions.push(changed);
    let mut changed = challenge;
    changed.expires_at_ns += 1;
    substitutions.push(changed);
    for substituted in substitutions {
        assert!(
            key.verify_strict(&registration_message(&substituted).unwrap(), &signature)
                .is_err()
        );
    }
    assert!(
        key.verify_strict(&message[wire::REGISTRATION_DOMAIN.len()..], &signature)
            .is_err()
    );
}

#[test]
fn root_registration_binds_actual_issuer_key_domain_and_original_deadlines() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let request = request(1, 42);
    let challenge = finish(ticket(request.clone(), NOW), [9; 32], NOW).unwrap();
    let before_authorization = RootPortableEnrollmentState::read();
    assert_eq!(authorize_issuer(domain(), request.approval.issuer), Ok(()));
    assert_eq!(
        authorize_issuer(domain(), Principal::anonymous()),
        Err(E::Binding)
    );
    assert_eq!(
        authorize_issuer(domain(), Principal::from_slice(&[3])),
        Err(E::Binding)
    );
    assert_eq!(RootPortableEnrollmentState::read(), before_authorization);
    assert_eq!(
        lookup(domain(), request.approval.issuer, request.operation_id, NOW).unwrap(),
        PortableIssuerRegistrationStatus::Challenge(challenge.clone())
    );
    let signature = proof(&challenge, 42);
    assert_eq!(
        register(domain(), Principal::anonymous(), signature.clone(), NOW),
        Err(E::Binding)
    );
    let mut wrong_domain = domain();
    wrong_domain.fleet = fleet_key(2);
    assert_eq!(
        register(
            wrong_domain,
            request.approval.issuer,
            signature.clone(),
            NOW
        ),
        Err(E::Binding)
    );
    assert_eq!(
        register(
            domain(),
            request.approval.issuer,
            proof(&challenge, 43),
            NOW
        ),
        Err(E::Possession)
    );
    let result = register(domain(), request.approval.issuer, signature.clone(), NOW).unwrap();
    assert_eq!(result, enrollment(&request.approval));
    let bytes = RootPortableEnrollmentState::read().bytes;
    let after_challenge = challenge.expires_at_ns + 1;
    assert_eq!(
        register(
            domain(),
            request.approval.issuer,
            signature,
            after_challenge
        )
        .unwrap(),
        result
    );
    assert_eq!(RootPortableEnrollmentState::read().bytes, bytes);
    let before_read = RootPortableEnrollmentState::read();
    assert_eq!(
        lookup(
            domain(),
            request.approval.issuer,
            request.operation_id,
            after_challenge
        )
        .unwrap(),
        PortableIssuerRegistrationStatus::Registered(result)
    );
    assert_eq!(RootPortableEnrollmentState::read(), before_read);
}

#[test]
fn root_registration_retires_previous_authority_and_rejects_late_callbacks() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let first = request(1, 42);
    let challenge = finish(ticket(first.clone(), NOW), [8; 32], NOW).unwrap();
    register(domain(), first.approval.issuer, proof(&challenge, 42), NOW).unwrap();
    let old_ticket = ticket(request(2, 43), NOW + 1);
    let state = load_registry(&RootPortableEnrollmentState::read()).unwrap();
    assert!(state.entries.is_empty());
    assert!(
        state
            .retired_keys
            .contains(&key_id(&enrollment(&first.approval).key))
    );
    let current_ticket = ticket(request(3, 44), NOW + 2);
    assert_eq!(finish(old_ticket, [7; 32], NOW + 3), Err(E::Conflict));
    let current = finish(current_ticket.clone(), [6; 32], NOW + 4).unwrap();
    assert_eq!(finish(current_ticket, [5; 32], NOW + 5).unwrap(), current);
    assert_eq!(
        register(
            domain(),
            first.approval.issuer,
            proof(&challenge, 42),
            NOW + 5
        ),
        Err(E::Conflict)
    );
    assert_eq!(
        begin(domain(), request(4, 42), NOW + 6).err(),
        Some(E::Retired)
    );
}

#[test]
fn root_registration_never_downgrades_tee_or_refreshes_expired_work() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let mut tee = request(1, 42);
    tee.approval.custody = PortableIssuerCustody::TeeRequired;
    assert_eq!(begin(domain(), tee, NOW).err(), Some(E::Custody));
    assert!(RootPortableEnrollmentState::read().bytes.is_empty());
    let request = request(2, 42);
    let challenge = finish(ticket(request.clone(), NOW), [9; 32], NOW).unwrap();
    let mut changed = request.clone();
    changed.approval.governance_digest = [8; 32];
    assert_eq!(begin(domain(), changed, NOW).err(), Some(E::Conflict));
    assert_eq!(
        register(
            domain(),
            request.approval.issuer,
            proof(&challenge, 42),
            challenge.expires_at_ns
        ),
        Err(E::Expired)
    );
    let before = RootPortableEnrollmentState::read();
    assert_eq!(
        lookup(
            domain(),
            request.approval.issuer,
            request.operation_id,
            challenge.expires_at_ns + 1
        )
        .unwrap(),
        PortableIssuerRegistrationStatus::Challenge(challenge)
    );
    assert_eq!(RootPortableEnrollmentState::read(), before);
    assert_eq!(
        begin(
            domain(),
            request,
            NOW + wire::REGISTRATION_CHALLENGE_TTL_NS + 1
        )
        .err(),
        Some(E::Expired)
    );
}

#[test]
fn root_registration_revocation_cancels_pending_work_and_never_reenrolls_an_issued_key() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let first = request(1, 42);
    let challenge = finish(ticket(first.clone(), NOW), [9; 32], NOW).unwrap();
    register(domain(), first.approval.issuer, proof(&challenge, 42), NOW).unwrap();
    assert_eq!(
        crate::ops::auth::enrollment::authority::commit_registry(
            domain(),
            Vec::new(),
            Vec::new(),
            NOW
        ),
        Err(E::Conflict)
    );
    retire(domain(), first.approval.issuer, NOW + 1).unwrap();
    assert_eq!(
        lookup(domain(), first.approval.issuer, first.operation_id, NOW + 2),
        Err(E::Unavailable)
    );
    assert_eq!(
        begin(domain(), request(2, 42), NOW + 2).err(),
        Some(E::Retired)
    );
    let pending = ticket(request(3, 43), NOW + 3);
    retire(domain(), first.approval.issuer, NOW + 4).unwrap();
    assert_eq!(finish(pending, [8; 32], NOW + 5), Err(E::Conflict));
}

#[test]
fn root_registration_recovers_completion_from_exact_authority_without_reissuing_it() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let request = request(1, 42);
    let challenge = finish(ticket(request.clone(), NOW), [9; 32], NOW).unwrap();
    let signature = proof(&challenge, 42);
    let result = register(domain(), request.approval.issuer, signature.clone(), NOW).unwrap();
    let record = RootPortableEnrollmentState::read();
    let registry = load_registry(&record).unwrap();
    let generation = registry.generation;
    let after_challenge = challenge.expires_at_ns + 1;
    assert_eq!(
        lookup(
            domain(),
            request.approval.issuer,
            request.operation_id,
            after_challenge
        )
        .unwrap(),
        PortableIssuerRegistrationStatus::Registered(result.clone())
    );
    assert_eq!(RootPortableEnrollmentState::read(), record);
    assert_eq!(
        register(
            domain(),
            request.approval.issuer,
            signature,
            after_challenge
        )
        .unwrap(),
        result
    );
    let recovered = load_registry(&RootPortableEnrollmentState::read()).unwrap();
    assert_eq!(recovered.generation, generation);
    assert_eq!(recovered.entries, registry.entries);
    assert_eq!(recovered, registry);
}
