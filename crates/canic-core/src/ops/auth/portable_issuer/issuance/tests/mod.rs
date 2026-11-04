//! Issuance custody, original deadlines, account isolation and late authority replies.

mod bounds;
mod reclamation;
mod retry;

use super::*;
use crate::{
    ops::auth::portable_issuer as keys,
    storage::stable::auth::PortableEnrollmentRecord,
    test::{seams, support::fleet_key},
};
use canic_contracts::dto::portable_enrollment::{
    PortableEnrollmentGrant, PortableIssuerChallenge, PortableIssuerCustody,
    PortableIssuerEnrollment, PreparePortableIssuerKey,
};
use ic_platform::ed25519::{Signature, SigningKey};

const NOW: u64 = 100_000_000_000;
const SECOND: u64 = 1_000_000_000;

struct StateGuard {
    checkpoint: crate::storage::stable::auth::PortableEnrollmentRecord,
    key: Option<keys::VolatileKey>,
}
impl StateGuard {
    fn empty() -> Self {
        let guard = Self {
            checkpoint: PortableIssuerState::read(),
            key: KEY.with_borrow_mut(Option::take),
        };
        PortableIssuerState::write(PortableEnrollmentRecord::default());
        keys::restore().unwrap();
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
        issuer: Principal::from_slice(&[2]),
        fleet: fleet_key(1),
    }
}
fn account() -> Principal {
    Principal::from_slice(&[43])
}
fn policy() -> PortableIssuancePolicy {
    PortableIssuancePolicy {
        login_ttl_ns: 3_600 * SECOND,
        proof_ttl_ns: 6 * SECOND,
        receipt_timeout_ns: 10 * SECOND,
        reconciliation_retention_ns: 60 * SECOND,
    }
}
fn request(id: u8) -> IssuePortableCredential {
    IssuePortableCredential {
        epoch: 1,
        operation_id: [id; 32],
        login_epoch: 1,
        login_id: [91; 32],
        browser_public_key: SigningKey::from_bytes(&[3; 32]).verifying_key().to_bytes(),
        grants: vec![PortableEnrollmentGrant {
            role: "issuer".into(),
            scopes: vec!["read".into()],
        }],
    }
}
fn enroll() -> PortableIssuerEnrollment {
    enroll_with_grants(request(1).grants)
}

fn enroll_with_grants(grants: Vec<PortableEnrollmentGrant>) -> PortableIssuerEnrollment {
    let keys::PortableIssuerKeyPreparation::Pending(ticket) = keys::begin(
        domain(),
        PreparePortableIssuerKey {
            operation_id: [11; 32],
            custody: PortableIssuerCustody::TrustedVolatileLocal,
        },
    )
    .unwrap() else {
        panic!("new key");
    };
    let prepared = keys::finish(ticket, [1; 32]).unwrap();
    let (ticket, _) = keys::registration(
        domain(),
        PortableIssuerChallenge {
            authority: domain().authority,
            fleet: domain().fleet,
            issuer: domain().issuer,
            public_key: prepared.public_key,
            operation_id: prepared.operation_id,
            approval_digest: [6; 32],
            nonce: [7; 32],
            issued_at_ns: NOW,
            expires_at_ns: NOW + 60 * SECOND,
        },
        NOW,
    )
    .unwrap();
    keys::registered(
        &ticket,
        PortableIssuerEnrollment {
            issuer: domain().issuer,
            key: PortableIssuerKey::Ed25519 {
                public_key: prepared.public_key,
            },
            not_before_ns: NOW,
            expires_at_ns: NOW + 3_600 * SECOND,
            deployment_approval_digest: [6; 32],
            deployment_approval_expires_at_ns: NOW + 3_600 * SECOND,
            grants,
        },
    )
    .unwrap()
}
fn pending_at(id: u8, now_ns: u64) -> PortableIssuanceTicket {
    match begin(domain(), account(), request(id), policy(), now_ns).unwrap() {
        PortableIssuancePreparation::Pending(ticket) => ticket,
        PortableIssuancePreparation::Ready(_) => panic!("new issue operation"),
    }
}
fn snapshot(
    pending: &PortableIssuanceTicket,
    entry: PortableIssuerEnrollment,
    revision: u64,
) -> PortableEnrollmentSnapshot {
    PortableEnrollmentSnapshot {
        authority: domain().authority,
        receiver: domain().issuer,
        fleet: domain().fleet,
        boot_challenge: challenge(pending, [revision.try_into().unwrap(); 32], NOW).unwrap(),
        revision,
        generation: 1,
        issued_at_ns: NOW,
        expires_at_ns: NOW + 3_600 * SECOND,
        entries: vec![entry],
        retired_keys: vec![],
    }
}
fn complete(
    pending: &PortableIssuanceTicket,
    snapshot: PortableEnrollmentSnapshot,
) -> IssuedPortableCredential {
    accept_authority(pending, snapshot, NOW).unwrap();
    finish(pending, NOW).unwrap()
}

#[test]
fn authentic_subject_browser_binding_and_original_credential_survive_exact_recovery() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let entry = enroll();
    let pending = pending_at(1, NOW);
    assert_eq!(
        reconcile(&domain(), account(), 1, [1; 32]),
        Ok(PortableCredentialIssuanceStatus::Unresolved)
    );
    let snapshot = snapshot(&pending, entry.clone(), 1);
    let result = complete(&pending, snapshot);
    let PortableIssuancePreparation::Ready(replayed) =
        begin(domain(), account(), request(1), policy(), NOW).unwrap()
    else {
        panic!("completed issuance must return its retained credential");
    };
    assert_eq!(replayed, result);
    let credential: PortableCredential = result.credential.clone().try_into().unwrap();
    let decoded = ic_auth::portable::decode_claims(&credential.claims).unwrap();
    assert_eq!(decoded.account_subject, account());
    assert_eq!(decoded.browser_public_key, request(1).browser_public_key);
    assert_ne!(
        ic_auth::portable::browser_principal(&decoded.browser_public_key),
        account()
    );
    assert_eq!(decoded.admission_expires_at_ns, NOW + 6 * SECOND);
    assert_eq!(
        result.admission_deadline_ns,
        decoded.admission_expires_at_ns
    );
    let PortableProof::Ed25519 { signature } = credential.proof else {
        panic!("local proof");
    };
    let PortableIssuerKey::Ed25519 { public_key } = entry.key else {
        panic!("local key");
    };
    VerifyingKey::from_bytes(&public_key)
        .unwrap()
        .verify_strict(
            &ic_auth::portable::claims_hash(&decoded).unwrap(),
            &Signature::from_slice(&signature).unwrap(),
        )
        .unwrap();
    assert_eq!(finish(&pending, NOW + 60 * SECOND), Ok(result.clone()));
    let before = PortableIssuerState::read();
    assert_eq!(
        reconcile(&domain(), account(), 1, [1; 32]),
        Ok(PortableCredentialIssuanceStatus::Issued(result.clone()))
    );
    assert_eq!(
        reconcile(&domain(), Principal::from_slice(&[44]), 1, [1; 32]),
        Ok(PortableCredentialIssuanceStatus::Unresolved)
    );
    assert_eq!(
        reconcile(&domain(), account(), 1, [2; 32]),
        Ok(PortableCredentialIssuanceStatus::Unresolved)
    );
    assert_eq!(
        PortableIssuerState::read(),
        before,
        "lookup performs no stable writes"
    );
    keys::restore().unwrap();
    assert_eq!(
        reconcile(&domain(), account(), 1, [1; 32]),
        Ok(PortableCredentialIssuanceStatus::Issued(result))
    );
}

#[test]
fn nonce_first_completion_and_exact_intent_conflicts_never_replace_work() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    enroll();
    let pending = pending_at(1, NOW);
    assert_eq!(challenge(&pending, [5; 32], NOW), Ok([5; 32]));
    assert_eq!(challenge(&pending, [6; 32], NOW), Ok([5; 32]));
    for field in 0..4 {
        let mut changed = request(1);
        let mut limits = policy();
        match field {
            0 => changed.login_id = [92; 32],
            1 => {
                changed.browser_public_key =
                    SigningKey::from_bytes(&[4; 32]).verifying_key().to_bytes();
            }
            2 => changed.grants[0].scopes = vec!["write".into()],
            _ => limits.proof_ttl_ns += SECOND,
        }
        assert!(matches!(
            begin(domain(), account(), changed, limits, NOW),
            Err(E::Conflict)
        ));
    }
    assert_eq!(load().unwrap().issuances.len(), 1);
    assert_eq!(
        load().unwrap().issuances[0].admission_deadline_ns,
        NOW + 6 * SECOND
    );
}

#[test]
fn pending_expiry_and_restore_do_not_authorize_replacement_or_signing() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let entry = enroll();
    let pending = pending_at(1, NOW);
    let original = snapshot(&pending, entry, 1);
    assert_eq!(
        accept_authority(&pending, original.clone(), NOW + 6 * SECOND),
        Err(E::Expired)
    );
    assert!(matches!(
        begin(domain(), account(), request(1), policy(), NOW + 6 * SECOND),
        Err(E::Expired)
    ));
    assert_eq!(
        reconcile(&domain(), account(), 1, [1; 32]),
        Ok(PortableCredentialIssuanceStatus::Unresolved)
    );
    keys::restore().unwrap();
    assert_eq!(
        accept_authority(&pending, original, NOW + 6 * SECOND),
        Err(E::Conflict)
    );
    assert_eq!(
        load().unwrap().issuances[0].admission_deadline_ns,
        NOW + 6 * SECOND
    );
}

#[test]
fn renewed_proof_preserves_original_login_and_rejects_a_retargeted_browser() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let entry = enroll();
    let first = pending_at(1, NOW);
    let root = snapshot(&first, entry, 1);
    complete(&first, root);
    let next = pending_at(2, NOW + SECOND);
    let journal = load().unwrap();
    assert_eq!(journal.issuances[1].login_started_at_ns, NOW);
    assert_eq!(
        journal.issuances[1].login_deadline_ns,
        journal.issuances[0].login_deadline_ns
    );
    assert_eq!(journal.issuances[1].issued_at_ns, NOW + SECOND);
    let mut changed = request(3);
    changed.browser_public_key = SigningKey::from_bytes(&[4; 32]).verifying_key().to_bytes();
    assert!(matches!(
        begin(domain(), account(), changed, policy(), NOW + SECOND),
        Err(E::Conflict)
    ));
    assert_ne!(next.operation_id, first.operation_id);
}

#[test]
fn fresh_root_scope_and_binding_checks_precede_signing_and_retain_revocation_progress() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let entry = enroll();
    let first = pending_at(1, NOW);
    let old = snapshot(&first, entry.clone(), 1);
    let mut bad = old.clone();
    bad.receiver = account();
    assert_eq!(accept_authority(&first, bad, NOW), Err(E::Binding));
    let mut retired = old.clone();
    retired.entries.clear();
    let PortableIssuerKey::Ed25519 { public_key } = entry.key else {
        panic!("local");
    };
    retired.retired_keys = vec![Sha256::digest(public_key).into()];
    retired.revision = 2;
    retired.generation = 2;
    accept_authority(&first, retired, NOW).unwrap();
    assert_eq!(finish(&first, NOW), Err(E::Unavailable));
    assert_eq!(accept_authority(&first, old, NOW), Err(E::Storage));
    assert_eq!(load().unwrap().enrollment.unwrap().revision, 2);
    assert_eq!(
        reconcile(&domain(), account(), 1, [1; 32]),
        Ok(PortableCredentialIssuanceStatus::Unresolved)
    );
}

#[test]
fn root_ceiling_clock_and_policy_limits_cannot_be_bypassed_by_valid_signing_key() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let entry = enroll();
    let mut requested = request(1);
    requested.grants[0].scopes = vec!["write".into()];
    let PortableIssuancePreparation::Pending(pending) =
        begin(domain(), account(), requested, policy(), NOW).unwrap()
    else {
        panic!("new intent");
    };
    let root = snapshot(&pending, entry, 1);
    accept_authority(&pending, root, NOW).unwrap();
    assert_eq!(finish(&pending, NOW), Err(E::Denied));
    assert!(matches!(
        begin(domain(), account(), request(2), policy(), NOW - 1),
        Err(E::Clock)
    ));
    let mut bad = policy();
    bad.proof_ttl_ns = 0;
    assert!(matches!(
        begin(domain(), account(), request(2), bad, NOW),
        Err(E::Invalid)
    ));
    assert!(matches!(
        begin(domain(), Principal::anonymous(), request(2), policy(), NOW),
        Err(E::Denied)
    ));
    assert!(
        !crate::domain::policy::pure::auth::portable_clock_is_qualified(
            canic_contracts::ids::BuildNetwork::Ic
        )
    );
}

#[test]
fn journal_capacity_and_read_only_missing_lookup_preserve_retained_work() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let empty = PortableIssuerState::read();
    assert_eq!(
        reconcile(&domain(), account(), 1, [1; 32]),
        Ok(PortableCredentialIssuanceStatus::Unresolved)
    );
    assert_eq!(PortableIssuerState::read(), empty);
    enroll();
    for id in 1..=u8::try_from(MAX_ISSUANCES).unwrap() {
        pending_at(id, NOW);
    }
    assert!(matches!(
        begin(domain(), account(), request(99), policy(), NOW),
        Err(E::Capacity)
    ));
    assert_eq!(load().unwrap().issuances.len(), MAX_ISSUANCES);
    let before = PortableIssuerState::read();
    assert_eq!(
        reconcile(&domain(), account(), 1, [99; 32]),
        Ok(PortableCredentialIssuanceStatus::Unresolved)
    );
    assert_eq!(
        reconcile(&domain(), Principal::anonymous(), 1, [1; 32]),
        Err(E::Denied)
    );
    assert_eq!(PortableIssuerState::read(), before);
}

#[test]
fn auth_owns_proof_window_ceiling_before_any_credential_is_signed() {
    let _lock = seams::lock();
    let _state = StateGuard::empty();
    let entry = enroll();
    let mut limits = policy();
    limits.proof_ttl_ns = 61 * SECOND;
    let PortableIssuancePreparation::Pending(pending) =
        begin(domain(), account(), request(1), limits, NOW).unwrap()
    else {
        panic!("new intent");
    };
    let root = snapshot(&pending, entry, 1);
    accept_authority(&pending, root, NOW).unwrap();
    assert_eq!(finish(&pending, NOW), Err(E::Denied));
    assert_eq!(
        reconcile(&domain(), account(), 1, [1; 32]),
        Ok(PortableCredentialIssuanceStatus::Unresolved)
    );
}
