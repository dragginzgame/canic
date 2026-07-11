//! Module: ops::storage::index::mapper
//!
//! Responsibility: convert app/subnet index data to boundary views and inputs.
//! Does not own: stable index mutation, workflow orchestration, or DTO definitions.
//! Boundary: storage ops conversion layer for topology index snapshots.

use crate::{
    domain::policy::pure::topology::IndexPolicyInput,
    dto::{
        page::Page,
        topology::{AppIndexArgs, IndexEntryInput, IndexEntryResponse, SubnetIndexArgs},
    },
    storage::stable::index::{IndexEntryRecord, app::AppIndexData, subnet::SubnetIndexData},
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
    pub fn record_page_to_response(page: Page<IndexEntryRecord>) -> Page<IndexEntryResponse> {
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
    pub fn records_to_policy_input(entries: &[IndexEntryRecord]) -> Vec<IndexPolicyInput> {
        entries
            .iter()
            .map(|entry| IndexPolicyInput {
                role: entry.role.clone(),
                pid: entry.pid,
            })
            .collect()
    }
}
