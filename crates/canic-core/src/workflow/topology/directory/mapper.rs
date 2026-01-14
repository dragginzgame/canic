use crate::{
    dto::topology::{AppDirectoryView, DirectoryEntryView, SubnetDirectoryView},
    storage::stable::directory::{app::AppDirectoryData, subnet::SubnetDirectoryData},
};

///
/// AppDirectoryMapper
///

pub struct AppDirectoryMapper;

impl AppDirectoryMapper {
    #[must_use]
    pub fn data_to_view(data: AppDirectoryData) -> AppDirectoryView {
        let entries = data
            .entries
            .into_iter()
            .map(|(role, pid)| DirectoryEntryView { role, pid })
            .collect();

        AppDirectoryView(entries)
    }

    #[must_use]
    pub fn view_to_data(view: AppDirectoryView) -> AppDirectoryData {
        let entries = view
            .0
            .into_iter()
            .map(|entry| (entry.role, entry.pid))
            .collect();

        AppDirectoryData { entries }
    }
}

///
/// SubnetDirectoryMapper
///

pub struct SubnetDirectoryMapper;

impl SubnetDirectoryMapper {
    #[must_use]
    pub fn data_to_view(data: SubnetDirectoryData) -> SubnetDirectoryView {
        let entries = data
            .entries
            .into_iter()
            .map(|(role, pid)| DirectoryEntryView { role, pid })
            .collect();

        SubnetDirectoryView(entries)
    }

    #[must_use]
    pub fn view_to_data(view: SubnetDirectoryView) -> SubnetDirectoryData {
        let entries = view
            .0
            .into_iter()
            .map(|entry| (entry.role, entry.pid))
            .collect();

        SubnetDirectoryData { entries }
    }
}
