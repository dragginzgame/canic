use crate::{
    dto::registry::{AppRegistryView, SubnetRegistryView},
    ops::storage::registry::{app::AppRegistryOps, subnet::SubnetRegistryOps},
    workflow::registry::adapter::{
        app_registry_view_from_snapshot, subnet_registry_view_from_snapshot,
    },
};

pub(crate) fn app_registry_view() -> AppRegistryView {
    let snapshot = AppRegistryOps::snapshot();
    app_registry_view_from_snapshot(snapshot)
}

pub(crate) fn subnet_registry_view() -> SubnetRegistryView {
    let snapshot = SubnetRegistryOps::snapshot();
    subnet_registry_view_from_snapshot(snapshot)
}
