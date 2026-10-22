//! Module: model::auth::application_authorization::authority
//!
//! Responsibility: retain Canic protected fleet authority and generation bindings.
//! Does not own: proof cryptography, stable records, access reads, or authorization policy.
//! Boundary: ops supplies verified values; policy receives invariant-bearing models.

#[cfg(any(test, feature = "auth-local-application-authorization"))]
use crate::{
    cdk::types::Principal, model::auth::application_authorization::CanonicalApplicationScopes,
};
use canic_contracts::ids::{CanisterRole, FleetKey};

/// Protected local authority used to validate retained application sessions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalApplicationAuthoritySnapshot {
    fleet: FleetKey,
    role: CanisterRole,
}

impl LocalApplicationAuthoritySnapshot {
    #[must_use]
    #[cfg(any(
        test,
        feature = "auth-local-application-authorization",
        feature = "internal-test-fixtures"
    ))]
    pub const fn new(fleet: FleetKey, role: CanisterRole) -> Self {
        Self { fleet, role }
    }

    #[must_use]
    pub const fn fleet(&self) -> FleetKey {
        self.fleet
    }

    #[must_use]
    pub const fn role(&self) -> &CanisterRole {
        &self.role
    }
}

/// Current protected inputs whose narrowing can invalidate retained sessions.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg(any(test, feature = "auth-local-application-authorization"))]
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

#[cfg(any(test, feature = "auth-local-application-authorization"))]
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
