use crate::{
    dto::topology::{AppDirectoryArgs, DirectoryEntryInput, SubnetDirectoryArgs},
    storage::stable::directory::{app::AppDirectoryRecord, subnet::SubnetDirectoryRecord},
};

///
/// AppDirectoryRecordMapper
///

pub struct AppDirectoryRecordMapper;

impl AppDirectoryRecordMapper {
    #[must_use]
    pub fn record_to_view(data: AppDirectoryRecord) -> AppDirectoryArgs {
        let entries = data
            .entries
            .into_iter()
            .map(|(role, pid)| DirectoryEntryInput { role, pid })
            .collect();

        AppDirectoryArgs(entries)
    }

    #[must_use]
    pub fn dto_to_record(view: AppDirectoryArgs) -> AppDirectoryRecord {
        // TODO: mapping from DTO to storage record must remain in ops.
        let entries = view
            .0
            .into_iter()
            .map(|entry| (entry.role, entry.pid))
            .collect();

        AppDirectoryRecord { entries }
    }
}

///
/// SubnetDirectoryRecordMapper
///

pub struct SubnetDirectoryRecordMapper;

impl SubnetDirectoryRecordMapper {
    #[must_use]
    pub fn record_to_view(data: SubnetDirectoryRecord) -> SubnetDirectoryArgs {
        let entries = data
            .entries
            .into_iter()
            .map(|(role, pid)| DirectoryEntryInput { role, pid })
            .collect();

        SubnetDirectoryArgs(entries)
    }

    #[must_use]
    pub fn dto_to_record(view: SubnetDirectoryArgs) -> SubnetDirectoryRecord {
        // TODO: mapping from DTO to storage record must remain in ops.
        let entries = view
            .0
            .into_iter()
            .map(|entry| (entry.role, entry.pid))
            .collect();

        SubnetDirectoryRecord { entries }
    }
}
