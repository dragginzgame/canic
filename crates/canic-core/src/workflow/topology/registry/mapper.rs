use crate::{
    dto::canister::CanisterRecordView,
    dto::topology::{
        AppRegistryEntryView, AppRegistryView, SubnetRegistryEntryView, SubnetRegistryView,
    },
    ops::storage::registry::{app::AppRegistrySnapshot, subnet::SubnetRegistrySnapshot},
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
            .map(|(pid, record)| {
                let record_view = CanisterRecordView {
                    pid,
                    role: record.role.clone(),
                    parent_pid: record.parent_pid,
                    module_hash: record.module_hash,
                    created_at: record.created_at,
                };

                SubnetRegistryEntryView {
                    pid,
                    role: record_view.role.clone(),
                    record: record_view,
                }
            })
            .collect();

        SubnetRegistryView(entries)
    }
}
