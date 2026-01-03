use crate::{
    dto::topology::{AppRegistryView, SubnetRegistryView},
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
        AppRegistryView(snapshot.entries)
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
                (role, view)
            })
            .collect();

        SubnetRegistryView(entries)
    }
}
