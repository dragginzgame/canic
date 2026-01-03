use crate::{
    dto::topology::{AppDirectoryView, DirectoryEntryView, SubnetDirectoryView},
    ops::storage::directory::{app::AppDirectorySnapshot, subnet::SubnetDirectorySnapshot},
};

///
/// AppDirectoryMapper
///

pub struct AppDirectoryMapper;

impl AppDirectoryMapper {
    #[must_use]
    pub fn snapshot_to_view(snapshot: AppDirectorySnapshot) -> AppDirectoryView {
        let entries = snapshot
            .entries
            .into_iter()
            .map(|(role, pid)| DirectoryEntryView { role, pid })
            .collect();

        AppDirectoryView(entries)
    }

    #[must_use]
    pub fn view_to_snapshot(view: AppDirectoryView) -> AppDirectorySnapshot {
        let entries = view
            .0
            .into_iter()
            .map(|entry| (entry.role, entry.pid))
            .collect();

        AppDirectorySnapshot { entries }
    }
}

///
/// SubnetDirectoryMapper
///

pub struct SubnetDirectoryMapper;

impl SubnetDirectoryMapper {
    #[must_use]
    pub fn snapshot_to_view(snapshot: SubnetDirectorySnapshot) -> SubnetDirectoryView {
        let entries = snapshot
            .entries
            .into_iter()
            .map(|(role, pid)| DirectoryEntryView { role, pid })
            .collect();

        SubnetDirectoryView(entries)
    }

    #[must_use]
    pub fn view_to_snapshot(view: SubnetDirectoryView) -> SubnetDirectorySnapshot {
        let entries = view
            .0
            .into_iter()
            .map(|entry| (entry.role, entry.pid))
            .collect();

        SubnetDirectorySnapshot { entries }
    }
}
