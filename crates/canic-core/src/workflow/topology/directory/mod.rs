pub mod builder;
pub mod mapper;
pub mod query;

use crate::{
    ops::{
        config::ConfigOps,
        runtime::env::EnvOps,
        storage::{
            directory::{
                app::{AppDirectoryOps, AppDirectorySnapshot},
                subnet::{SubnetDirectoryOps, SubnetDirectorySnapshot},
            },
            registry::subnet::SubnetRegistryOps,
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
        if EnvOps::is_root() {
            let registry = SubnetRegistryOps::snapshot();
            let cfg = ConfigOps::get().expect("config must be available on root");

            RootAppDirectoryBuilder::build(&registry, &cfg.app_directory)
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
        if EnvOps::is_root() {
            let registry = SubnetRegistryOps::snapshot();
            let cfg = ConfigOps::current_subnet().expect("subnet config must be available on root");

            RootSubnetDirectoryBuilder::build(&registry, &cfg.subnet_directory)
        } else {
            SubnetDirectoryOps::snapshot()
        }
    }
}
