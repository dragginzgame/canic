use crate::{
    dto::canister::CanisterRecordView,
    dto::topology::{
        AppRegistryEntryView, AppRegistryView, SubnetRegistryEntryView, SubnetRegistryView,
    },
    storage::stable::registry::{app::AppRegistryData, subnet::SubnetRegistryData},
};

///
/// AppRegistryMapper
///

pub struct AppRegistryMapper;

impl AppRegistryMapper {
    #[must_use]
    pub fn data_to_view(data: AppRegistryData) -> AppRegistryView {
        let entries = data
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
    pub fn data_to_view(data: SubnetRegistryData) -> SubnetRegistryView {
        let entries = data
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
