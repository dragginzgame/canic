pub mod adapter;
pub mod builder;

use crate::{
    dto::directory::{AppDirectoryView, SubnetDirectoryView},
    ops::{
        runtime::env::EnvOps,
        storage::directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
    },
    workflow::directory::{
        adapter::{app_directory_view_from_snapshot, subnet_directory_view_from_snapshot},
        builder::{RootAppDirectoryBuilder, RootSubnetDirectoryBuilder},
    },
};

///
/// AppDirectoryResolver
///

pub struct AppDirectoryResolver;

impl AppDirectoryResolver {
    #[must_use]
    pub fn resolve_view() -> AppDirectoryView {
        if EnvOps::is_root() {
            RootAppDirectoryBuilder::build_from_registry()
        } else {
            let snapshot = AppDirectoryOps::snapshot();
            app_directory_view_from_snapshot(snapshot)
        }
    }
}

///
/// SubnetDirectoryResolver
///

pub struct SubnetDirectoryResolver;

impl SubnetDirectoryResolver {
    #[must_use]
    pub fn resolve_view() -> SubnetDirectoryView {
        if EnvOps::is_root() {
            RootSubnetDirectoryBuilder::build_from_registry()
        } else {
            let snapshot = SubnetDirectoryOps::snapshot();
            subnet_directory_view_from_snapshot(snapshot)
        }
    }
}
