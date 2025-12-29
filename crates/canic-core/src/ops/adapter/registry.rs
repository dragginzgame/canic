use crate::{
    dto::registry::{AppRegistryView, SubnetRegistryView},
    model::memory::registry::{AppRegistryData, SubnetRegistryData},
    ops::adapter::canister::canister_entry_to_view,
};

#[must_use]
pub const fn app_registry_to_view(data: AppRegistryData) -> AppRegistryView {
    AppRegistryView(data)
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
