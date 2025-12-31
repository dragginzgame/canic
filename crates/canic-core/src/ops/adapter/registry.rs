use crate::{
    dto::registry::{AppRegistryView, SubnetRegistryView},
    ops::adapter::canister::canister_entry_to_view,
    storage::memory::registry::{app::AppRegistryData, subnet::SubnetRegistryData},
};

#[must_use]
pub fn app_registry_to_view(data: AppRegistryData) -> AppRegistryView {
    AppRegistryView(data.entries)
}

#[must_use]
pub fn subnet_registry_to_view(data: SubnetRegistryData) -> SubnetRegistryView {
    let entries = data
        .entries
        .into_iter()
        .map(|(_, entry)| {
            let role = entry.role.clone();
            let view = canister_entry_to_view(&entry);
            (role, view)
        })
        .collect();

    SubnetRegistryView(entries)
}
