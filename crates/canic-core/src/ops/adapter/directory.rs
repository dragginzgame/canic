use crate::{
    dto::directory::{AppDirectoryView, SubnetDirectoryView},
    model::memory::directory::{AppDirectoryData, SubnetDirectoryData},
};

#[must_use]
pub fn app_directory_from_view(view: AppDirectoryView) -> AppDirectoryData {
    view.0
}

#[must_use]
pub fn subnet_directory_from_view(view: SubnetDirectoryView) -> SubnetDirectoryData {
    view.0
}

#[must_use]
pub fn app_directory_to_view(data: AppDirectoryData) -> AppDirectoryView {
    AppDirectoryView(data)
}

#[must_use]
pub fn subnet_directory_to_view(data: SubnetDirectoryData) -> SubnetDirectoryView {
    SubnetDirectoryView(data)
}
