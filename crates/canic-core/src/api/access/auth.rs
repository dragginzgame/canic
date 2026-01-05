use crate::{
    PublicError,
    access::auth::{self, AuthRuleFn},
    cdk::types::Principal,
    ids::CanisterRole,
};

///
/// AuthApi
///

pub struct AuthApi;

impl AuthApi {
    // --- Require --------------------------------------------------------

    pub async fn require_all(rules: Vec<AuthRuleFn>) -> Result<(), PublicError> {
        auth::require_all(rules).await.map_err(PublicError::from)
    }

    pub async fn require_any(rules: Vec<AuthRuleFn>) -> Result<(), PublicError> {
        auth::require_any(rules).await.map_err(PublicError::from)
    }

    // --- Rules ----------------------------------------------------------

    pub async fn is_app_directory_role(
        caller: Principal,
        role: CanisterRole,
    ) -> Result<(), PublicError> {
        auth::is_app_directory_role(caller, role)
            .await
            .map_err(PublicError::from)
    }

    pub async fn is_child(caller: Principal) -> Result<(), PublicError> {
        auth::is_child(caller).await.map_err(PublicError::from)
    }

    pub async fn is_controller(caller: Principal) -> Result<(), PublicError> {
        auth::is_controller(caller).await.map_err(PublicError::from)
    }

    pub async fn is_parent(caller: Principal) -> Result<(), PublicError> {
        auth::is_parent(caller).await.map_err(PublicError::from)
    }

    pub async fn is_principal(caller: Principal, expected: Principal) -> Result<(), PublicError> {
        auth::is_principal(caller, expected)
            .await
            .map_err(PublicError::from)
    }

    pub async fn is_registered_to_subnet(caller: Principal) -> Result<(), PublicError> {
        auth::is_registered_to_subnet(caller)
            .await
            .map_err(PublicError::from)
    }

    pub async fn is_root(caller: Principal) -> Result<(), PublicError> {
        auth::is_root(caller).await.map_err(PublicError::from)
    }

    pub async fn is_same_canister(caller: Principal) -> Result<(), PublicError> {
        auth::is_same_canister(caller)
            .await
            .map_err(PublicError::from)
    }

    pub async fn is_subnet_directory_role(
        caller: Principal,
        role: CanisterRole,
    ) -> Result<(), PublicError> {
        auth::is_subnet_directory_role(caller, role)
            .await
            .map_err(PublicError::from)
    }

    pub async fn is_whitelisted(caller: Principal) -> Result<(), PublicError> {
        auth::is_whitelisted(caller)
            .await
            .map_err(PublicError::from)
    }
}
