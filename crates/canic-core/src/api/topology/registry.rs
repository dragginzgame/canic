use crate::{
    dto::topology::SubnetRegistryResponse, workflow::topology::registry::query::SubnetRegistryQuery,
};

///
/// SubnetRegistryApi
///

pub struct SubnetRegistryApi;

impl SubnetRegistryApi {
    #[must_use]
    pub fn registry() -> SubnetRegistryResponse {
        SubnetRegistryQuery::registry()
    }
}
