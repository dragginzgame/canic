//! Module: ops::auth::enrollment::validation
//!
//! Responsibility: validate bounded enrollment shape using Auth-owned grant rules.
//! Boundary: this validates data, never its IC provenance or issuer key possession.

#[cfg(any(
    test,
    feature = "auth-local-application-authorization",
    feature = "auth-portable-issuer"
))]
use crate::ops::auth::enrollment::EnrollmentBinding;
use crate::ops::auth::enrollment::PortableEnrollmentError;
#[cfg(any(
    test,
    feature = "auth-local-application-authorization",
    feature = "auth-portable-issuer"
))]
use canic_contracts::dto::portable_enrollment::PortableEnrollmentSnapshot;
use canic_contracts::{
    dto::portable_enrollment::{
        PortableEnrollmentGrant, PortableIssuerEnrollment, PortableIssuerKey,
    },
    protocol::portable_enrollment as wire,
};
use ic_auth_protocol_types::{AuthRole, DelegatedRoleGrant, limits};
use ic_platform::Principal;
use sha2::{Digest, Sha256};

pub(super) fn key_id(key: &PortableIssuerKey) -> [u8; 32] {
    match key {
        PortableIssuerKey::Certified { certificate_hash } => *certificate_hash,
        PortableIssuerKey::Ed25519 { public_key } => Sha256::digest(public_key).into(),
    }
}

pub(in crate::ops::auth) fn validate_contents(
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
        grants(&entry.grants)?;
        previous = Some(id);
    }
    Ok(())
}

/// One Auth-owned grammar/order admission for Root, issuer and receiver grants.
pub(in crate::ops::auth) fn grants(
    input: &[PortableEnrollmentGrant],
) -> Result<ic_auth::portable::ValidatedGrants, PortableEnrollmentError> {
    use PortableEnrollmentError as E;
    if input.len() > limits::MAX_GRANTS
        || input
            .iter()
            .any(|grant| grant.scopes.len() > limits::MAX_SCOPES_PER_GRANT)
    {
        return Err(E::Capacity);
    }
    let grants = input
        .iter()
        .map(|grant| {
            let target: AuthRole = grant.role.parse().map_err(|_| E::Invalid)?;
            Ok(DelegatedRoleGrant {
                target,
                scopes: ic_auth_protocol_types::ScopeLabel::from_strings(grant.scopes.clone())
                    .map_err(|_| E::Invalid)?,
            })
        })
        .collect::<Result<Vec<_>, E>>()?;
    ic_auth::portable::ValidatedGrants::new(grants).map_err(|_| E::Invalid)
}

#[cfg(any(
    test,
    feature = "auth-local-application-authorization",
    feature = "auth-portable-issuer"
))]
pub(in crate::ops::auth) fn validate_snapshot(
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

/// Preserve the Root's complete authority history across all authenticated consumers.
#[cfg(any(
    test,
    feature = "auth-local-application-authorization",
    feature = "auth-portable-issuer"
))]
pub(in crate::ops::auth) fn validate_progress(
    retained: &PortableEnrollmentSnapshot,
    next: &PortableEnrollmentSnapshot,
) -> Result<(), PortableEnrollmentError> {
    use PortableEnrollmentError as E;
    validate_contents(&retained.entries, &retained.retired_keys)?;
    if retained.authority != next.authority
        || retained.receiver != next.receiver
        || retained.fleet != next.fleet
    {
        return Err(E::Binding);
    }
    if next.revision < retained.revision
        || next.generation < retained.generation
        || (next.revision == retained.revision && retained != next)
    {
        return Err(E::Rollback);
    }
    if retained
        .retired_keys
        .iter()
        .any(|id| next.retired_keys.binary_search(id).is_err())
        || retained.entries.iter().any(|entry| {
            let id = key_id(&entry.key);
            !next.entries.iter().any(|entry| key_id(&entry.key) == id)
                && next.retired_keys.binary_search(&id).is_err()
        })
    {
        return Err(E::Retired);
    }
    if next.generation == retained.generation
        && (next.entries != retained.entries || next.retired_keys != retained.retired_keys)
    {
        return Err(E::Generation);
    }
    Ok(())
}
