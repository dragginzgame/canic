use crate::{
    dto::topology::{AppRegistryView, SubnetRegistryView},
    ops::storage::registry::{app::AppRegistryOps, subnet::SubnetRegistryOps},
    workflow::topology::registry::mapper::{AppRegistryMapper, SubnetRegistryMapper},
};

pub fn app_registry_view() -> AppRegistryView {
    let snapshot = AppRegistryOps::snapshot();
    AppRegistryMapper::snapshot_to_view(snapshot)
}

pub fn subnet_registry_view() -> SubnetRegistryView {
    let snapshot = SubnetRegistryOps::snapshot();
    SubnetRegistryMapper::snapshot_to_view(snapshot)
}
