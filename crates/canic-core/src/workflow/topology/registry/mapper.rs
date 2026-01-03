use crate::{
    dto::topology::{
        AppRegistryEntryView, AppRegistryView, SubnetRegistryEntryView, SubnetRegistryView,
    },
    ops::storage::registry::{app::AppRegistrySnapshot, subnet::SubnetRegistrySnapshot},
    workflow::canister::mapper::CanisterMapper,
};

///
/// AppRegistryMapper
///

pub struct AppRegistryMapper;

impl AppRegistryMapper {
    #[must_use]
    pub fn snapshot_to_view(snapshot: AppRegistrySnapshot) -> AppRegistryView {
        let entries = snapshot
            .entries
            .into_iter()
            .map(|(subnet_pid, root_pid)| AppRegistryEntryView {
                subnet_pid,
                root_pid,
            })
            .collect();

        AppRegistryView(entries)
    }
}

///
/// SubnetRegistryMapper
///

pub struct SubnetRegistryMapper;

impl SubnetRegistryMapper {
    #[must_use]
    pub fn snapshot_to_view(snapshot: SubnetRegistrySnapshot) -> SubnetRegistryView {
        let entries = snapshot
            .entries
            .into_iter()
            .map(|(_, entry)| {
                let role = entry.role.clone();
                let view = CanisterMapper::entry_to_view(&entry);
                SubnetRegistryEntryView { role, entry: view }
            })
            .collect();

        SubnetRegistryView(entries)
    }
}
