//! Issuer-local volatile key custody and bounded public preparation journal.
//!
//! Only authenticated workflows select an operation. No secret enters stable
//! checkpoints, logs or responses through this owner. IC heap snapshots require
//! separately qualified deployment fencing before production issuance is enabled.

pub mod issuance;
mod receipt;
#[cfg(test)]
mod tests;

#[cfg(feature = "auth-portable-issuer")]
use crate::ops::{
    config::ConfigOps, ic::IcOps, runtime::env::EnvOps,
    storage::fleet_activation::FleetActivationOps,
};
use crate::{
    model::auth::portable_issuer::{
        PortableIssuerDomain, PortableIssuerJournalRecord, PortableIssuerKeyPreparation,
        PortableIssuerKeyRecord, PortableIssuerKeyTicket,
    },
    ops::auth::enrollment::PortableEnrollmentError as E,
    storage::stable::auth::enrollment::{PortableEnrollmentRecord, PortableIssuerState},
};
#[cfg(feature = "auth-portable-issuer")]
use canic_contracts::protocol::portable_enrollment::registration as wire;
use canic_contracts::{
    dto::portable_enrollment::{
        PortableIssuerChallenge, PortableIssuerCustody, PortableIssuerEnrollment,
        PortableIssuerKey, PortableIssuerPreparedKey, PreparePortableIssuerKey,
        RegisterPortableIssuer,
    },
    ops::portable_enrollment::registration_message,
};
#[cfg(feature = "auth-portable-issuer")]
use ic_platform::call::{Call, WaitPolicy};
use ic_platform::ed25519::{Signer, SigningKey};
use std::cell::RefCell;

const MAX_KEYS: usize = 64;
const JOURNAL_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 131_072,
    allocation: 4_194_304,
    nodes: 65_536,
    depth: 16,
};

/// Read installed authority and issuer role; request fields cannot choose this domain.
#[cfg(feature = "auth-portable-issuer")]
pub fn current_domain() -> Result<PortableIssuerDomain, E> {
    if !IcOps::in_replicated_execution() {
        return Err(E::Unavailable);
    }
    protected_domain()
}

/// Read protected installation identity without requiring an update or opening signing.
#[cfg(feature = "auth-portable-issuer")]
pub fn protected_domain() -> Result<PortableIssuerDomain, E> {
    FleetActivationOps::require_active(false).map_err(|_| E::Unavailable)?;
    if !ConfigOps::current_canister()
        .map_err(|_| E::Unavailable)?
        .auth
        .delegated_token_issuer
    {
        return Err(E::Binding);
    }
    Ok(PortableIssuerDomain {
        authority: EnvOps::root_pid().map_err(|_| E::Unavailable)?.into(),
        fleet: FleetActivationOps::fleet_binding()
            .map_err(|_| E::Unavailable)?
            .fleet,
        issuer: IcOps::canister_self().into(),
    })
}

/// Complete the one bounded Root call through Canic's Platform execution adapter.
#[cfg(feature = "auth-portable-issuer")]
pub async fn complete_at_root(
    domain: &PortableIssuerDomain,
    request: RegisterPortableIssuer,
) -> Result<
    Result<
        PortableIssuerEnrollment,
        canic_contracts::dto::portable_enrollment::PortableIssuerRegistrationRefusal,
    >,
    E,
> {
    root_call(
        domain,
        wire::complete::METHOD,
        wire::complete::METHOD_ID,
        &request,
    )
    .await
}

/// Root lookup is an authenticated update with read-only registration semantics.
#[cfg(feature = "auth-portable-issuer")]
pub async fn lookup_at_root(
    domain: &PortableIssuerDomain,
    request: canic_contracts::dto::portable_enrollment::PortableIssuerRegistrationLookup,
) -> Result<
    Result<
        canic_contracts::dto::portable_enrollment::PortableIssuerRegistrationStatus,
        canic_contracts::dto::portable_enrollment::PortableIssuerRegistrationRefusal,
    >,
    E,
> {
    root_call(
        domain,
        wire::lookup::METHOD,
        wire::lookup::METHOD_ID,
        &request,
    )
    .await
}

#[cfg(feature = "auth-portable-issuer")]
async fn root_call<T: serde::Serialize + Sync, R: serde::de::DeserializeOwned>(
    domain: &PortableIssuerDomain,
    method: &str,
    method_id: u64,
    request: &T,
) -> Result<R, E> {
    let bytes = ic_wire::native::encode_request(method_id, request, wire::REQUEST_LIMITS)
        .map_err(|_| E::Capacity)?;
    let response = crate::ops::ic::call::execute(
        Call::new(
            domain.authority,
            method,
            WaitPolicy::Bounded {
                timeout_seconds: 30,
            },
        )
        .take_raw_args(bytes),
    )
    .await
    .map_err(|_| E::Transport)?;
    ic_wire::native::decode_response(method_id, &response.into_bytes(), wire::RESPONSE_LIMITS)
        .map_err(|_| E::Transport)
}

// Deliberately neither Clone, Debug nor serializable. SigningKey owns secret drop.
struct VolatileKey {
    ticket: PortableIssuerKeyTicket,
    signing_key: SigningKey,
}

thread_local! {
    static KEY: RefCell<Option<VolatileKey>> = const { RefCell::new(None) };
}

pub(super) fn load() -> Result<PortableIssuerJournalRecord, E> {
    let checkpoint = PortableIssuerState::read();
    let journal: PortableIssuerJournalRecord = if checkpoint.bytes.is_empty() {
        PortableIssuerJournalRecord::default()
    } else {
        ic_wire::native::decode(&checkpoint.bytes, JOURNAL_LIMITS).map_err(|_| E::Storage)?
    };
    if journal.keys.len() > MAX_KEYS
        || journal.issuances.len() > issuance::MAX_ISSUANCES
        || journal.logins.len() > issuance::MAX_LOGINS
        || (journal.boot != 0 && journal.issuance_epoch == 0)
        || journal.active_sequence.is_some_and(|sequence| {
            sequence == 0 || usize::try_from(sequence).ok() != Some(journal.keys.len())
        })
        || (!journal.keys.is_empty() && journal.domain.is_none())
    {
        return Err(E::Storage);
    }
    let mut operations = std::collections::BTreeSet::new();
    for record in &journal.keys {
        if record.boot > journal.boot
            || record.request.operation_id == [0; 32]
            || record.request.custody != PortableIssuerCustody::TrustedVolatileLocal
            || !operations.insert(record.request.operation_id)
            || (record.challenge.is_some() && record.public_key.is_none())
            || (record.enrollment.is_some() && record.challenge.is_none())
            || record
                .enrollment
                .as_ref()
                .is_some_and(|entry| !receipt::valid_window(entry))
        {
            return Err(E::Storage);
        }
    }
    issuance::validate_journal(&journal, checkpoint.observed_at_ns)?;
    Ok(journal)
}

pub(super) fn encode_journal(journal: &PortableIssuerJournalRecord) -> Result<Vec<u8>, E> {
    let bytes = ic_wire::native::encode(journal, JOURNAL_LIMITS).map_err(|_| E::Capacity)?;
    // Encoder budgets alone do not establish that the retained shape can be read.
    let _: PortableIssuerJournalRecord =
        ic_wire::native::decode(&bytes, JOURNAL_LIMITS).map_err(|_| E::Capacity)?;
    Ok(bytes)
}

pub(super) fn commit(journal: &PortableIssuerJournalRecord) -> Result<(), E> {
    let bytes = encode_journal(journal)?;
    PortableIssuerState::write(PortableEnrollmentRecord {
        bytes,
        observed_at_ns: PortableIssuerState::read().observed_at_ns,
    });
    Ok(())
}

/// Clear signing authority synchronously before hooks or deferred work.
/// Original operation IDs remain consumed even when a pending key was never returned.
pub fn restore() -> Result<(), E> {
    KEY.with_borrow_mut(|key| *key = None);
    let mut journal = load()?;
    journal.boot = journal.boot.checked_add(1).ok_or(E::Capacity)?;
    journal.issuance_epoch = journal.issuance_epoch.checked_add(1).ok_or(E::Capacity)?;
    journal.active_sequence = None;
    commit(&journal)
}

/// Persist the exact public intent before requesting randomness.
pub fn begin(
    domain: PortableIssuerDomain,
    request: PreparePortableIssuerKey,
) -> Result<PortableIssuerKeyPreparation, E> {
    if request.custody != PortableIssuerCustody::TrustedVolatileLocal {
        return Err(E::Custody);
    }
    if request.operation_id == [0; 32] {
        return Err(E::Invalid);
    }
    let mut journal = load()?;
    if journal.boot == 0 {
        return Err(E::Unavailable);
    }
    if journal
        .domain
        .as_ref()
        .is_some_and(|previous| previous != &domain)
    {
        return Err(E::Binding);
    }
    if let Some(index) = journal
        .keys
        .iter()
        .position(|row| row.request.operation_id == request.operation_id)
    {
        let record = &journal.keys[index];
        if record.request != request {
            return Err(E::Conflict);
        }
        let sequence = u64::try_from(index + 1).map_err(|_| E::Capacity)?;
        if journal.active_sequence != Some(sequence) || record.boot != journal.boot {
            return Err(E::Unavailable);
        }
        let ticket = ticket(&domain, &journal, sequence, request.operation_id);
        return if let Some(public_key) = record.public_key {
            require_key(&ticket, public_key)?;
            Ok(PortableIssuerKeyPreparation::Ready(prepared(
                record, public_key,
            )))
        } else {
            Ok(PortableIssuerKeyPreparation::Pending(ticket))
        };
    }
    if journal.keys.len() == MAX_KEYS {
        return Err(E::Capacity);
    }
    journal.domain = Some(domain.clone());
    journal.keys.push(PortableIssuerKeyRecord {
        request: request.clone(),
        boot: journal.boot,
        public_key: None,
        challenge: None,
        enrollment: None,
    });
    let sequence = u64::try_from(journal.keys.len()).map_err(|_| E::Capacity)?;
    journal.active_sequence = Some(sequence);
    commit(&journal)?;
    KEY.with_borrow_mut(|key| *key = None);
    Ok(PortableIssuerKeyPreparation::Pending(ticket(
        &domain,
        &journal,
        sequence,
        request.operation_id,
    )))
}

fn ticket(
    domain: &PortableIssuerDomain,
    journal: &PortableIssuerJournalRecord,
    sequence: u64,
    operation_id: [u8; 32],
) -> PortableIssuerKeyTicket {
    PortableIssuerKeyTicket {
        domain: domain.clone(),
        boot: journal.boot,
        sequence,
        operation_id,
    }
}

fn current_record<'a>(
    journal: &'a mut PortableIssuerJournalRecord,
    ticket: &PortableIssuerKeyTicket,
) -> Result<&'a mut PortableIssuerKeyRecord, E> {
    if journal.domain.as_ref() != Some(&ticket.domain)
        || journal.boot != ticket.boot
        || journal.active_sequence != Some(ticket.sequence)
    {
        return Err(E::Conflict);
    }
    let index = usize::try_from(ticket.sequence).map_err(|_| E::Storage)?;
    let record = journal
        .keys
        .get_mut(index.checked_sub(1).ok_or(E::Storage)?)
        .ok_or(E::Storage)?;
    if record.request.operation_id != ticket.operation_id || record.boot != ticket.boot {
        return Err(E::Conflict);
    }
    Ok(record)
}

fn require_key(ticket: &PortableIssuerKeyTicket, public_key: [u8; 32]) -> Result<(), E> {
    KEY.with_borrow(|key| match key {
        Some(key)
            if key.ticket == *ticket
                && key.signing_key.verifying_key().to_bytes() == public_key =>
        {
            Ok(())
        }
        _ => Err(E::Unavailable),
    })
}

fn prepared(record: &PortableIssuerKeyRecord, public_key: [u8; 32]) -> PortableIssuerPreparedKey {
    PortableIssuerPreparedKey {
        operation_id: record.request.operation_id,
        custody: record.request.custody.clone(),
        public_key,
    }
}

/// First matching completion wins; late or restored callbacks cannot replace a key.
pub fn finish(
    ticket: PortableIssuerKeyTicket,
    seed: [u8; 32],
) -> Result<PortableIssuerPreparedKey, E> {
    if seed == [0; 32] {
        return Err(E::Invalid);
    }
    let mut journal = load()?;
    let record = current_record(&mut journal, &ticket)?;
    if let Some(public_key) = record.public_key {
        require_key(&ticket, public_key)?;
        return Ok(prepared(record, public_key));
    }
    let signing_key = SigningKey::from_bytes(&seed);
    let public_key = signing_key.verifying_key().to_bytes();
    if journal
        .keys
        .iter()
        .any(|row| row.public_key == Some(public_key))
    {
        return Err(E::Retired);
    }
    let record = current_record(&mut journal, &ticket)?;
    record.public_key = Some(public_key);
    let result = prepared(record, public_key);
    commit(&journal)?;
    KEY.with_borrow_mut(|key| {
        *key = Some(VolatileKey {
            ticket,
            signing_key,
        });
    });
    Ok(result)
}

/// Retain the exact Root challenge before signing; no generic signing surface exists.
pub fn registration(
    domain: PortableIssuerDomain,
    challenge: PortableIssuerChallenge,
    now_ns: u64,
) -> Result<(PortableIssuerKeyTicket, RegisterPortableIssuer), E> {
    let mut journal = load()?;
    let sequence = journal.active_sequence.ok_or(E::Unavailable)?;
    let ticket = ticket(&domain, &journal, sequence, challenge.operation_id);
    let record = current_record(&mut journal, &ticket)?;
    let public_key = record.public_key.ok_or(E::Unavailable)?;
    let challenge_domain = PortableIssuerDomain {
        authority: challenge.authority,
        fleet: challenge.fleet,
        issuer: challenge.issuer,
    };
    if challenge_domain != domain
        || challenge.public_key != public_key
        || challenge.nonce == [0; 32]
        || challenge.approval_digest == [0; 32]
        || challenge.issued_at_ns >= challenge.expires_at_ns
    {
        return Err(E::Binding);
    }
    if record
        .challenge
        .as_ref()
        .is_some_and(|original| original != &challenge)
    {
        return Err(E::Conflict);
    }
    // Retried bytes stay exact; Root decides whether original completion exists.
    if record.challenge.is_none()
        && (now_ns < challenge.issued_at_ns || now_ns >= challenge.expires_at_ns)
    {
        return Err(E::Expired);
    }
    require_key(&ticket, public_key)?;
    let message = registration_message(&challenge).map_err(|_| E::Capacity)?;
    record.challenge = Some(challenge);
    commit(&journal)?;
    let signature = KEY.with_borrow(|key| {
        key.as_ref()
            .ok_or(E::Unavailable)
            .map(|key| key.signing_key.sign(&message).to_bytes().to_vec())
    })?;
    Ok((
        ticket.clone(),
        RegisterPortableIssuer {
            operation_id: ticket.operation_id,
            signature,
        },
    ))
}

/// Commit only the original authenticated Root result for the still-live key.
pub fn registered(
    ticket: &PortableIssuerKeyTicket,
    result: PortableIssuerEnrollment,
) -> Result<PortableIssuerEnrollment, E> {
    let mut journal = load()?;
    let record = current_record(&mut journal, ticket)?;
    let public_key = record.public_key.ok_or(E::Unavailable)?;
    require_key(ticket, public_key)?;
    if record.challenge.is_none()
        || result.issuer != ticket.domain.issuer
        || result.key != (PortableIssuerKey::Ed25519 { public_key })
    {
        return Err(E::Binding);
    }
    crate::ops::auth::enrollment::validation::validate_contents(
        std::slice::from_ref(&result),
        &[],
    )?;
    let receipt = receipt::from_result(ticket, &result)?;
    if record
        .enrollment
        .as_ref()
        .is_some_and(|original| original != &receipt)
    {
        return Err(E::Conflict);
    }
    record.enrollment = Some(receipt);
    commit(&journal)?;
    Ok(result)
}
