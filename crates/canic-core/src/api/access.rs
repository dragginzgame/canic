use crate::{PublicError, access};

///
/// Access Api
///

pub fn guard_app_query() -> Result<(), PublicError> {
    access::guard::guard_app_query().map_err(PublicError::from)
}

pub fn guard_app_update() -> Result<(), PublicError> {
    access::guard::guard_app_update().map_err(PublicError::from)
}

pub async fn require_all(rules: Vec<access::auth::AuthRuleFn>) -> Result<(), PublicError> {
    access::auth::require_all(rules)
        .await
        .map_err(PublicError::from)
}

pub async fn require_any(rules: Vec<access::auth::AuthRuleFn>) -> Result<(), PublicError> {
    access::auth::require_any(rules)
        .await
        .map_err(PublicError::from)
}

pub async fn build_network_ic() -> Result<(), PublicError> {
    access::rule::build_network_ic()
        .await
        .map_err(PublicError::from)
}

pub async fn build_network_local() -> Result<(), PublicError> {
    access::rule::build_network_local()
        .await
        .map_err(PublicError::from)
}
