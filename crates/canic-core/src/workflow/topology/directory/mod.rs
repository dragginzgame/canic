pub mod builder;
pub mod mapper;
pub mod query;

use crate::{
    ops::{
        runtime::env,
        storage::directory::{
            app::{AppDirectoryOps, AppDirectorySnapshot},
            subnet::{SubnetDirectoryOps, SubnetDirectorySnapshot},
        },
    },
    workflow::topology::directory::builder::{RootAppDirectoryBuilder, RootSubnetDirectoryBuilder},
};

///
/// AppDirectoryResolver
///

pub struct AppDirectoryResolver;

impl AppDirectoryResolver {
    #[must_use]
    pub fn resolve() -> AppDirectorySnapshot {
        if env::is_root() {
            RootAppDirectoryBuilder::build_from_registry()
        } else {
            AppDirectoryOps::snapshot()
        }
    }
}

///
/// SubnetDirectoryResolver
///

pub struct SubnetDirectoryResolver;

impl SubnetDirectoryResolver {
    #[must_use]
    pub fn resolve() -> SubnetDirectorySnapshot {
        if env::is_root() {
            RootSubnetDirectoryBuilder::build_from_registry()
        } else {
            SubnetDirectoryOps::snapshot()
        }
    }
}
