pub mod builders;

pub use builders::*;

use crate::{
    dto::directory::{AppDirectoryView, SubnetDirectoryView},
    ops::{
        runtime::env::EnvOps,
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
        if EnvOps::is_root() {
            RootAppDirectoryBuilder::build_from_registry()
        } else {
            AppDirectoryOps::export_view()
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
    pub fn resolve_view() -> SubnetDirectoryView {
        if EnvOps::is_root() {
            RootSubnetDirectoryBuilder::build_from_registry()
        } else {
            SubnetDirectoryOps::export_view()
        }
    }
}
