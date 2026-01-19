use crate::{
    dto::topology::{AppRegistryResponse, SubnetRegistryResponse},
    workflow::topology::registry::query::{AppRegistryQuery, SubnetRegistryQuery},
};

///
/// AppRegistryApi
///

pub struct AppRegistryApi;

impl AppRegistryApi {
    #[must_use]
    pub fn registry() -> AppRegistryResponse {
        AppRegistryQuery::registry()
    }
}

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
