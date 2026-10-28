//! Module: ops::auth::enrollment::authority
//!
//! Responsibility: publish current approved registry snapshots and retain terminal retirements.
//! Boundary: authenticated registration and custody approval precede this adapter.

use super::*;
use crate::{
    ops::{ic::IcOps, runtime::env::EnvOps, storage::fleet_activation::FleetActivationOps},
    storage::stable::auth::enrollment::{PortableEnrollmentRecord, RootPortableEnrollmentState},
};
use std::collections::BTreeSet;

/// Internal Root adapter. Registration/custody approval is not supplied here.
pub struct RootPortableEnrollmentOps;
impl RootPortableEnrollmentOps {
    /// Commit an already authenticated and governance-approved complete registry.
    /// No endpoint may expose this operation without the registration contract.
    pub fn commit_approved(
        entries: Vec<PortableIssuerEnrollment>,
        retire: Vec<[u8; 32]>,
    ) -> Result<(), PortableEnrollmentError> {
        let (authority, fleet) = protected_root()?;
        commit_registry(
            EnrollmentDomain { authority, fleet },
            entries,
            retire,
            IcOps::now_nanos(),
        )
    }

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

fn protected_root() -> Result<(Principal, FleetKey), PortableEnrollmentError> {
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

fn observe(now_ns: u64) -> Result<PortableEnrollmentRecord, PortableEnrollmentError> {
    let mut record = RootPortableEnrollmentState::read();
    if now_ns < record.observed_at_ns {
        return Err(PortableEnrollmentError::Clock);
    }
    record.observed_at_ns = now_ns;
    RootPortableEnrollmentState::write(record.clone());
    Ok(record)
}

pub(super) fn commit_registry(
    domain: EnrollmentDomain,
    entries: Vec<PortableIssuerEnrollment>,
    retire: Vec<[u8; 32]>,
    now_ns: u64,
) -> Result<(), PortableEnrollmentError> {
    let mut record = observe(now_ns)?;
    let previous: RegistryContents = if record.bytes.is_empty() {
        RegistryContents::default()
    } else {
        decode(&record.bytes)?
    };
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
        && !record.bytes.is_empty()
    {
        return Ok(());
    }
    let next = RegistryContents {
        domain: Some(domain),
        revision: previous
            .revision
            .checked_add(1)
            .ok_or(PortableEnrollmentError::Capacity)?,
        generation: previous
            .generation
            .checked_add(1)
            .ok_or(PortableEnrollmentError::Capacity)?,
        entries,
        retired_keys,
    };
    record.bytes = encode(&next)?;
    RootPortableEnrollmentState::write(record);
    Ok(())
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
    let mut registry: RegistryContents = decode(&record.bytes)?;
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
    record.bytes = encode(&registry)?;
    RootPortableEnrollmentState::write(record);
    Ok(snapshot)
}
