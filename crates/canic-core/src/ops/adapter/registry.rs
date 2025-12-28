use crate::{
    dto::registry::{AppRegistryView, AppSubnetView, SubnetRegistryView},
    model::memory::registry::{AppRegistryData, AppSubnet, SubnetRegistryData},
    ops::adapter::canister::canister_entry_to_view,
};

#[must_use]
pub const fn app_subnet_to_view(data: AppSubnet) -> AppSubnetView {
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

#[must_use]
pub fn subnet_registry_to_view(data: SubnetRegistryData) -> SubnetRegistryView {
    let entries = data
        .into_iter()
        .map(|(_, entry)| {
            let role = entry.role.clone();
            let view = canister_entry_to_view(&entry);
            (role, view)
        })
        .collect();

    SubnetRegistryView(entries)
}
