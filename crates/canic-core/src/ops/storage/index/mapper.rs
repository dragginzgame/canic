//! Module: ops::storage::index::mapper
//!
//! Responsibility: convert app/subnet index data to boundary views and inputs.
//! Does not own: stable index mutation, workflow orchestration, or DTO definitions.
//! Boundary: storage ops conversion layer for topology index snapshots.

use crate::{
    dto::{
        page::Page,
        topology::{AppIndexArgs, IndexEntryInput, IndexEntryResponse, SubnetIndexArgs},
    },
    model::topology::TopologyIndexEntry,
    storage::stable::index::{IndexEntryRecord, app::AppIndexData, subnet::SubnetIndexData},
    view::topology::IndexEntryView,
};

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

// Map stored index records into the shared index input entry shape.
fn data_entries_to_input(entries: Vec<IndexEntryRecord>) -> Vec<IndexEntryInput> {
    entries
        .into_iter()
        .map(|entry| IndexEntryInput {
            role: entry.role,
            pid: entry.pid,
        })
        .collect()
}

// Map index input entries back into stored index records.
fn input_entries_to_data(entries: Vec<IndexEntryInput>) -> Vec<IndexEntryRecord> {
    entries
        .into_iter()
        .map(|entry| IndexEntryRecord {
            role: entry.role,
            pid: entry.pid,
        })
        .collect()
}

///
/// AppIndexDataMapper
///
/// Storage-ops mapper for app index data and boundary input shapes.
///

pub struct AppIndexDataMapper;

impl AppIndexDataMapper {
    #[must_use]
    pub fn data_to_input(data: AppIndexData) -> AppIndexArgs {
        AppIndexArgs(data_entries_to_input(data.entries))
    }

    #[must_use]
    pub fn input_to_data(input: AppIndexArgs) -> AppIndexData {
        AppIndexData {
            entries: input_entries_to_data(input.0),
        }
    }
}

///
/// SubnetIndexDataMapper
///
/// Storage-ops mapper for subnet index data and boundary input shapes.
///

pub struct SubnetIndexDataMapper;

impl SubnetIndexDataMapper {
    #[must_use]
    pub fn data_to_input(data: SubnetIndexData) -> SubnetIndexArgs {
        SubnetIndexArgs(data_entries_to_input(data.entries))
    }

    #[must_use]
    pub fn input_to_data(input: SubnetIndexArgs) -> SubnetIndexData {
        SubnetIndexData {
            entries: input_entries_to_data(input.0),
        }
    }
}

///
/// IndexEntryMapper
///
/// Storage-ops mapper for index records, policy inputs, and response entries.
///

pub struct IndexEntryMapper;

impl IndexEntryMapper {
    #[must_use]
    pub fn records_to_projections(entries: Vec<IndexEntryRecord>) -> Vec<IndexEntryView> {
        entries
            .into_iter()
            .map(|entry| IndexEntryView {
                role: entry.role,
                pid: entry.pid,
            })
            .collect()
    }

    #[must_use]
    pub fn projection_page_to_response(page: Page<IndexEntryView>) -> Page<IndexEntryResponse> {
        Page {
            entries: page
                .entries
                .into_iter()
                .map(|entry| IndexEntryResponse {
                    role: entry.role,
                    pid: entry.pid,
                })
                .collect(),
            total: page.total,
        }
    }

    #[must_use]
    pub fn records_to_topology_entries(entries: &[IndexEntryRecord]) -> Vec<TopologyIndexEntry> {
        entries
            .iter()
            .map(|entry| TopologyIndexEntry {
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
        let projections = IndexEntryMapper::records_to_projections(vec![IndexEntryRecord {
            role: CanisterRole::new("app"),
            pid,
        }]);

        let response = IndexEntryMapper::projection_page_to_response(Page {
            entries: projections,
            total: 1,
        });

        assert_eq!(response.total, 1);
        assert_eq!(response.entries.len(), 1);
        assert_eq!(response.entries[0].role, CanisterRole::new("app"));
        assert_eq!(response.entries[0].pid, pid);
    }
}
