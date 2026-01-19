use crate::{
    dto::canister::CanisterInfo,
    dto::topology::{
        AppRegistryEntry, AppRegistryResponse, SubnetRegistryEntry, SubnetRegistryResponse,
    },
    storage::stable::registry::{app::AppRegistryRecord, subnet::SubnetRegistryRecord},
};

///
/// AppRegistryResponseMapper
///

pub struct AppRegistryResponseMapper;

impl AppRegistryResponseMapper {
    #[must_use]
    pub fn record_to_view(data: AppRegistryRecord) -> AppRegistryResponse {
        let entries = data
            .entries
            .into_iter()
            .map(|(subnet_pid, root_pid)| AppRegistryEntry {
                subnet_pid,
                root_pid,
            })
            .collect();

        AppRegistryResponse(entries)
    }
}

///
/// SubnetRegistryResponseMapper
///

pub struct SubnetRegistryResponseMapper;

impl SubnetRegistryResponseMapper {
    #[must_use]
    pub fn record_to_view(data: SubnetRegistryRecord) -> SubnetRegistryResponse {
        let entries = data
            .entries
            .into_iter()
            .map(|(pid, record)| {
                let record_view = CanisterInfo {
                    pid,
                    role: record.role.clone(),
                    parent_pid: record.parent_pid,
                    module_hash: record.module_hash,
                    created_at: record.created_at,
                };

                SubnetRegistryEntry {
                    pid,
                    role: record_view.role.clone(),
                    record: record_view,
                }
            })
            .collect();

        SubnetRegistryResponse(entries)
    }
}
