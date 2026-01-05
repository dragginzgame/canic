use crate::{PublicError, access::rule};

///
/// Rule API
///

pub async fn build_network_ic() -> Result<(), PublicError> {
    rule::build_network_ic().await.map_err(PublicError::from)
}

pub async fn build_network_local() -> Result<(), PublicError> {
    rule::build_network_local().await.map_err(PublicError::from)
}
