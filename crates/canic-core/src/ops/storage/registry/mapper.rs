//! Module: ops::storage::registry::mapper
//!
//! Responsibility: convert registry records into public topology response shapes.
//! Does not own: stable registry mutation, workflow orchestration, or DTO definitions.
//! Boundary: storage ops conversion layer for topology registry records.

use crate::{
    cdk::types::Principal,
    dto::canister::CanisterInfo,
    dto::topology::{AppRegistryEntry, AppRegistryResponse, SubnetRegistryEntry},
    storage::stable::registry::app::AppRegistryData,
};

///
/// AppRegistryResponseMapper
///
/// Storage-ops mapper for app registry records and response views.
///

pub struct AppRegistryResponseMapper;

impl AppRegistryResponseMapper {
    #[must_use]
    pub fn data_to_response(data: AppRegistryData) -> AppRegistryResponse {
        let entries = data
            .entries
            .into_iter()
            .map(|entry| AppRegistryEntry {
                subnet_pid: entry.subnet_pid,
                root_pid: entry.root_pid,
            })
            .collect();

        AppRegistryResponse(entries)
    }
}

///
/// SubnetRegistryResponseMapper
///
/// Storage-ops mapper for subnet registry records and response views.
///

pub struct SubnetRegistryResponseMapper;

impl SubnetRegistryResponseMapper {
    #[must_use]
    pub fn record_to_response(
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
