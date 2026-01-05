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
        let snapshot = AppRegistryOps::snapshot();

        AppRegistryMapper::snapshot_to_view(snapshot)
    }
}

///
/// SubnetRegistryQuery
///

pub struct SubnetRegistryQuery;

impl SubnetRegistryQuery {
    pub fn view() -> SubnetRegistryView {
        let snapshot = SubnetRegistryOps::snapshot();

        SubnetRegistryMapper::snapshot_to_view(snapshot)
    }
}
