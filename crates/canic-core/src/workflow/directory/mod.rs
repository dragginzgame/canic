pub mod builders;

pub use builders::*;

use crate::ops::{
    env::EnvOps,
    storage::directory::{AppDirectoryOps, DirectoryView, SubnetDirectoryOps},
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
    pub fn resolve_view() -> DirectoryView {
        if EnvOps::is_root() {
            RootAppDirectoryBuilder::build_from_registry()
        } else {
            AppDirectoryOps::export()
        }
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
    pub fn resolve_view() -> DirectoryView {
        if EnvOps::is_root() {
            RootSubnetDirectoryBuilder::build_from_registry()
        } else {
            SubnetDirectoryOps::export()
        }
    }
}
