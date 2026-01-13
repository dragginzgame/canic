use crate::{
    InternalError,
    access::auth::{self, AuthRuleFn},
    cdk::types::Principal,
    dto::error::Error,
    ids::CanisterRole,
};

///
/// AuthAccessApi
///

pub struct AuthAccessApi;

impl AuthAccessApi {
    // --- Require --------------------------------------------------------

    pub async fn require_all(rules: Vec<AuthRuleFn>) -> Result<(), Error> {
        auth::require_all(rules)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn require_any(rules: Vec<AuthRuleFn>) -> Result<(), Error> {
        auth::require_any(rules)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    // --- Rules ----------------------------------------------------------

    pub async fn is_app_directory_role(caller: Principal, role: CanisterRole) -> Result<(), Error> {
        auth::is_app_directory_role(caller, role)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn is_child(caller: Principal) -> Result<(), Error> {
        auth::is_child(caller)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn is_controller(caller: Principal) -> Result<(), Error> {
        auth::is_controller(caller)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn is_parent(caller: Principal) -> Result<(), Error> {
        auth::is_parent(caller)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn is_principal(caller: Principal, expected: Principal) -> Result<(), Error> {
        auth::is_principal(caller, expected)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn is_registered_to_subnet(caller: Principal) -> Result<(), Error> {
        auth::is_registered_to_subnet(caller)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn is_root(caller: Principal) -> Result<(), Error> {
        auth::is_root(caller)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn is_same_canister(caller: Principal) -> Result<(), Error> {
        auth::is_same_canister(caller)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn is_subnet_directory_role(
        caller: Principal,
        role: CanisterRole,
    ) -> Result<(), Error> {
        auth::is_subnet_directory_role(caller, role)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn is_whitelisted(caller: Principal) -> Result<(), Error> {
        auth::is_whitelisted(caller)
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }
}
