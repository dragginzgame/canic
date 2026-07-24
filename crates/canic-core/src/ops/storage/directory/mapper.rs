//! Module: ops::storage::directory::mapper
//!
//! Responsibility: convert Fleet/Subnet Directory data to boundary views and inputs.
//! Does not own: stable Directory mutation, workflow orchestration, or DTO definitions.
//! Boundary: storage ops conversion layer for topology Directory snapshots.

use crate::{
    dto::{
        page::Page,
        topology::{
            DirectoryEntryInput, DirectoryEntryResponse, DirectoryProvenance, FleetDirectoryInput,
            SubnetDirectoryInput,
        },
    },
    model::topology::TopologyDirectoryEntry,
    storage::stable::directory::{
        DirectoryEntryRecord, fleet::FleetDirectoryData, subnet::SubnetDirectoryData,
    },
    view::topology::DirectoryEntryView,
};

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

// Map stored Directory records into the shared input entry shape.
fn data_entries_to_input(entries: Vec<DirectoryEntryRecord>) -> Vec<DirectoryEntryInput> {
    entries
        .into_iter()
        .map(|entry| DirectoryEntryInput {
            role: entry.role,
            pid: entry.pid,
        })
        .collect()
}

// Map Directory input entries back into stored records.
fn input_entries_to_data(entries: Vec<DirectoryEntryInput>) -> Vec<DirectoryEntryRecord> {
    entries
        .into_iter()
        .map(|entry| DirectoryEntryRecord {
            role: entry.role,
            pid: entry.pid,
        })
        .collect()
}

///
/// FleetDirectoryDataMapper
///
/// Storage-ops mapper for Fleet Directory data and boundary input shapes.
///

pub struct FleetDirectoryDataMapper;

impl FleetDirectoryDataMapper {
    #[must_use]
    pub fn data_to_input(
        data: FleetDirectoryData,
        provenance: DirectoryProvenance,
    ) -> FleetDirectoryInput {
        FleetDirectoryInput {
            provenance,
            entries: data_entries_to_input(data.entries),
        }
    }

    #[must_use]
    pub fn input_to_data(input: FleetDirectoryInput) -> FleetDirectoryData {
        FleetDirectoryData {
            entries: input_entries_to_data(input.entries),
        }
    }
}

///
/// SubnetDirectoryDataMapper
///
/// Storage-ops mapper for Subnet Directory data and boundary input shapes.
///

pub struct SubnetDirectoryDataMapper;

impl SubnetDirectoryDataMapper {
    #[must_use]
    pub fn data_to_input(
        data: SubnetDirectoryData,
        provenance: DirectoryProvenance,
    ) -> SubnetDirectoryInput {
        SubnetDirectoryInput {
            provenance,
            entries: data_entries_to_input(data.entries),
        }
    }

    #[must_use]
    pub fn input_to_data(input: SubnetDirectoryInput) -> SubnetDirectoryData {
        SubnetDirectoryData {
            entries: input_entries_to_data(input.entries),
        }
    }
}

///
/// DirectoryEntryMapper
///
/// Storage-ops mapper for Directory records, policy inputs, and response entries.
///

pub struct DirectoryEntryMapper;

impl DirectoryEntryMapper {
    #[must_use]
    pub fn records_to_projections(entries: Vec<DirectoryEntryRecord>) -> Vec<DirectoryEntryView> {
        entries
            .into_iter()
            .map(|entry| DirectoryEntryView {
                role: entry.role,
                pid: entry.pid,
            })
            .collect()
    }

    #[must_use]
    pub fn projection_page_to_response(
        page: Page<DirectoryEntryView>,
    ) -> Page<DirectoryEntryResponse> {
        Page {
            entries: page
                .entries
                .into_iter()
                .map(|entry| DirectoryEntryResponse {
                    role: entry.role,
                    pid: entry.pid,
                })
                .collect(),
            total: page.total,
        }
    }

    #[must_use]
    pub fn records_to_topology_entries(
        entries: &[DirectoryEntryRecord],
    ) -> Vec<TopologyDirectoryEntry> {
        entries
            .iter()
            .map(|entry| TopologyDirectoryEntry {
                role: entry.role.clone(),
                pid: entry.pid,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cdk::types::Principal, ids::CanisterRole};

    #[test]
    fn records_project_before_response_mapping() {
        let pid = Principal::from_slice(&[1]);
        let projections =
            DirectoryEntryMapper::records_to_projections(vec![DirectoryEntryRecord {
                role: CanisterRole::new("app"),
                pid,
            }]);

        let response = DirectoryEntryMapper::projection_page_to_response(Page {
            entries: projections,
            total: 1,
        });

        assert_eq!(response.total, 1);
        assert_eq!(response.entries.len(), 1);
        assert_eq!(response.entries[0].role, CanisterRole::new("app"));
        assert_eq!(response.entries[0].pid, pid);
    }
}
