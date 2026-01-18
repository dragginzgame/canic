use crate::{
    InternalError,
    access::{self, AccessRuleFn},
    cdk::types::Principal,
    dto::auth::{DelegatedToken, DelegatedTokenClaims},
    dto::error::Error,
    ids::CanisterRole,
};

///
/// AuthAccessApi
///

pub struct AuthAccessApi;

impl AuthAccessApi {
    // --- Require --------------------------------------------------------

    pub async fn require_all(rules: Vec<AccessRuleFn>) -> Result<(), Error> {
        access::require_all(rules)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn require_any(rules: Vec<AccessRuleFn>) -> Result<(), Error> {
        access::require_any(rules)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    // --- Rules ----------------------------------------------------------

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

    pub async fn is_root(caller: Principal) -> Result<(), Error> {
        access::topology::is_root(caller)
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

    // --- Token auth ----------------------------------------------------

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

    pub async fn require_scope(
        claims: DelegatedTokenClaims,
        required_scope: String,
    ) -> Result<(), Error> {
        access::auth::require_scope(claims, required_scope)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn require_audience(
        claims: DelegatedTokenClaims,
        required_audience: String,
    ) -> Result<(), Error> {
        access::auth::require_audience(claims, required_audience)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }
}
