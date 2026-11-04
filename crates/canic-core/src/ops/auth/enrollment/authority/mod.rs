//! Module: ops::auth::enrollment::authority
//!
//! Responsibility: publish current approved registry snapshots and retain terminal retirements.
//! Boundary: authenticated registration and custody approval precede this adapter.

use crate::{
    model::auth::portable_registration::RootPortableEnrollmentRecord,
    ops::{
        auth::enrollment::{
            EnrollmentBinding, EnrollmentDomain, PortableEnrollmentError,
            validation::{key_id, validate_contents},
        },
        ic::IcOps,
        runtime::env::EnvOps,
        storage::fleet_activation::FleetActivationOps,
    },
    storage::stable::auth::enrollment::{PortableEnrollmentRecord, RootPortableEnrollmentState},
};
use canic_contracts::{
    dto::portable_enrollment::{
        PortableEnrollmentRequest, PortableEnrollmentSnapshot, PortableIssuerEnrollment,
    },
    ids::FleetKey,
    protocol::portable_enrollment as wire,
};
use ic_platform::Principal;
use std::collections::BTreeSet;

// The stable registry retains both current authority and exact registration intent.
// Its decoder budget is independent of the bounded single-response transport.
const JOURNAL_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 131_072,
    allocation: 4_194_304,
    nodes: 65_536,
    depth: 16,
};

pub(super) fn encode_registry(
    registry: &RootPortableEnrollmentRecord,
) -> Result<Vec<u8>, PortableEnrollmentError> {
    let bytes = ic_wire::native::encode(registry, JOURNAL_LIMITS)
        .map_err(|_| PortableEnrollmentError::Capacity)?;
    let _: RootPortableEnrollmentRecord = ic_wire::native::decode(&bytes, JOURNAL_LIMITS)
        .map_err(|_| PortableEnrollmentError::Capacity)?;
    Ok(bytes)
}

/// Internal Root snapshot adapter; authenticated registration owns all mutations.
pub struct RootPortableEnrollmentOps;
impl RootPortableEnrollmentOps {
    /// Issue for the actual IC caller. Freshness is protected operator policy.
    pub fn snapshot(
        request: PortableEnrollmentRequest,
        freshness_ns: u64,
    ) -> Result<PortableEnrollmentSnapshot, PortableEnrollmentError> {
        let (authority, fleet) = protected_root()?;
        issue_snapshot(
            EnrollmentBinding {
                authority,
                receiver: IcOps::msg_caller().into(),
                fleet,
            },
            request.boot_challenge,
            IcOps::now_nanos(),
            freshness_ns,
        )
    }
}

pub(super) fn protected_root() -> Result<(Principal, FleetKey), PortableEnrollmentError> {
    let root: Principal = EnvOps::root_pid()
        .map_err(|_| PortableEnrollmentError::Unavailable)?
        .into();
    if root != IcOps::canister_self().into() {
        return Err(PortableEnrollmentError::Binding);
    }
    let fleet = FleetActivationOps::fleet_binding()
        .map_err(|_| PortableEnrollmentError::Unavailable)?
        .fleet;
    Ok((root, fleet))
}

pub(super) fn observe(now_ns: u64) -> Result<PortableEnrollmentRecord, PortableEnrollmentError> {
    let mut record = RootPortableEnrollmentState::read();
    if now_ns < record.observed_at_ns {
        return Err(PortableEnrollmentError::Clock);
    }
    record.observed_at_ns = now_ns;
    RootPortableEnrollmentState::write(record.clone());
    Ok(record)
}

#[cfg(test)]
pub(super) fn commit_registry(
    domain: EnrollmentDomain,
    entries: Vec<PortableIssuerEnrollment>,
    retire: Vec<[u8; 32]>,
    now_ns: u64,
) -> Result<(), PortableEnrollmentError> {
    let mut record = observe(now_ns)?;
    let previous = load_registry(&record)?;
    // The complete-registry bootstrap must not bypass a retained registration owner.
    if !previous.registrations.is_empty() {
        return Err(PortableEnrollmentError::Conflict);
    }
    let next = replace_registry(previous, domain, entries, retire)?;
    record.bytes = encode_registry(&next)?;
    RootPortableEnrollmentState::write(record);
    Ok(())
}

pub(super) fn load_registry(
    record: &PortableEnrollmentRecord,
) -> Result<RootPortableEnrollmentRecord, PortableEnrollmentError> {
    let registry = if record.bytes.is_empty() {
        RootPortableEnrollmentRecord::default()
    } else {
        ic_wire::native::decode(&record.bytes, JOURNAL_LIMITS)
            .map_err(|_| PortableEnrollmentError::Storage)?
    };
    if registry.domain.is_none() && registry != RootPortableEnrollmentRecord::default() {
        return Err(PortableEnrollmentError::Storage);
    }
    if registry
        .registrations
        .iter()
        .filter_map(|registration| registration.challenge.as_ref())
        .any(|challenge| challenge.issued_at_ns > record.observed_at_ns)
    {
        return Err(PortableEnrollmentError::Storage);
    }
    validate_contents(&registry.entries, &registry.retired_keys)?;
    crate::ops::auth::enrollment::registration::validate_records(&registry)?;
    Ok(registry)
}

pub(super) fn replace_registry(
    mut previous: RootPortableEnrollmentRecord,
    domain: EnrollmentDomain,
    entries: Vec<PortableIssuerEnrollment>,
    retire: Vec<[u8; 32]>,
) -> Result<RootPortableEnrollmentRecord, PortableEnrollmentError> {
    if previous
        .domain
        .as_ref()
        .is_some_and(|previous| previous != &domain)
    {
        return Err(PortableEnrollmentError::Binding);
    }
    validate_contents(&previous.entries, &previous.retired_keys)?;
    validate_contents(&entries, &[])?;
    if retire.len() > wire::MAX_RETIRED_KEYS {
        return Err(PortableEnrollmentError::Capacity);
    }
    let mut retired = previous
        .retired_keys
        .iter()
        .copied()
        .chain(retire)
        .collect::<BTreeSet<_>>();
    let active = entries
        .iter()
        .map(|entry| key_id(&entry.key))
        .collect::<BTreeSet<_>>();
    for entry in &previous.entries {
        let id = key_id(&entry.key);
        if !active.contains(&id) {
            retired.insert(id);
        }
    }
    let retired_keys = retired.into_iter().collect::<Vec<_>>();
    validate_contents(&entries, &retired_keys)?;
    if entries == previous.entries
        && retired_keys == previous.retired_keys
        && previous.domain.is_some()
    {
        return Ok(previous);
    }
    previous.domain = Some(domain);
    previous.revision = previous
        .revision
        .checked_add(1)
        .ok_or(PortableEnrollmentError::Capacity)?;
    previous.generation = previous
        .generation
        .checked_add(1)
        .ok_or(PortableEnrollmentError::Capacity)?;
    previous.entries = entries;
    previous.retired_keys = retired_keys;
    Ok(previous)
}

pub(super) fn issue_snapshot(
    binding: EnrollmentBinding,
    challenge: [u8; 32],
    now_ns: u64,
    freshness_ns: u64,
) -> Result<PortableEnrollmentSnapshot, PortableEnrollmentError> {
    let mut record = observe(now_ns)?;
    if record.bytes.is_empty() {
        return Err(PortableEnrollmentError::Unavailable);
    }
    if binding.authority == Principal::anonymous()
        || binding.receiver == Principal::anonymous()
        || challenge == [0; 32]
        || freshness_ns == 0
        || freshness_ns > wire::MAX_FRESHNESS_NS
    {
        return Err(PortableEnrollmentError::Invalid);
    }
    let mut registry = load_registry(&record)?;
    if registry.domain
        != Some(EnrollmentDomain {
            authority: binding.authority,
            fleet: binding.fleet,
        })
    {
        return Err(PortableEnrollmentError::Binding);
    }
    validate_contents(&registry.entries, &registry.retired_keys)?;
    registry.revision = registry
        .revision
        .checked_add(1)
        .ok_or(PortableEnrollmentError::Capacity)?;
    let snapshot = PortableEnrollmentSnapshot {
        authority: binding.authority,
        receiver: binding.receiver,
        fleet: binding.fleet,
        boot_challenge: challenge,
        revision: registry.revision,
        generation: registry.generation,
        issued_at_ns: now_ns,
        expires_at_ns: now_ns
            .checked_add(freshness_ns)
            .ok_or(PortableEnrollmentError::Capacity)?,
        entries: registry.entries.clone(),
        retired_keys: registry.retired_keys.clone(),
    };
    let response: Result<_, &crate::dto::error::Error> = Ok(&snapshot);
    ic_wire::native::encode_response(wire::METHOD_ID, &response, wire::RESPONSE_LIMITS)
        .map_err(|_| PortableEnrollmentError::Capacity)?;
    record.bytes = encode_registry(&registry)?;
    RootPortableEnrollmentState::write(record);
    Ok(snapshot)
}
