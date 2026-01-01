use crate::{
    dto::{
        pool::CanisterPoolView,
        registry::{AppRegistryView, SubnetRegistryView},
    },
    ops::storage::{
        pool::PoolOps,
        registry::{app::AppRegistryOps, subnet::SubnetRegistryOps},
    },
};

pub(crate) fn app_registry_view() -> AppRegistryView {
    AppRegistryOps::export_view()
}

pub(crate) fn subnet_registry_view() -> SubnetRegistryView {
    SubnetRegistryOps::export_view()
}

pub(crate) fn pool_list_view() -> CanisterPoolView {
    PoolOps::export_view()
}
