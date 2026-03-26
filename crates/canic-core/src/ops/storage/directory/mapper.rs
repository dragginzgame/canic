use crate::{
    cdk::types::Principal,
    dto::{
        page::Page,
        topology::{
            AppDirectoryArgs, DirectoryEntryInput, DirectoryEntryResponse, SubnetDirectoryArgs,
        },
    },
    ids::CanisterRole,
    storage::stable::directory::{app::AppDirectoryRecord, subnet::SubnetDirectoryRecord},
};

// --- Helpers ---------------------------------------------------------------

// Map stored directory tuples into the shared DTO entry shape.
fn record_entries_to_dto(entries: Vec<(CanisterRole, Principal)>) -> Vec<DirectoryEntryInput> {
    entries
        .into_iter()
        .map(|(role, pid)| DirectoryEntryInput { role, pid })
        .collect()
}

// Map stored directory tuples into the public response entry shape.
fn record_entries_to_response(
    entries: Vec<(CanisterRole, Principal)>,
) -> Vec<DirectoryEntryResponse> {
    entries
        .into_iter()
        .map(|(role, pid)| DirectoryEntryResponse { role, pid })
        .collect()
}

// Map DTO entry snapshots back into stored directory tuples.
fn dto_entries_to_record(entries: Vec<DirectoryEntryInput>) -> Vec<(CanisterRole, Principal)> {
    entries
        .into_iter()
        .map(|entry| (entry.role, entry.pid))
        .collect()
}

///
/// AppDirectoryRecordMapper
///

pub struct AppDirectoryRecordMapper;

impl AppDirectoryRecordMapper {
    #[must_use]
    pub fn record_to_view(data: AppDirectoryRecord) -> AppDirectoryArgs {
        AppDirectoryArgs(record_entries_to_dto(data.entries))
    }

    #[must_use]
    pub fn dto_to_record(view: AppDirectoryArgs) -> AppDirectoryRecord {
        // TODO: mapping from DTO to storage record must remain in ops.
        AppDirectoryRecord {
            entries: dto_entries_to_record(view.0),
        }
    }
}

///
/// SubnetDirectoryRecordMapper
///

pub struct SubnetDirectoryRecordMapper;

impl SubnetDirectoryRecordMapper {
    #[must_use]
    pub fn record_to_view(data: SubnetDirectoryRecord) -> SubnetDirectoryArgs {
        SubnetDirectoryArgs(record_entries_to_dto(data.entries))
    }

    #[must_use]
    pub fn dto_to_record(view: SubnetDirectoryArgs) -> SubnetDirectoryRecord {
        // TODO: mapping from DTO to storage record must remain in ops.
        SubnetDirectoryRecord {
            entries: dto_entries_to_record(view.0),
        }
    }
}

///
/// DirectoryResponseMapper
///

pub struct DirectoryResponseMapper;

impl DirectoryResponseMapper {
    #[must_use]
    pub fn record_page_to_response(
        page: Page<(CanisterRole, Principal)>,
    ) -> Page<DirectoryEntryResponse> {
        Page {
            entries: record_entries_to_response(page.entries),
            total: page.total,
        }
    }
}
