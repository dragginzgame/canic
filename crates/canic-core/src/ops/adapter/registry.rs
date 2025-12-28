use crate::{
    dto::registry::{AppRegistryView, AppSubnetView},
    model::memory::registry::{AppRegistryData, AppSubnet},
};

pub fn app_subnet_to_view(data: AppSubnet) -> AppSubnetView {
    AppSubnetView {
        subnet_pid: data.subnet_pid,
        root_pid: data.root_pid,
    }
}

#[must_use]
pub fn app_registry_to_view(data: AppRegistryData) -> AppRegistryView {
    let subnets = data
        .into_iter()
        .map(|(principal, subnet)| (principal, app_subnet_to_view(subnet)))
        .collect();

    AppRegistryView(subnets)
}
