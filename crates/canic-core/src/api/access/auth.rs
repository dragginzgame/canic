use crate::{
    InternalError,
    access::{self, AccessRuleFn},
    cdk::types::Principal,
    dto::{
        auth::{DelegatedToken, DelegatedTokenClaims},
        error::Error,
    },
    ids::CanisterRole,
};

///
/// AuthAccessApi
///
/// WHY THIS FILE EXISTS
/// ---------------------
/// This module defines the **public authorization API** exposed to:
///   - macro-expanded endpoints
///   - DSL-generated auth guards
///   - higher-level application code
///
/// It intentionally sits between:
///   - `access::*` (internal authorization logic)
///   - `dto::error::Error` (external error surface)
///
/// Responsibilities:
///
/// 1. **Error domain translation**
///    Access-layer errors are internal and must never leak directly.
///    This API converts them into stable, user-facing error types.
///
/// 2. **Signature normalization**
///    This is the canonical place to adapt access-layer contracts
///    (e.g. `&'static str` policy constants) to callers.
///
/// 3. **Stability during refactors**
///    Access internals may change freely as long as this API remains stable.
///    Callers MUST NOT depend directly on `access::*`.
///
/// If this file appears repetitive, that is intentional.
/// DO NOT collapse it into the access layer.
///

pub struct AuthAccessApi;

impl AuthAccessApi {
    // --- Composition ---------------------------------------------------

    /// Require that ALL access rules succeed.
    ///
    /// Intended for use by DSL-expanded authorization pipelines.
    pub async fn require_all(rules: Vec<AccessRuleFn>) -> Result<(), Error> {
        access::require_all(rules)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    /// Require that ANY access rule succeeds.
    pub async fn require_any(rules: Vec<AccessRuleFn>) -> Result<(), Error> {
        access::require_any(rules)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    // --- Topology / identity rules ------------------------------------

    pub async fn is_app_directory_role(caller: Principal, role: CanisterRole) -> Result<(), Error> {
        access::rule::is_app_directory_role(caller, role)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn is_child(caller: Principal) -> Result<(), Error> {
        access::topology::is_child(caller)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn is_controller(caller: Principal) -> Result<(), Error> {
        access::env::is_controller(caller)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn is_parent(caller: Principal) -> Result<(), Error> {
        access::topology::is_parent(caller)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn is_principal(caller: Principal, expected: Principal) -> Result<(), Error> {
        access::rule::is_principal(caller, expected)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn is_registered_to_subnet(caller: Principal) -> Result<(), Error> {
        access::rule::is_registered_to_subnet(caller)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn caller_is_root(caller: Principal) -> Result<(), Error> {
        access::topology::caller_is_root(caller)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn is_same_canister(caller: Principal) -> Result<(), Error> {
        access::topology::is_same_canister(caller)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn is_subnet_directory_role(
        caller: Principal,
        role: CanisterRole,
    ) -> Result<(), Error> {
        access::rule::is_subnet_directory_role(caller, role)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn is_whitelisted(caller: Principal) -> Result<(), Error> {
        access::env::is_whitelisted(caller)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    // --- Delegated token auth -----------------------------------------

    /// Verify a delegated token read from the ingress payload.
    ///
    /// Intended for DSL-generated auth guards only.
    pub async fn verify_delegated_token() -> Result<(), Error> {
        access::auth::verify_delegated_token()
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn verify_token(
        token: DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
    ) -> Result<(), Error> {
        access::auth::verify_token(token, authority_pid, now_secs)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    /// Require that the delegated token includes a specific scope.
    ///
    /// `required_scope` MUST be a compile-time policy constant.
    pub async fn require_scope(
        claims: DelegatedTokenClaims,
        required_scope: &'static str,
    ) -> Result<(), Error> {
        access::auth::require_scope(claims, required_scope)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    /// Require that the delegated token targets a specific audience.
    ///
    /// `required_audience` MUST be a compile-time policy constant.
    pub async fn require_audience(
        claims: DelegatedTokenClaims,
        required_audience: &'static str,
    ) -> Result<(), Error> {
        access::auth::require_audience(claims, required_audience)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }
}
