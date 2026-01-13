use crate::{Error, access::rule};

///
/// RuleApi
///

pub struct RuleApi;

impl RuleApi {
    pub async fn build_network_ic() -> Result<(), Error> {
        rule::build_network_ic().await.map_err(Error::from)
    }

    pub async fn build_network_local() -> Result<(), Error> {
        rule::build_network_local().await.map_err(Error::from)
    }
}
