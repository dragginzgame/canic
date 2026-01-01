// workflow/cascade/directory.rs

use crate::{
    dto::directory::{AppDirectoryView, SubnetDirectoryView},
    ops::storage::directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
    storage::memory::directory::{app::AppDirectoryData, subnet::SubnetDirectoryData},
};

pub(crate) fn import_app_directory(view: AppDirectoryView) {
    let data = AppDirectoryData { entries: view.0 };
    AppDirectoryOps::import(data);
}

pub(crate) fn import_subnet_directory(view: SubnetDirectoryView) {
    let data = SubnetDirectoryData { entries: view.0 };
    SubnetDirectoryOps::import(data);
}
