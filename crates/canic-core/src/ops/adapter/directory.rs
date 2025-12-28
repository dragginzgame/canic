use crate::{
    dto::directory::{AppDirectoryView, SubnetDirectoryView},
    model::memory::directory::{AppDirectoryData, SubnetDirectoryData},
};

#[must_use]
pub fn app_directory_from_dto(view: AppDirectoryView) -> AppDirectoryData {
    view.0
}

#[must_use]
pub fn subnet_directory_from_dto(view: SubnetDirectoryView) -> SubnetDirectoryData {
    view.0
}
