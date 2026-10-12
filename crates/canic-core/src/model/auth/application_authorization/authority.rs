//! Module: model::auth::application_authorization::authority
//!
//! Responsibility: enforce canonical verified-authority and local-session invariants.
//! Does not own: proof cryptography, stable records, access reads, or authorization policy.
//! Boundary: ops supplies verified values; policy receives invariant-bearing models.

use crate::{
    cdk::types::Principal,
    model::auth::application_authorization::{ApplicationScopeError, CanonicalApplicationScopes},
};
use canic_contracts::ids::{CanisterRole, FleetKey};
use thiserror::Error;

/// Issuer identity returned only after complete shared token verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedApplicationAuthority {
    issuer: Principal,
}

impl VerifiedApplicationAuthority {
    #[cfg(any(test, feature = "auth-delegated-token-verify"))]
    pub const fn new(issuer: Principal) -> Self {
        Self { issuer }
    }
    #[must_use]
    pub const fn issuer(&self) -> Principal {
        self.issuer
    }
}

/// Protected local authority used to validate retained application sessions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalApplicationAuthoritySnapshot {
    fleet: FleetKey,
    role: CanisterRole,
    generation: u64,
}

impl LocalApplicationAuthoritySnapshot {
    #[must_use]
    pub const fn new(fleet: FleetKey, role: CanisterRole, generation: u64) -> Self {
        Self {
            fleet,
            role,
            generation,
        }
    }

    #[must_use]
    pub const fn fleet(&self) -> FleetKey {
        self.fleet
    }

    #[must_use]
    pub const fn role(&self) -> &CanisterRole {
        &self.role
    }

    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }
}

/// Current protected inputs whose narrowing can invalidate retained sessions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalApplicationAuthorityBinding {
    Disabled,
    Enabled {
        fleet: FleetKey,
        role: CanisterRole,
        verifier_root_canister_id: Principal,
        minimum_accepted_registry_epoch: Option<u64>,
        allowed_scopes: CanonicalApplicationScopes,
        maximum_session_ttl_secs: u64,
    },
}

impl LocalApplicationAuthorityBinding {
    /// Construct one enabled protected binding from already validated inputs.
    #[must_use]
    pub const fn enabled(
        fleet: FleetKey,
        role: CanisterRole,
        verifier_root_canister_id: Principal,
        minimum_accepted_registry_epoch: Option<u64>,
        allowed_scopes: CanonicalApplicationScopes,
        maximum_session_ttl_secs: u64,
    ) -> Self {
        Self::Enabled {
            fleet,
            role,
            verifier_root_canister_id,
            minimum_accepted_registry_epoch,
            allowed_scopes,
            maximum_session_ttl_secs,
        }
    }
}

/// Canonical active local application session inspected by pure policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalApplicationSession {
    shared_session: Vec<u8>,
    transport_caller: Principal,
    authenticated_subject: Principal,
    issuer: Principal,
    fleet: FleetKey,
    role: CanisterRole,
    scopes: CanonicalApplicationScopes,
    authority_generation: u64,
    established_at_ns: u64,
    expires_at_ns: u64,
    proof_fingerprint: [u8; 32],
    establishment_request_hash: [u8; 32],
}

impl LocalApplicationSession {
    /// Construct one invariant-bearing local application session value.
    #[expect(
        clippy::too_many_arguments,
        reason = "one constructor binds the complete session tuple"
    )]
    pub fn new(
        transport_caller: Principal,
        authenticated_subject: Principal,
        issuer: Principal,
        fleet: FleetKey,
        role: CanisterRole,
        scopes: CanonicalApplicationScopes,
        authority_generation: u64,
        established_at_ns: u64,
        expires_at_ns: u64,
        proof_fingerprint: [u8; 32],
        establishment_request_hash: [u8; 32],
    ) -> Result<Self, ApplicationAuthorityModelError> {
        if transport_caller != authenticated_subject {
            return Err(ApplicationAuthorityModelError::CallerSubjectMismatch);
        }
        if expires_at_ns <= established_at_ns {
            return Err(ApplicationAuthorityModelError::InvalidSessionWindow);
        }
        Ok(Self {
            transport_caller,
            shared_session: Vec::new(),
            authenticated_subject,
            issuer,
            fleet,
            role,
            scopes,
            authority_generation,
            established_at_ns,
            expires_at_ns,
            proof_fingerprint,
            establishment_request_hash,
        })
    }

    /// Attach the canonical shared session to its product-only view projection.
    pub(crate) fn with_shared_session(mut self, bytes: Vec<u8>) -> Self {
        self.shared_session = bytes;
        self
    }

    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    pub(crate) fn shared_session_bytes(&self) -> &[u8] {
        &self.shared_session
    }

    #[must_use]
    pub const fn transport_caller(&self) -> Principal {
        self.transport_caller
    }

    #[must_use]
    pub const fn authenticated_subject(&self) -> Principal {
        self.authenticated_subject
    }

    #[must_use]
    pub const fn issuer(&self) -> Principal {
        self.issuer
    }

    #[must_use]
    pub const fn fleet(&self) -> FleetKey {
        self.fleet
    }

    #[must_use]
    pub const fn role(&self) -> &CanisterRole {
        &self.role
    }

    #[must_use]
    pub const fn scopes(&self) -> &CanonicalApplicationScopes {
        &self.scopes
    }

    #[must_use]
    pub const fn authority_generation(&self) -> u64 {
        self.authority_generation
    }

    #[must_use]
    pub const fn established_at_ns(&self) -> u64 {
        self.established_at_ns
    }

    #[must_use]
    pub const fn expires_at_ns(&self) -> u64 {
        self.expires_at_ns
    }

    #[must_use]
    pub const fn proof_fingerprint(&self) -> [u8; 32] {
        self.proof_fingerprint
    }

    #[must_use]
    #[cfg(any(test, feature = "auth-local-application-authorization"))]
    pub const fn establishment_request_hash(&self) -> [u8; 32] {
        self.establishment_request_hash
    }
}

/// Durable proof-consumption identity retained independently of an active session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalApplicationReplay {
    proof_fingerprint: [u8; 32],
    transport_caller: Principal,
    authenticated_subject: Principal,
    authority_generation: u64,
    remove_at_ns: u64,
}

impl LocalApplicationReplay {
    /// Construct one replay tombstone bound to the exact consumed proof authority.
    pub fn new(
        proof_fingerprint: [u8; 32],
        transport_caller: Principal,
        authenticated_subject: Principal,
        authority_generation: u64,
        remove_at_ns: u64,
    ) -> Result<Self, ApplicationAuthorityModelError> {
        if transport_caller != authenticated_subject {
            return Err(ApplicationAuthorityModelError::CallerSubjectMismatch);
        }
        if remove_at_ns == 0 {
            return Err(ApplicationAuthorityModelError::InvalidReplayRemovalTime);
        }
        Ok(Self {
            proof_fingerprint,
            transport_caller,
            authenticated_subject,
            authority_generation,
            remove_at_ns,
        })
    }

    #[must_use]
    pub const fn proof_fingerprint(&self) -> [u8; 32] {
        self.proof_fingerprint
    }

    #[must_use]
    pub const fn authenticated_subject(&self) -> Principal {
        self.authenticated_subject
    }

    #[must_use]
    pub const fn authority_generation(&self) -> u64 {
        self.authority_generation
    }
}

/// Typed model invariant failure for application authority values.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ApplicationAuthorityModelError {
    #[error("application session caller and subject differ")]
    CallerSubjectMismatch,

    #[error("application replay removal time must be nonzero")]
    InvalidReplayRemovalTime,

    #[error("application session expiry must be after establishment")]
    InvalidSessionWindow,

    #[error(transparent)]
    Scope(#[from] ApplicationScopeError),
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::auth::application_authorization::ApplicationScope;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn scopes() -> CanonicalApplicationScopes {
        CanonicalApplicationScopes::for_session(vec![ApplicationScope::parse("app:read").unwrap()])
            .unwrap()
    }

    #[test]
    fn local_session_rejects_different_caller_and_subject() {
        let err = LocalApplicationSession::new(
            p(1),
            p(2),
            p(3),
            crate::test::support::fleet_key(1),
            CanisterRole::new("component"),
            scopes(),
            4,
            10,
            20,
            [5; 32],
            [6; 32],
        )
        .unwrap_err();
        assert_eq!(err, ApplicationAuthorityModelError::CallerSubjectMismatch);
    }

    #[test]
    fn replay_rejects_different_caller_and_subject() {
        let err = LocalApplicationReplay::new([1; 32], p(1), p(2), 3, 4).unwrap_err();
        assert_eq!(err, ApplicationAuthorityModelError::CallerSubjectMismatch);
    }
}
