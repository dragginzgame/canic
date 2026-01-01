use crate::{
    dto::registry::{AppRegistryView, SubnetRegistryView},
    ops::storage::registry::{app::AppRegistryOps, subnet::SubnetRegistryOps},
    workflow::registry::mapper::{AppRegistryMapper, SubnetRegistryMapper},
};

pub(crate) fn app_registry_view() -> AppRegistryView {
    let snapshot = AppRegistryOps::snapshot();
    AppRegistryMapper::snapshot_to_view(snapshot)
}

pub(crate) fn subnet_registry_view() -> SubnetRegistryView {
    let snapshot = SubnetRegistryOps::snapshot();
    SubnetRegistryMapper::snapshot_to_view(snapshot)
}
