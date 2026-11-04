//! Exact authenticated issuance intent and lookup-only recovery.
//!
//! Application permission checks precede this owner. IC Auth owns claims encoding,
//! grant rules and time validation; the issuer retains only public signed results.

mod budget;
mod retention;
#[cfg(test)]
mod tests;

use crate::{
    model::auth::portable_issuer::{
        PortableIssuancePolicy, PortableIssuancePreparation, PortableIssuanceRecord,
        PortableIssuanceTicket, PortableIssuerDomain, PortableIssuerJournalRecord,
    },
    ops::auth::{
        enrollment::{EnrollmentBinding, PortableEnrollmentError as KeyError, validation},
        portable_issuer::{KEY, commit, current_record, load, require_key, ticket},
    },
    storage::stable::auth::enrollment::PortableIssuerState,
};
use canic_contracts::{
    dto::{
        portable_enrollment::{PortableEnrollmentSnapshot, PortableIssuerKey},
        portable_issuance::{
            IssuePortableCredential, IssuedPortableCredential,
            PortableCredentialIssuanceRefusal as E, PortableCredentialIssuanceStatus,
            PortableIssuanceContext, PortableIssuanceMaintenance,
        },
    },
    protocol::portable_enrollment,
};
use ic_auth::portable::{PortableAuthority, PortableClaimsContext};
use ic_auth_protocol_types::{
    AudienceId, CanonicalId, PortableClaims, PortableCredential, PortableMechanism, PortableProof,
};
use ic_platform::{
    Principal,
    ed25519::{Signer, VerifyingKey},
};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub(super) const MAX_ISSUANCES: usize = 32;
pub(super) use retention::MAX_LOGINS;

/// Production activation remains closed until the maintainer qualifies its clock.
#[cfg(feature = "auth-portable-issuer")]
pub fn require_local() -> Result<(), E> {
    let network = crate::ops::config::ConfigOps::delegated_tokens_config()
        .map_err(|_| E::Unavailable)?
        .build_network;
    if crate::domain::policy::pure::auth::portable_clock_is_qualified(network) {
        Ok(())
    } else {
        Err(E::Unavailable)
    }
}

impl From<KeyError> for E {
    fn from(error: KeyError) -> Self {
        match error {
            KeyError::Binding => Self::Binding,
            KeyError::Capacity => Self::Capacity,
            KeyError::Clock => Self::Clock,
            KeyError::Conflict => Self::Conflict,
            KeyError::Expired => Self::Expired,
            KeyError::Invalid | KeyError::Possession => Self::Invalid,
            KeyError::Storage | KeyError::Rollback | KeyError::Generation => Self::Storage,
            KeyError::Transport => Self::Transport,
            KeyError::Unavailable | KeyError::Retired | KeyError::Custody => Self::Unavailable,
        }
    }
}

fn validate_request(
    caller: Principal,
    request: &IssuePortableCredential,
    policy: PortableIssuancePolicy,
) -> Result<(), E> {
    if caller == Principal::anonymous() {
        return Err(E::Denied);
    }
    if request.operation_id == [0; 32]
        || request.epoch == 0
        || request.login_epoch == 0
        || request.login_epoch > request.epoch
        || request.login_id == [0; 32]
        || policy.login_ttl_ns == 0
        || policy.proof_ttl_ns == 0
        || policy.receipt_timeout_ns == 0
        || policy.reconciliation_retention_ns == 0
        || policy.receipt_timeout_ns > portable_enrollment::MAX_FRESHNESS_NS
    {
        return Err(E::Invalid);
    }
    let key = VerifyingKey::from_bytes(&request.browser_public_key).map_err(|_| E::Invalid)?;
    if key.is_weak() {
        return Err(E::Invalid);
    }
    validation::grants(&request.grants)?;
    Ok(())
}

fn observe(now_ns: u64) -> Result<(), E> {
    let mut checkpoint = PortableIssuerState::read();
    if now_ns < checkpoint.observed_at_ns {
        return Err(E::Clock);
    }
    checkpoint.observed_at_ns = now_ns;
    PortableIssuerState::write(checkpoint);
    Ok(())
}

/// Retained rows must be complete current records before lookup can return them.
pub(super) fn validate_journal(
    journal: &PortableIssuerJournalRecord,
    observed_at_ns: u64,
) -> Result<(), KeyError> {
    let mut operations = BTreeSet::new();
    let mut logins = BTreeSet::new();
    for login in &journal.logins {
        if !logins.insert(login.binding.clone())
            || login.binding.caller == Principal::anonymous()
            || login.binding.epoch == 0
            || login.binding.epoch > journal.issuance_epoch
            || login.binding.login_id == [0; 32]
            || login.started_at_ns > observed_at_ns
            || login.started_at_ns >= login.deadline_ns
        {
            return Err(KeyError::Storage);
        }
    }
    for row in &journal.issuances {
        let index = usize::try_from(row.key.sequence)
            .ok()
            .and_then(|index| index.checked_sub(1));
        let key = index
            .and_then(|index| journal.keys.get(index))
            .ok_or(KeyError::Storage)?;
        let invalid_key = journal.domain.as_ref() != Some(&row.key.domain)
            || key.boot != row.key.boot
            || key.request.operation_id != row.key.operation_id
            || key.public_key.is_none();
        if invalid_key
            || validate_request(row.caller, &row.request, row.policy).is_err()
            || row.request.epoch > journal.issuance_epoch
            || !operations.insert((row.caller, row.request.epoch, row.request.operation_id))
            || row.login_started_at_ns > row.issued_at_ns
            || row.issued_at_ns > observed_at_ns
            || row.login_started_at_ns >= row.login_deadline_ns
            || row.issued_at_ns >= row.admission_deadline_ns
            || row.admission_deadline_ns > row.login_deadline_ns
            || row.receipt_deadline_ns <= row.issued_at_ns
            || row
                .admission_deadline_ns
                .max(row.receipt_deadline_ns)
                .checked_add(row.policy.reconciliation_retention_ns)
                != Some(row.remove_at_ns)
            || row.challenge == Some([0; 32])
            || (row.result.is_some() && row.challenge.is_none())
        {
            return Err(KeyError::Storage);
        }
        let login = journal
            .logins
            .iter()
            .find(|login| retention::references(row, login))
            .ok_or(KeyError::Storage)?;
        if login.browser_public_key != row.request.browser_public_key
            || login.started_at_ns != row.login_started_at_ns
            || login.deadline_ns != row.login_deadline_ns
        {
            return Err(KeyError::Storage);
        }
        if let Some(result) = &row.result {
            let credential: PortableCredential = result
                .credential
                .clone()
                .try_into()
                .map_err(|_| KeyError::Storage)?;
            let expected = claims(row, key.public_key.ok_or(KeyError::Storage)?)
                .map_err(|_| KeyError::Storage)?;
            let signature_valid_shape = matches!(&credential.proof, PortableProof::Ed25519 { signature } if signature.len() == 64);
            if result.admission_deadline_ns != row.admission_deadline_ns
                || !signature_valid_shape
                || credential.claims
                    != ic_auth::portable::encode_claims(&expected).map_err(|_| KeyError::Storage)?
            {
                return Err(KeyError::Storage);
            }
        }
    }
    Ok(())
}

/// Reserve exact caller/login/request/key/deadlines before randomness or Root effects.
pub fn begin(
    domain: PortableIssuerDomain,
    caller: Principal,
    request: IssuePortableCredential,
    policy: PortableIssuancePolicy,
    now_ns: u64,
) -> Result<PortableIssuancePreparation, E> {
    observe(now_ns)?;
    validate_request(caller, &request, policy)?;
    let mut journal = load()?;
    if journal.domain.as_ref() != Some(&domain) {
        return Err(E::Binding);
    }
    if let Some(row) = journal.issuances.iter().find(|row| {
        row.caller == caller
            && row.request.epoch == request.epoch
            && row.request.operation_id == request.operation_id
    }) {
        if row.request != request || row.policy != policy {
            return Err(E::Conflict);
        }
        return if let Some(result) = &row.result {
            Ok(PortableIssuancePreparation::Ready(result.clone()))
        } else {
            let pending = issue_ticket(row);
            check_pending(&mut journal, &pending, now_ns)?;
            Ok(PortableIssuancePreparation::Pending(pending))
        };
    }
    if request.epoch != journal.issuance_epoch {
        return Err(E::EpochClosed);
    }
    let (login, insert_login) =
        retention::select_login(&journal, caller, &request, policy, now_ns)?;
    // The incoming current-epoch request was absent from complete retained history.
    // It may be admitted in the same transaction that closes that epoch.
    retention::maintain(&mut journal, now_ns)?;
    if journal.issuances.len() == MAX_ISSUANCES
        || (insert_login && journal.logins.len() == MAX_LOGINS)
    {
        return Err(E::Capacity);
    }
    let sequence = journal.active_sequence.ok_or(E::Unavailable)?;
    let key_operation = journal.keys.last().ok_or(E::Storage)?.request.operation_id;
    let key = ticket(&domain, &journal, sequence, key_operation);
    let key_row = current_record(&mut journal, &key)?;
    let public_key = key_row.public_key.ok_or(E::Unavailable)?;
    require_key(&key, public_key)?;
    let enrollment = key_row.enrollment.as_ref().ok_or(E::Unavailable)?;
    if now_ns < enrollment.not_before_ns || now_ns >= enrollment.expires_at_ns {
        return Err(E::Expired);
    }
    let authority_deadline = enrollment
        .expires_at_ns
        .min(enrollment.deployment_approval_expires_at_ns);
    let login_started_at_ns = login.started_at_ns;
    let login_deadline_ns = login.deadline_ns;
    let admission_deadline_ns = now_ns
        .checked_add(policy.proof_ttl_ns)
        .ok_or(E::Capacity)?
        .min(login_deadline_ns)
        .min(authority_deadline);
    if admission_deadline_ns <= now_ns {
        return Err(E::Expired);
    }
    let receipt_deadline_ns = now_ns
        .checked_add(policy.receipt_timeout_ns)
        .ok_or(E::Capacity)?;
    let remove_at_ns = admission_deadline_ns
        .max(receipt_deadline_ns)
        .checked_add(policy.reconciliation_retention_ns)
        .ok_or(E::Capacity)?;
    let row = PortableIssuanceRecord {
        caller,
        request,
        policy,
        key,
        login_started_at_ns,
        login_deadline_ns,
        issued_at_ns: now_ns,
        admission_deadline_ns,
        receipt_deadline_ns,
        remove_at_ns,
        challenge: None,
        result: None,
    };
    let pending = issue_ticket(&row);
    if insert_login {
        journal.logins.push(login);
    }
    journal.issuances.push(row);
    budget::admit_completions(&journal)?;
    commit(&journal)?;
    Ok(PortableIssuancePreparation::Pending(pending))
}

fn issue_ticket(row: &PortableIssuanceRecord) -> PortableIssuanceTicket {
    PortableIssuanceTicket {
        caller: row.caller,
        epoch: row.request.epoch,
        operation_id: row.request.operation_id,
        key: row.key.clone(),
    }
}

fn row_index(
    journal: &PortableIssuerJournalRecord,
    pending: &PortableIssuanceTicket,
) -> Result<usize, E> {
    journal
        .issuances
        .iter()
        .position(|row| issue_ticket(row) == *pending)
        .ok_or(E::Conflict)
}

fn check_pending(
    journal: &mut PortableIssuerJournalRecord,
    pending: &PortableIssuanceTicket,
    now_ns: u64,
) -> Result<usize, E> {
    let key = current_record(journal, &pending.key)?;
    require_key(&pending.key, key.public_key.ok_or(E::Unavailable)?)?;
    let index = row_index(journal, pending)?;
    let row = &journal.issuances[index];
    if now_ns >= row.admission_deadline_ns || now_ns >= row.receipt_deadline_ns {
        return Err(E::Expired);
    }
    Ok(index)
}

/// Read a live intent's committed nonce without changing its clock or deadlines.
pub fn retained_challenge(
    pending: &PortableIssuanceTicket,
    now_ns: u64,
) -> Result<Option<[u8; 32]>, E> {
    if now_ns < PortableIssuerState::read().observed_at_ns {
        return Err(E::Clock);
    }
    let mut journal = load()?;
    let index = check_pending(&mut journal, pending, now_ns)?;
    Ok(journal.issuances[index].challenge)
}

/// First nonce completion wins; duplicate callbacks reuse the exact original nonce.
pub fn challenge(
    pending: &PortableIssuanceTicket,
    randomness: [u8; 32],
    now_ns: u64,
) -> Result<[u8; 32], E> {
    observe(now_ns)?;
    if randomness == [0; 32] {
        return Err(E::Invalid);
    }
    let mut journal = load()?;
    let index = check_pending(&mut journal, pending, now_ns)?;
    let challenge = *journal.issuances[index].challenge.get_or_insert(randomness);
    commit(&journal)?;
    Ok(challenge)
}

/// Fetch only the installed Root for one still-live retained intent.
#[cfg(feature = "auth-portable-issuer")]
pub async fn fetch_authority(
    pending: &PortableIssuanceTicket,
) -> Result<PortableEnrollmentSnapshot, E> {
    let mut journal = load()?;
    let index = check_pending(&mut journal, pending, crate::ops::ic::IcOps::now_nanos())?;
    let challenge = journal.issuances[index].challenge.ok_or(E::Unavailable)?;
    crate::ops::auth::enrollment::fetch_snapshot(pending.key.domain.authority, challenge)
        .await
        .map_err(Into::into)
}

fn claims(row: &PortableIssuanceRecord, public_key: [u8; 32]) -> Result<PortableClaims, E> {
    let fleet = row.key.domain.fleet;
    let challenge = row.challenge.ok_or(E::Unavailable)?;
    let mut nonce = [0; 16];
    nonce.copy_from_slice(&challenge[..16]);
    Ok(PortableClaims {
        mechanism: PortableMechanism::Ed25519,
        audience: AudienceId {
            canonical_network_id: CanonicalId::from_bytes(*fleet.canonical_network_id.as_bytes()),
            fleet_id: CanonicalId::from_bytes(*fleet.fleet_id.as_bytes()),
        },
        issuer: row.key.domain.issuer,
        key_binding_id: Sha256::digest(public_key).into(),
        browser_public_key: row.request.browser_public_key,
        account_subject: row.caller,
        login_started_at_ns: row.login_started_at_ns,
        login_deadline_ns: row.login_deadline_ns,
        issued_at_ns: row.issued_at_ns,
        admission_expires_at_ns: row.admission_deadline_ns,
        nonce,
        grants: validation::grants(&row.request.grants)?.into_vec(),
    })
}

/// Retain authenticated Root progress even when permission or signing subsequently refuses.
pub fn accept_authority(
    pending: &PortableIssuanceTicket,
    snapshot: PortableEnrollmentSnapshot,
    now_ns: u64,
) -> Result<(), E> {
    observe(now_ns)?;
    let mut journal = load()?;
    let index = check_pending(&mut journal, pending, now_ns)?;
    let row = &journal.issuances[index];
    let domain = &pending.key.domain;
    validation::validate_snapshot(
        &snapshot,
        EnrollmentBinding {
            authority: domain.authority,
            receiver: domain.issuer,
            fleet: domain.fleet,
        },
        row.challenge.ok_or(E::Unavailable)?,
        now_ns,
        row.receipt_deadline_ns,
    )?;
    if let Some(retained) = &journal.enrollment {
        validation::validate_progress(retained, &snapshot)?;
    }
    journal.enrollment = Some(snapshot);
    if let Err(error) = commit(&journal) {
        // An authenticated authority observation must never be forgotten while signing stays live.
        KEY.with_borrow_mut(|key| *key = None);
        return Err(error.into());
    }
    Ok(())
}

/// Complete once under retained fresh Root authority; retries never re-sign or extend.
pub fn finish(
    pending: &PortableIssuanceTicket,
    now_ns: u64,
) -> Result<IssuedPortableCredential, E> {
    observe(now_ns)?;
    let mut journal = load()?;
    let original_index = row_index(&journal, pending)?;
    if let Some(result) = &journal.issuances[original_index].result {
        return Ok(result.clone());
    }
    let index = check_pending(&mut journal, pending, now_ns)?;
    let row = &journal.issuances[index];
    let domain = &pending.key.domain;
    let snapshot = journal.enrollment.as_ref().ok_or(E::Unavailable)?;
    validation::validate_snapshot(
        snapshot,
        EnrollmentBinding {
            authority: domain.authority,
            receiver: domain.issuer,
            fleet: domain.fleet,
        },
        row.challenge.ok_or(E::Unavailable)?,
        now_ns,
        row.receipt_deadline_ns,
    )?;
    let key_row = journal
        .keys
        .get(usize::try_from(pending.key.sequence - 1).map_err(|_| E::Storage)?)
        .ok_or(E::Storage)?;
    let public_key = key_row.public_key.ok_or(E::Unavailable)?;
    let entry = snapshot
        .entries
        .iter()
        .find(|entry| {
            entry.issuer == domain.issuer
                && entry.key == (PortableIssuerKey::Ed25519 { public_key })
        })
        .ok_or(E::Unavailable)?;
    let claims = claims(row, public_key)?;
    let grants = validation::grants(&entry.grants)?;
    let context = PortableClaimsContext {
        authority: PortableAuthority {
            audience: &claims.audience,
            issuer: domain.issuer,
            mechanism: PortableMechanism::Ed25519,
            key_binding_id: claims.key_binding_id,
            grants: &grants,
            caller: ic_auth::portable::browser_principal(&claims.browser_public_key),
            now_ns,
            authority_not_before_ns: entry.not_before_ns,
            authority_expires_at_ns: entry
                .expires_at_ns
                .min(entry.deployment_approval_expires_at_ns),
            enrollment_expires_at_ns: snapshot.expires_at_ns,
            min_login_started_at_ns: 0,
        },
        min_proof_issued_at_ns: 0,
        max_login_ttl_ns: row.policy.login_ttl_ns,
        max_proof_ttl_ns: row.policy.proof_ttl_ns,
    };
    ic_auth::portable::validate_claims_context(&claims, &context).map_err(|_| E::Denied)?;
    if row.admission_deadline_ns > snapshot.expires_at_ns
        || row.admission_deadline_ns > entry.expires_at_ns
    {
        return Err(E::Expired);
    }
    let encoded = ic_auth::portable::encode_claims(&claims).map_err(|_| E::Invalid)?;
    budget::admit_completions(&journal)?;
    let hash = ic_auth::portable::claims_hash(&claims).map_err(|_| E::Invalid)?;
    let signature = KEY.with_borrow(|key| {
        key.as_ref()
            .filter(|key| key.ticket == pending.key)
            .map(|key| key.signing_key.sign(&hash).to_bytes().to_vec())
            .ok_or(E::Unavailable)
    })?;
    let result = wire_result(row, encoded, signature)?;
    journal.issuances[index].result = Some(result.clone());
    commit(&journal)?;
    Ok(result)
}

fn wire_result(
    row: &PortableIssuanceRecord,
    encoded: Vec<u8>,
    signature: Vec<u8>,
) -> Result<IssuedPortableCredential, E> {
    Ok(IssuedPortableCredential {
        credential: PortableCredential {
            claims: encoded,
            proof: PortableProof::Ed25519 { signature },
        }
        .try_into()
        .map_err(|_| E::Capacity)?,
        admission_deadline_ns: row.admission_deadline_ns,
    })
}

/// Read the actual account's original result without clock writes, nonce creation or signing.
pub fn reconcile(
    domain: &PortableIssuerDomain,
    caller: Principal,
    epoch: u64,
    operation_id: [u8; 32],
) -> Result<PortableCredentialIssuanceStatus, E> {
    if caller == Principal::anonymous() {
        return Err(E::Denied);
    }
    let journal = load()?;
    if journal
        .domain
        .as_ref()
        .is_some_and(|retained| retained != domain)
    {
        return Err(E::Binding);
    }
    Ok(journal
        .issuances
        .iter()
        .find(|row| {
            row.caller == caller
                && row.request.epoch == epoch
                && row.request.operation_id == operation_id
        })
        .and_then(|row| row.result.clone())
        .map_or(
            PortableCredentialIssuanceStatus::Unresolved,
            PortableCredentialIssuanceStatus::Issued,
        ))
}

/// Read-only discovery; the epoch is an operation identity, not permission to issue.
pub fn context(domain: &PortableIssuerDomain) -> Result<PortableIssuanceContext, E> {
    let journal = load()?;
    if journal
        .domain
        .as_ref()
        .is_some_and(|retained| retained != domain)
    {
        return Err(E::Binding);
    }
    if journal.issuance_epoch == 0 {
        return Err(E::Unavailable);
    }
    Ok(PortableIssuanceContext {
        epoch: journal.issuance_epoch,
    })
}

/// Reclaim only elapsed retention and expired unreferenced logins in one atomic update.
pub fn reclaim(
    domain: &PortableIssuerDomain,
    now_ns: u64,
) -> Result<PortableIssuanceMaintenance, E> {
    observe(now_ns)?;
    let mut journal = load()?;
    if journal
        .domain
        .as_ref()
        .is_some_and(|retained| retained != domain)
    {
        return Err(E::Binding);
    }
    let maintenance = retention::maintain(&mut journal, now_ns)?;
    commit(&journal)?;
    Ok(maintenance)
}
