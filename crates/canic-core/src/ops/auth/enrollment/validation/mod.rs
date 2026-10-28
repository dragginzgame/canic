//! Module: ops::auth::enrollment::validation
//!
//! Responsibility: validate bounded enrollment shape using Auth-owned grant rules.
//! Boundary: this validates data, never its IC provenance or issuer key possession.

use super::*;
use ic_auth_protocol_types::{AuthRole, DelegatedRoleGrant, limits};

pub(super) fn key_id(key: &PortableIssuerKey) -> [u8; 32] {
    match key {
        PortableIssuerKey::Certified { certificate_hash } => *certificate_hash,
        PortableIssuerKey::Ed25519 { public_key } => Sha256::digest(public_key).into(),
    }
}

pub(super) fn validate_contents(
    entries: &[PortableIssuerEnrollment],
    retired_keys: &[[u8; 32]],
) -> Result<(), PortableEnrollmentError> {
    use PortableEnrollmentError as E;
    if entries.len() > wire::MAX_ENTRIES || retired_keys.len() > wire::MAX_RETIRED_KEYS {
        return Err(E::Capacity);
    }
    if retired_keys.contains(&[0; 32]) {
        return Err(E::Invalid);
    }
    if !retired_keys.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err(E::Invalid);
    }
    let mut previous = None;
    for entry in entries {
        let id = key_id(&entry.key);
        if entry.issuer == Principal::anonymous()
            || id == [0; 32]
            || previous.is_some_and(|previous| previous >= id)
            || entry.not_before_ns >= entry.expires_at_ns
            || entry.expires_at_ns > entry.deployment_approval_expires_at_ns
            || entry.deployment_approval_digest == [0; 32]
        {
            return Err(E::Invalid);
        }
        if retired_keys.binary_search(&id).is_ok() {
            return Err(E::Retired);
        }
        if entry.grants.len() > limits::MAX_GRANTS {
            return Err(E::Capacity);
        }
        if entry.grants.iter().any(|grant| {
            grant.scopes.len() > limits::MAX_SCOPES_PER_GRANT
                || grant
                    .scopes
                    .iter()
                    .any(|scope| scope.len() > limits::MAX_LABEL_BYTES)
        }) {
            return Err(E::Capacity);
        }
        let grants = entry
            .grants
            .iter()
            .map(|grant| {
                let target: AuthRole = grant.role.parse().map_err(|_| E::Invalid)?;
                Ok(DelegatedRoleGrant {
                    target,
                    scopes: grant.scopes.clone(),
                })
            })
            .collect::<Result<Vec<_>, E>>()?;
        ic_auth::certificate_issuance::validate_grants(
            &grants,
            limits::MAX_GRANTS,
            limits::MAX_SCOPES_PER_GRANT,
        )
        .map_err(|_| E::Invalid)?;
        previous = Some(id);
    }
    Ok(())
}

#[cfg(any(test, feature = "auth-local-application-authorization"))]
pub(super) fn validate_snapshot(
    snapshot: &PortableEnrollmentSnapshot,
    binding: EnrollmentBinding,
    challenge: [u8; 32],
    now_ns: u64,
    receipt_deadline_ns: u64,
) -> Result<(), PortableEnrollmentError> {
    use PortableEnrollmentError as E;
    if snapshot.authority != binding.authority
        || snapshot.receiver != binding.receiver
        || snapshot.fleet != binding.fleet
        || snapshot.boot_challenge != challenge
    {
        return Err(E::Binding);
    }
    if snapshot.revision == 0
        || snapshot.issued_at_ns > now_ns
        || snapshot.issued_at_ns >= snapshot.expires_at_ns
        || snapshot.expires_at_ns - snapshot.issued_at_ns > wire::MAX_FRESHNESS_NS
    {
        return Err(E::Invalid);
    }
    if now_ns >= snapshot.expires_at_ns || now_ns >= receipt_deadline_ns {
        return Err(E::Expired);
    }
    validate_contents(&snapshot.entries, &snapshot.retired_keys)
}
