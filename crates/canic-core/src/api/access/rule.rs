use crate::{InternalError, access::rule, dto::error::Error};

///
/// RuleApi
///

pub struct RuleApi;

impl RuleApi {
    pub async fn build_network_ic() -> Result<(), Error> {
        rule::build_network_ic()
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub async fn build_network_local() -> Result<(), Error> {
        rule::build_network_local()
            .await
            .map_err(InternalError::from)
            .map_err(Error::from)
    }
}
