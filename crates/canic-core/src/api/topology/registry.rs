use crate::{
    dto::topology::{AppRegistryView, SubnetRegistryView},
    workflow,
};

///
/// AppRegistryApi
///

pub struct AppRegistryApi;

impl AppRegistryApi {
    #[must_use]
    pub fn view() -> AppRegistryView {
        workflow::topology::registry::query::AppRegistryQuery::view()
    }
}

///
/// SubnetRegistryApi
///

pub struct SubnetRegistryApi;

impl SubnetRegistryApi {
    #[must_use]
    pub fn view() -> SubnetRegistryView {
        workflow::topology::registry::query::SubnetRegistryQuery::view()
    }
}
