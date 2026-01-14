use crate::{
    dto::topology::{AppRegistryView, SubnetRegistryView},
    ops::storage::registry::{app::AppRegistryOps, subnet::SubnetRegistryOps},
    workflow::topology::registry::mapper::{AppRegistryMapper, SubnetRegistryMapper},
};

///
/// AppRegistryQuery
///

pub struct AppRegistryQuery;

impl AppRegistryQuery {
    pub fn view() -> AppRegistryView {
        let data = AppRegistryOps::data();

        AppRegistryMapper::data_to_view(data)
    }
}

///
/// SubnetRegistryQuery
///

pub struct SubnetRegistryQuery;

impl SubnetRegistryQuery {
    pub fn view() -> SubnetRegistryView {
        let data = SubnetRegistryOps::data();

        SubnetRegistryMapper::data_to_view(data)
    }
}
