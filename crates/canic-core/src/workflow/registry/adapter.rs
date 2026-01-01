use crate::{
    dto::registry::{AppRegistryView, SubnetRegistryView},
    ops::storage::{
        canister::adapter::canister_entry_to_view,
        registry::{app::AppRegistrySnapshot, subnet::SubnetRegistrySnapshot},
    },
};

#[must_use]
pub fn app_registry_view_from_snapshot(snapshot: AppRegistrySnapshot) -> AppRegistryView {
    AppRegistryView(snapshot.entries)
}

#[must_use]
pub fn subnet_registry_view_from_snapshot(snapshot: SubnetRegistrySnapshot) -> SubnetRegistryView {
    let entries = snapshot
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
