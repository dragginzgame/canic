use crate::{
    dto::topology::{AppDirectoryView, SubnetDirectoryView},
    ops::storage::directory::{app::AppDirectorySnapshot, subnet::SubnetDirectorySnapshot},
};

///
/// AppDirectoryMapper
///

pub struct AppDirectoryMapper;

impl AppDirectoryMapper {
    #[must_use]
    pub fn snapshot_to_view(snapshot: AppDirectorySnapshot) -> AppDirectoryView {
        AppDirectoryView(snapshot.entries)
    }

    #[must_use]
    pub fn view_to_snapshot(view: AppDirectoryView) -> AppDirectorySnapshot {
        AppDirectorySnapshot { entries: view.0 }
    }
}

///
/// SubnetDirectoryMapper
///

pub struct SubnetDirectoryMapper;

impl SubnetDirectoryMapper {
    #[must_use]
    pub fn snapshot_to_view(snapshot: SubnetDirectorySnapshot) -> SubnetDirectoryView {
        SubnetDirectoryView(snapshot.entries)
    }

    #[must_use]
    pub fn view_to_snapshot(view: SubnetDirectoryView) -> SubnetDirectorySnapshot {
        SubnetDirectorySnapshot { entries: view.0 }
    }
}
