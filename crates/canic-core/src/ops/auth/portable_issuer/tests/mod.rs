//! Volatile custody, public retry history and stale-callback regressions.

use super::*;
use crate::test::{seams, support::fleet_key};
use ic_platform::{
    Principal,
    ed25519::{Signature, VerifyingKey},
};

struct StateGuard {
    checkpoint: PortableEnrollmentRecord,
    key: Option<VolatileKey>,
}
impl StateGuard {
    fn empty() -> Self {
        let guard = Self {
            checkpoint: PortableIssuerState::read(),
            key: KEY.with_borrow_mut(Option::take),
        };
        PortableIssuerState::write(PortableEnrollmentRecord::default());
        restore().unwrap();
        guard
    }
}
impl Drop for StateGuard {
    fn drop(&mut self) {
        PortableIssuerState::write(self.checkpoint.clone());
        KEY.with_borrow_mut(|key| *key = self.key.take());
    }
}

fn domain() -> PortableIssuerDomain {
    PortableIssuerDomain {
        authority: Principal::from_slice(&[1]),
        fleet: fleet_key(1),
        issuer: Principal::from_slice(&[2]),
    }
}
fn request(id: u8) -> PreparePortableIssuerKey {
    PreparePortableIssuerKey {
        operation_id: [id; 32],
        custody: PortableIssuerCustody::TrustedVolatileLocal,
    }
}
fn pending(id: u8) -> PortableIssuerKeyTicket {
    match begin(domain(), request(id)).unwrap() {
        PortableIssuerKeyPreparation::Pending(ticket) => {
            let _: PortableIssuerJournalRecord =
                ic_wire::native::decode(&PortableIssuerState::read().bytes, JOURNAL_LIMITS)
                    .expect("public issuer journal must round trip");
            ticket
        }
        PortableIssuerKeyPreparation::Ready(_) => panic!("expected new operation"),
    }
}
fn key(id: u8) -> PortableIssuerPreparedKey {
    finish(pending(id), [id; 32]).unwrap()
}
fn challenge(key: &PortableIssuerPreparedKey) -> PortableIssuerChallenge {
    let domain = domain();
    PortableIssuerChallenge {
        authority: domain.authority,
        fleet: domain.fleet,
        issuer: domain.issuer,
        public_key: key.public_key,
        operation_id: key.operation_id,
        approval_digest: [8; 32],
        nonce: [9; 32],
        issued_at_ns: 100,
        expires_at_ns: 200,
    }
}
fn enrollment(key: &PortableIssuerPreparedKey) -> PortableIssuerEnrollment {
    PortableIssuerEnrollment {
        issuer: domain().issuer,
        key: PortableIssuerKey::Ed25519 {
            public_key: key.public_key,
        },
        not_before_ns: 100,
        expires_at_ns: 1_000,
        grants: vec![
            canic_contracts::dto::portable_enrollment::PortableEnrollmentGrant {
                role: "issuer".into(),
                scopes: vec!["read".into()],
            },
        ],
        deployment_approval_digest: [8; 32],
        deployment_approval_expires_at_ns: 2_000,
    }
}

#[test]
fn key_preparation_refuses_unqualified_custody_and_preserves_exact_first_completion() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let checkpoint = PortableIssuerState::read();
    let mut tee = request(1);
    tee.custody = PortableIssuerCustody::TeeRequired;
    assert!(matches!(begin(domain(), tee), Err(E::Custody)));
    assert!(matches!(begin(domain(), request(0)), Err(E::Invalid)));
    assert_eq!(checkpoint, PortableIssuerState::read());
    let ticket = pending(1);
    assert_eq!(load().unwrap().keys[0].public_key, None);
    let prepared = finish(ticket.clone(), [1; 32]).unwrap();
    assert_eq!(finish(ticket, [2; 32]), Ok(prepared.clone()));
    assert!(
        matches!(begin(domain(), request(1)), Ok(PortableIssuerKeyPreparation::Ready(result)) if result == prepared)
    );
    assert_eq!(load().unwrap().keys.len(), 1);
}

#[test]
fn supersession_rejects_old_randomness_and_never_reuses_an_old_operation() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let previous = pending(1);
    let current = pending(2);
    let checkpoint = PortableIssuerState::read();
    assert_eq!(finish(previous, [1; 32]), Err(E::Conflict));
    assert_eq!(checkpoint, PortableIssuerState::read());
    assert!(matches!(begin(domain(), request(1)), Err(E::Unavailable)));
    assert_eq!(finish(current.clone(), [0; 32]), Err(E::Invalid));
    assert!(finish(current, [2; 32]).is_ok());
}

#[test]
fn restoration_clears_ready_and_pending_keys_before_new_work() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let first = key(1);
    let first_challenge = challenge(&first);
    assert!(registration(domain(), first_challenge.clone(), 100).is_ok());
    restore().unwrap();
    assert!(KEY.with_borrow(Option::is_none));
    assert!(matches!(begin(domain(), request(1)), Err(E::Unavailable)));
    assert_eq!(
        registration(domain(), first_challenge, 100),
        Err(E::Unavailable)
    );
    assert_eq!(load().unwrap().keys[0].public_key, Some(first.public_key));
    let pending = pending(2);
    restore().unwrap();
    assert_eq!(finish(pending, [2; 32]), Err(E::Conflict));
    assert!(matches!(begin(domain(), request(2)), Err(E::Unavailable)));
    assert_ne!(key(3).public_key, first.public_key);
}

#[test]
fn registration_signs_only_original_domain_bound_challenge_and_recovers_exact_result() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let prepared = key(1);
    let challenge = challenge(&prepared);
    let (ticket, signed) = registration(domain(), challenge.clone(), 100).unwrap();
    let public_key = VerifyingKey::from_bytes(&prepared.public_key).unwrap();
    let signature = Signature::from_slice(&signed.signature).unwrap();
    public_key
        .verify_strict(&registration_message(&challenge).unwrap(), &signature)
        .unwrap();
    let checkpoint = PortableIssuerState::read();
    let mut substituted = challenge.clone();
    substituted.nonce = [7; 32];
    assert_eq!(registration(domain(), substituted, 100), Err(E::Conflict));
    assert_eq!(checkpoint, PortableIssuerState::read());
    let result = enrollment(&prepared);
    assert_eq!(registered(&ticket, result.clone()), Ok(result.clone()));
    assert_eq!(registration(domain(), challenge, 500).unwrap().1, signed);
    assert_eq!(registered(&ticket, result.clone()), Ok(result.clone()));
    let mut different_grants = result.clone();
    different_grants.grants[0].scopes = vec!["write".into()];
    assert_eq!(registered(&ticket, different_grants), Err(E::Conflict));
    assert_eq!(registered(&ticket, result.clone()), Ok(result.clone()));
    let mut different = result;
    different.expires_at_ns += 1;
    assert_eq!(registered(&ticket, different), Err(E::Conflict));
    key(2);
    assert_eq!(registered(&ticket, enrollment(&prepared)), Err(E::Conflict));
}

#[test]
fn mismatched_domains_keys_deadlines_and_unrequested_results_cannot_activate_registration() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let prepared = key(1);
    let original = challenge(&prepared);
    for substitution in 0..6 {
        let mut changed = original.clone();
        match substitution {
            0 => changed.authority = Principal::from_slice(&[3]),
            1 => changed.fleet = fleet_key(2),
            2 => changed.issuer = Principal::from_slice(&[3]),
            3 => changed.public_key = [4; 32],
            4 => changed.nonce = [0; 32],
            _ => changed.approval_digest = [0; 32],
        }
        assert_eq!(registration(domain(), changed, 100), Err(E::Binding));
    }
    assert_eq!(
        registration(domain(), original.clone(), 99),
        Err(E::Expired)
    );
    assert_eq!(
        registration(domain(), original.clone(), 200),
        Err(E::Expired)
    );
    let journal = load().unwrap();
    let ticket = ticket(&domain(), &journal, 1, prepared.operation_id);
    assert_eq!(registered(&ticket, enrollment(&prepared)), Err(E::Binding));
    let mut changed_domain = domain();
    changed_domain.fleet = fleet_key(2);
    assert!(matches!(begin(changed_domain, request(2)), Err(E::Binding)));
    assert!(registration(domain(), original, 100).is_ok());
}

#[test]
fn capacity_refusal_preserves_current_signer_and_consumed_operation_history() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let mut current = key(1);
    for id in 2..=u8::try_from(MAX_KEYS).unwrap() {
        current = key(id);
    }
    let checkpoint = PortableIssuerState::read();
    assert!(matches!(begin(domain(), request(65)), Err(E::Capacity)));
    assert_eq!(checkpoint, PortableIssuerState::read());
    assert!(registration(domain(), challenge(&current), 100).is_ok());
    assert!(matches!(begin(domain(), request(1)), Err(E::Unavailable)));
}

#[test]
fn failed_restore_still_drops_secret_and_does_not_repair_corrupt_history() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    key(1);
    PortableIssuerState::write(PortableEnrollmentRecord {
        bytes: vec![255],
        observed_at_ns: 0,
    });
    assert_eq!(restore(), Err(E::Storage));
    assert!(KEY.with_borrow(Option::is_none));
    let mut journal = PortableIssuerJournalRecord {
        boot: u64::MAX,
        issuance_epoch: 1,
        ..Default::default()
    };
    commit(&journal).unwrap();
    assert_eq!(restore(), Err(E::Capacity));
    journal.boot = 1;
    journal.keys.push(PortableIssuerKeyRecord {
        request: request(1),
        boot: 1,
        public_key: None,
        challenge: None,
        enrollment: None,
    });
    commit(&journal).unwrap();
    assert!(matches!(load(), Err(E::Storage)));
}

#[test]
fn fresh_operation_cannot_reissue_a_previously_generated_key() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let original = key(1);
    restore().unwrap();
    let next = pending(2);
    assert_eq!(finish(next.clone(), [1; 32]), Err(E::Retired));
    assert!(KEY.with_borrow(Option::is_none));
    assert_ne!(
        finish(next, [2; 32]).unwrap().public_key,
        original.public_key
    );
}
