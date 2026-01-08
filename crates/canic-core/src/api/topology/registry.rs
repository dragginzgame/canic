use crate::{
    dto::topology::{AppRegistryView, SubnetRegistryView},
    workflow::topology::registry::query::{AppRegistryQuery, SubnetRegistryQuery},
};

///
/// AppRegistryApi
///

pub struct AppRegistryApi;

impl AppRegistryApi {
    #[must_use]
    pub fn view() -> AppRegistryView {
        AppRegistryQuery::view()
    }
}

///
/// SubnetRegistryApi
///

pub struct SubnetRegistryApi;

impl SubnetRegistryApi {
    #[must_use]
    pub fn view() -> SubnetRegistryView {
        SubnetRegistryQuery::view()
    }
}
