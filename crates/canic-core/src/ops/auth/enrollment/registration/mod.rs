//! Root governance and authenticated public-key registration; no issuer secret storage.
//! Controller authentication belongs to endpoints, and the workflow captures IC callers.

use crate::{
    model::auth::portable_registration::{
        EnrollmentDomain, PortableIssuerPreparation, PortableIssuerPreparationState,
        PortableIssuerRegistrationRecord, RootPortableEnrollmentRecord,
    },
    ops::auth::enrollment::{
        PortableEnrollmentError as E,
        authority::{encode_registry, load_registry, observe, replace_registry},
        encode,
        validation::{key_id, validate_contents},
    },
    storage::stable::auth::enrollment::{PortableEnrollmentRecord, RootPortableEnrollmentState},
};
use canic_contracts::{
    dto::portable_enrollment::{
        PortableIssuerApproval, PortableIssuerChallenge, PortableIssuerCustody,
        PortableIssuerEnrollment, PortableIssuerKey, PortableIssuerRegistrationStatus,
        PreparePortableIssuer, RegisterPortableIssuer,
    },
    ops::portable_enrollment::registration_message,
    protocol::portable_enrollment as wire,
};
use ic_platform::{
    Principal,
    ed25519::{Signature, VerifyingKey},
};
use sha2::{Digest, Sha256};

#[cfg(feature = "auth-root-delegation-state")]
pub fn current_domain() -> Result<EnrollmentDomain, E> {
    let (authority, fleet) = crate::ops::auth::enrollment::authority::protected_root()?;
    Ok(EnrollmentDomain { authority, fleet })
}

/// Endpoint admission reads retained controller intent without changing the registry.
pub fn authorize_issuer(domain: EnrollmentDomain, caller: Principal) -> Result<(), E> {
    let registry = load_registry(&RootPortableEnrollmentState::read())?;
    domain_matches(&registry, &domain)?;
    if registry
        .registrations
        .iter()
        .any(|record| record.request.approval.issuer == caller)
    {
        Ok(())
    } else {
        Err(E::Binding)
    }
}

fn approval_digest(
    domain: &EnrollmentDomain,
    request: &PreparePortableIssuer,
) -> Result<[u8; 32], E> {
    let mut hash = Sha256::new();
    hash.update(wire::APPROVAL_DOMAIN);
    hash.update(encode(&(domain, request))?);
    Ok(hash.finalize().into())
}

fn enrollment(approval: &PortableIssuerApproval) -> PortableIssuerEnrollment {
    PortableIssuerEnrollment {
        issuer: approval.issuer,
        key: PortableIssuerKey::Ed25519 {
            public_key: approval.public_key,
        },
        not_before_ns: approval.not_before_ns,
        expires_at_ns: approval.expires_at_ns,
        grants: approval.grants.clone(),
        deployment_approval_digest: approval.governance_digest,
        deployment_approval_expires_at_ns: approval.governance_expires_at_ns,
    }
}

fn retained_result(
    registry: &RootPortableEnrollmentRecord,
    registration: &PortableIssuerRegistrationRecord,
) -> Option<PortableIssuerEnrollment> {
    registration.challenge.as_ref()?;
    let approved = enrollment(&registration.request.approval);
    registry
        .entries
        .iter()
        .find(|entry| **entry == approved)
        .cloned()
}

fn validate_approval(request: &PreparePortableIssuer) -> Result<(), E> {
    if request.approval.custody != PortableIssuerCustody::TrustedVolatileLocal {
        return Err(E::Custody);
    }
    if request.operation_id == [0; 32] || request.approval.public_key == [0; 32] {
        return Err(E::Invalid);
    }
    let public =
        VerifyingKey::from_bytes(&request.approval.public_key).map_err(|_| E::Possession)?;
    if public.is_weak() {
        return Err(E::Possession);
    }
    validate_contents(&[enrollment(&request.approval)], &[])
}

fn domain_matches(
    registry: &RootPortableEnrollmentRecord,
    domain: &EnrollmentDomain,
) -> Result<(), E> {
    if registry
        .domain
        .as_ref()
        .is_some_and(|stored| stored != domain)
    {
        return Err(E::Binding);
    }
    Ok(())
}

fn commit(
    mut record: PortableEnrollmentRecord,
    registry: &RootPortableEnrollmentRecord,
) -> Result<(), E> {
    validate_records(registry)?;
    record.bytes = encode_registry(registry)?;
    RootPortableEnrollmentState::write(record);
    Ok(())
}

/// Validate the retained intent before a query, callback or registry mutation uses it.
pub(super) fn validate_records(registry: &RootPortableEnrollmentRecord) -> Result<(), E> {
    if registry.registrations.len() > wire::MAX_ENTRIES {
        return Err(E::Capacity);
    }
    let mut previous = None;
    for registration in &registry.registrations {
        let issuer = registration.request.approval.issuer;
        if previous.is_some_and(|previous: Principal| previous.as_slice() >= issuer.as_slice())
            || registration.sequence == 0
            || registration.sequence > registry.registration_sequence
        {
            return Err(E::Storage);
        }
        let domain = registry.domain.as_ref().ok_or(E::Storage)?;
        validate_approval(&registration.request)?;
        if approval_digest(domain, &registration.request)? != registration.digest {
            return Err(E::Storage);
        }
        if let Some(challenge) = &registration.challenge {
            let expected = challenge_for(
                domain,
                registration,
                challenge.nonce,
                challenge.issued_at_ns,
            )?;
            if challenge != &expected {
                return Err(E::Storage);
            }
        }
        previous = Some(issuer);
    }
    Ok(())
}

fn challenge_for(
    domain: &EnrollmentDomain,
    registration: &PortableIssuerRegistrationRecord,
    nonce: [u8; 32],
    now_ns: u64,
) -> Result<PortableIssuerChallenge, E> {
    let approval = &registration.request.approval;
    if nonce == [0; 32] {
        return Err(E::Invalid);
    }
    if now_ns < approval.not_before_ns || now_ns >= approval.expires_at_ns {
        return Err(E::Expired);
    }
    Ok(PortableIssuerChallenge {
        authority: domain.authority,
        fleet: domain.fleet,
        issuer: approval.issuer,
        public_key: approval.public_key,
        operation_id: registration.request.operation_id,
        approval_digest: registration.digest,
        nonce,
        issued_at_ns: now_ns,
        expires_at_ns: now_ns
            .checked_add(wire::REGISTRATION_CHALLENGE_TTL_NS)
            .ok_or(E::Capacity)?
            .min(approval.expires_at_ns),
    })
}

/// Retain exact controller intent before randomness. Ready retries return original work.
pub fn begin(
    domain: EnrollmentDomain,
    request: PreparePortableIssuer,
    now_ns: u64,
) -> Result<PortableIssuerPreparationState, E> {
    let record = observe(now_ns)?;
    let mut registry = load_registry(&record)?;
    domain_matches(&registry, &domain)?;
    validate_approval(&request)?;
    let approval = &request.approval;
    if now_ns < approval.not_before_ns || now_ns >= approval.expires_at_ns {
        return Err(E::Expired);
    }
    let digest = approval_digest(&domain, &request)?;
    if let Some(previous) = registry
        .registrations
        .iter()
        .find(|r| r.request.approval.issuer == approval.issuer)
        && previous.request.operation_id == request.operation_id
    {
        if previous.digest != digest {
            return Err(E::Conflict);
        }
        return Ok(if let Some(challenge) = &previous.challenge {
            if retained_result(&registry, previous).is_none() && now_ns >= challenge.expires_at_ns {
                return Err(E::Expired);
            }
            PortableIssuerPreparationState::Ready(challenge.clone())
        } else {
            PortableIssuerPreparationState::Pending(PortableIssuerPreparation {
                domain,
                issuer: approval.issuer,
                sequence: previous.sequence,
                digest,
            })
        });
    }
    let id = key_id(&PortableIssuerKey::Ed25519 {
        public_key: approval.public_key,
    });
    if registry.retired_keys.binary_search(&id).is_ok() {
        return Err(E::Retired);
    }
    if registry
        .entries
        .iter()
        .any(|entry| key_id(&entry.key) == id)
    {
        return Err(E::Conflict);
    }
    if registry.registrations.len() == wire::MAX_ENTRIES
        && !registry
            .registrations
            .iter()
            .any(|r| r.request.approval.issuer == approval.issuer)
    {
        return Err(E::Capacity);
    }
    // Narrowed/replaced authority is retired before the asynchronous boundary.
    let entries = registry
        .entries
        .iter()
        .filter(|entry| entry.issuer != approval.issuer)
        .cloned()
        .collect();
    registry = replace_registry(registry, domain.clone(), entries, Vec::new())?;
    registry.registration_sequence = registry
        .registration_sequence
        .checked_add(1)
        .ok_or(E::Capacity)?;
    let sequence = registry.registration_sequence;
    let issuer = approval.issuer;
    registry
        .registrations
        .retain(|r| r.request.approval.issuer != issuer);
    registry
        .registrations
        .push(PortableIssuerRegistrationRecord {
            sequence,
            request,
            digest,
            challenge: None,
        });
    registry.registrations.sort_by(|left, right| {
        left.request
            .approval
            .issuer
            .as_slice()
            .cmp(right.request.approval.issuer.as_slice())
    });
    commit(record, &registry)?;
    Ok(PortableIssuerPreparationState::Pending(
        PortableIssuerPreparation {
            domain,
            issuer,
            sequence,
            digest,
        },
    ))
}

/// First matching randomness completion wins; a late callback cannot replace intent.
pub fn finish(
    ticket: PortableIssuerPreparation,
    nonce: [u8; 32],
    now_ns: u64,
) -> Result<PortableIssuerChallenge, E> {
    let record = observe(now_ns)?;
    let mut registry = load_registry(&record)?;
    domain_matches(&registry, &ticket.domain)?;
    let registration = registry
        .registrations
        .iter_mut()
        .find(|r| r.request.approval.issuer == ticket.issuer)
        .ok_or(E::Conflict)?;
    if registration.sequence != ticket.sequence || registration.digest != ticket.digest {
        return Err(E::Conflict);
    }
    if let Some(challenge) = &registration.challenge {
        return Ok(challenge.clone());
    }
    let challenge = challenge_for(&ticket.domain, registration, nonce, now_ns)?;
    registration.challenge = Some(challenge.clone());
    commit(record, &registry)?;
    Ok(challenge)
}

/// Caller is captured by the authenticated API, never deserialized from the proof.
pub fn register(
    domain: EnrollmentDomain,
    caller: Principal,
    request: RegisterPortableIssuer,
    now_ns: u64,
) -> Result<PortableIssuerEnrollment, E> {
    let record = observe(now_ns)?;
    let mut registry = load_registry(&record)?;
    domain_matches(&registry, &domain)?;
    let index = registry
        .registrations
        .iter()
        .position(|r| r.request.approval.issuer == caller)
        .ok_or(E::Binding)?;
    let registration = &registry.registrations[index];
    if registration.request.operation_id != request.operation_id {
        return Err(E::Conflict);
    }
    let challenge = registration.challenge.as_ref().ok_or(E::Unavailable)?;
    let key = PortableIssuerKey::Ed25519 {
        public_key: challenge.public_key,
    };
    if registry.retired_keys.binary_search(&key_id(&key)).is_ok() {
        return Err(E::Retired);
    }
    let public = VerifyingKey::from_bytes(&challenge.public_key).map_err(|_| E::Possession)?;
    let signature = Signature::from_slice(&request.signature).map_err(|_| E::Possession)?;
    public
        .verify_strict(
            &registration_message(challenge).map_err(|_| E::Capacity)?,
            &signature,
        )
        .map_err(|_| E::Possession)?;
    if let Some(result) = retained_result(&registry, registration) {
        return Ok(result);
    }
    if now_ns >= challenge.expires_at_ns {
        return Err(E::Expired);
    }
    let result = enrollment(&registration.request.approval);
    let mut entries = registry.entries.clone();
    entries.push(result.clone());
    entries.sort_by_key(|entry| key_id(&entry.key));
    registry = replace_registry(registry, domain, entries, Vec::new())?;
    commit(record, &registry)?;
    Ok(result)
}

/// Query-only reconciliation preserves original deadlines and performs no writes.
pub fn lookup(
    domain: EnrollmentDomain,
    caller: Principal,
    operation_id: [u8; 32],
    now_ns: u64,
) -> Result<PortableIssuerRegistrationStatus, E> {
    let record = RootPortableEnrollmentState::read();
    if now_ns < record.observed_at_ns {
        return Err(E::Clock);
    }
    let registry = load_registry(&record)?;
    domain_matches(&registry, &domain)?;
    let registration = registry
        .registrations
        .iter()
        .find(|r| r.request.approval.issuer == caller && r.request.operation_id == operation_id)
        .ok_or(E::Unavailable)?;
    let id = key_id(&PortableIssuerKey::Ed25519 {
        public_key: registration.request.approval.public_key,
    });
    if registry.retired_keys.binary_search(&id).is_ok() {
        return Err(E::Retired);
    }
    Ok(
        if let Some(result) = retained_result(&registry, registration) {
            PortableIssuerRegistrationStatus::Registered(result)
        } else if let Some(challenge) = &registration.challenge {
            PortableIssuerRegistrationStatus::Challenge(challenge.clone())
        } else {
            PortableIssuerRegistrationStatus::Preparing
        },
    )
}

/// Controller revocation cancels pending callbacks and terminally retires issued keys.
pub fn retire(domain: EnrollmentDomain, issuer: Principal, now_ns: u64) -> Result<(), E> {
    let record = observe(now_ns)?;
    let registry = load_registry(&record)?;
    domain_matches(&registry, &domain)?;
    let entries = registry
        .entries
        .iter()
        .filter(|entry| entry.issuer != issuer)
        .cloned()
        .collect();
    let mut registry = replace_registry(registry, domain, entries, Vec::new())?;
    registry
        .registrations
        .retain(|registration| registration.request.approval.issuer != issuer);
    commit(record, &registry)
}

#[cfg(test)]
mod tests;
