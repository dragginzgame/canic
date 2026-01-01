use crate::{
    Error, PublicError,
    access::{self, AccessError},
};

#[must_use]
pub fn to_public(err: AccessError) -> PublicError {
    PublicError::from(Error::from(err))
}

pub fn guard_app_query() -> Result<(), PublicError> {
    access::guard::guard_app_query().map_err(to_public)
}

pub fn guard_app_update() -> Result<(), PublicError> {
    access::guard::guard_app_update().map_err(to_public)
}

pub async fn require_all(rules: Vec<access::auth::AuthRuleFn>) -> Result<(), PublicError> {
    access::auth::require_all(rules).await.map_err(to_public)
}

pub async fn require_any(rules: Vec<access::auth::AuthRuleFn>) -> Result<(), PublicError> {
    access::auth::require_any(rules).await.map_err(to_public)
}

pub async fn build_network_ic() -> Result<(), PublicError> {
    access::rule::build_network_ic().await.map_err(to_public)
}

pub async fn build_network_local() -> Result<(), PublicError> {
    access::rule::build_network_local().await.map_err(to_public)
}
