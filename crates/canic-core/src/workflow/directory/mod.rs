pub mod builders;

pub use builders::*;

use crate::{
    dto::directory::{AppDirectoryView, SubnetDirectoryView},
    ops::{
        adapter::directory::{app_directory_to_view, subnet_directory_to_view},
        env::EnvOps,
        storage::directory::{AppDirectoryOps, SubnetDirectoryOps},
    },
};

///
/// AppDirectoryResolver
///
/// Resolves the canonical AppDirectory view:
/// - Root rebuilds from registry
/// - Non-root uses imported snapshot
///
pub struct AppDirectoryResolver;

impl AppDirectoryResolver {
    #[must_use]
    pub fn resolve_view() -> AppDirectoryView {
        let data = if EnvOps::is_root() {
            RootAppDirectoryBuilder::build_from_registry()
        } else {
            AppDirectoryOps::export()
        };

        app_directory_to_view(data)
    }
}

///
/// SubnetDirectoryResolver
///
/// Resolves the canonical SubnetDirectory view:
/// - Root rebuilds from registry
/// - Non-root uses imported snapshot
///
pub struct SubnetDirectoryResolver;

impl SubnetDirectoryResolver {
    #[must_use]
    pub fn resolve_view() -> SubnetDirectoryView {
        let data = if EnvOps::is_root() {
            RootSubnetDirectoryBuilder::build_from_registry()
        } else {
            SubnetDirectoryOps::export()
        };

        subnet_directory_to_view(data)
    }
}
