use crate::{
    dto::directory::{AppDirectoryView, SubnetDirectoryView},
    storage::memory::directory::{app::AppDirectoryData, subnet::SubnetDirectoryData},
};

#[must_use]
pub fn app_directory_from_view(view: AppDirectoryView) -> AppDirectoryData {
    AppDirectoryData { entries: view.0 }
}

#[must_use]
pub fn subnet_directory_from_view(view: SubnetDirectoryView) -> SubnetDirectoryData {
    SubnetDirectoryData { entries: view.0 }
}

#[must_use]
pub fn app_directory_to_view(data: AppDirectoryData) -> AppDirectoryView {
    AppDirectoryView(data.entries)
}

#[must_use]
pub fn subnet_directory_to_view(data: SubnetDirectoryData) -> SubnetDirectoryView {
    SubnetDirectoryView(data.entries)
}
