use crate::{
    cdk::candid::Principal,
    dto::canister::CanisterInfo,
    dto::topology::{AppRegistryEntry, AppRegistryResponse, SubnetRegistryEntry},
    storage::stable::registry::app::AppRegistryRecord,
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
    pub fn entry_to_view(
        pid: Principal,
        record: crate::storage::canister::CanisterRecord,
    ) -> SubnetRegistryEntry {
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
    }
}
