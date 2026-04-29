use crate::{
    cdk::types::Principal,
    dto::{
        page::Page,
        topology::{AppIndexArgs, IndexEntryInput, IndexEntryResponse, SubnetIndexArgs},
    },
    ids::CanisterRole,
    storage::stable::index::{app::AppIndexRecord, subnet::SubnetIndexRecord},
};

// --- Helpers ---------------------------------------------------------------

// Map stored index tuples into the shared index input entry shape.
fn record_entries_to_input(entries: Vec<(CanisterRole, Principal)>) -> Vec<IndexEntryInput> {
    entries
        .into_iter()
        .map(|(role, pid)| IndexEntryInput { role, pid })
        .collect()
}

// Map stored index tuples into the public response entry shape.
fn record_entries_to_response(entries: Vec<(CanisterRole, Principal)>) -> Vec<IndexEntryResponse> {
    entries
        .into_iter()
        .map(|(role, pid)| IndexEntryResponse { role, pid })
        .collect()
}

// Map index input entries back into stored index tuples.
fn input_entries_to_record(entries: Vec<IndexEntryInput>) -> Vec<(CanisterRole, Principal)> {
    entries
        .into_iter()
        .map(|entry| (entry.role, entry.pid))
        .collect()
}

///
/// AppIndexRecordMapper
///

pub struct AppIndexRecordMapper;

impl AppIndexRecordMapper {
    #[must_use]
    pub fn record_to_input(data: AppIndexRecord) -> AppIndexArgs {
        AppIndexArgs(record_entries_to_input(data.entries))
    }

    #[must_use]
    pub fn input_to_record(input: AppIndexArgs) -> AppIndexRecord {
        AppIndexRecord {
            entries: input_entries_to_record(input.0),
        }
    }
}

///
/// SubnetIndexRecordMapper
///

pub struct SubnetIndexRecordMapper;

impl SubnetIndexRecordMapper {
    #[must_use]
    pub fn record_to_input(data: SubnetIndexRecord) -> SubnetIndexArgs {
        SubnetIndexArgs(record_entries_to_input(data.entries))
    }

    #[must_use]
    pub fn input_to_record(input: SubnetIndexArgs) -> SubnetIndexRecord {
        SubnetIndexRecord {
            entries: input_entries_to_record(input.0),
        }
    }
}

///
/// IndexResponseMapper
///

pub struct IndexResponseMapper;

impl IndexResponseMapper {
    #[must_use]
    pub fn record_page_to_response(
        page: Page<(CanisterRole, Principal)>,
    ) -> Page<IndexEntryResponse> {
        Page {
            entries: record_entries_to_response(page.entries),
            total: page.total,
        }
    }
}
