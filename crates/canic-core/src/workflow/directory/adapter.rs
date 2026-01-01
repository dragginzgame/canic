use crate::{
    dto::directory::{AppDirectoryView, SubnetDirectoryView},
    ops::storage::directory::{app::AppDirectorySnapshot, subnet::SubnetDirectorySnapshot},
};

#[must_use]
pub fn app_directory_view_from_snapshot(snapshot: AppDirectorySnapshot) -> AppDirectoryView {
    AppDirectoryView(snapshot.entries)
}

#[must_use]
pub fn subnet_directory_view_from_snapshot(
    snapshot: SubnetDirectorySnapshot,
) -> SubnetDirectoryView {
    SubnetDirectoryView(snapshot.entries)
}
