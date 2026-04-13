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

// Map stored index tuples into the shared DTO entry shape.
fn record_entries_to_dto(entries: Vec<(CanisterRole, Principal)>) -> Vec<IndexEntryInput> {
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

// Map DTO entry snapshots back into stored index tuples.
fn dto_entries_to_record(entries: Vec<IndexEntryInput>) -> Vec<(CanisterRole, Principal)> {
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
    pub fn record_to_view(data: AppIndexRecord) -> AppIndexArgs {
        AppIndexArgs(record_entries_to_dto(data.entries))
    }

    #[must_use]
    pub fn dto_to_record(view: AppIndexArgs) -> AppIndexRecord {
        // TODO: mapping from DTO to storage record must remain in ops.
        AppIndexRecord {
            entries: dto_entries_to_record(view.0),
        }
    }
}

///
/// SubnetIndexRecordMapper
///

pub struct SubnetIndexRecordMapper;

impl SubnetIndexRecordMapper {
    #[must_use]
    pub fn record_to_view(data: SubnetIndexRecord) -> SubnetIndexArgs {
        SubnetIndexArgs(record_entries_to_dto(data.entries))
    }

    #[must_use]
    pub fn dto_to_record(view: SubnetIndexArgs) -> SubnetIndexRecord {
        // TODO: mapping from DTO to storage record must remain in ops.
        SubnetIndexRecord {
            entries: dto_entries_to_record(view.0),
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
