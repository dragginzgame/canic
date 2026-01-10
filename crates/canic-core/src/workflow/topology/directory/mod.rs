pub mod builder;
pub mod mapper;
pub mod query;

use crate::{
    Error,
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
    pub fn resolve() -> Result<AppDirectorySnapshot, Error> {
        if EnvOps::is_root() {
            let registry = SubnetRegistryOps::snapshot();
            let cfg = ConfigOps::get().expect("config must be available on root");

            RootAppDirectoryBuilder::build(&registry, &cfg.app_directory)
        } else {
            Ok(AppDirectoryOps::snapshot())
        }
    }
}

///
/// SubnetDirectoryResolver
///

pub struct SubnetDirectoryResolver;

impl SubnetDirectoryResolver {
    pub fn resolve() -> Result<SubnetDirectorySnapshot, Error> {
        if EnvOps::is_root() {
            let registry = SubnetRegistryOps::snapshot();
            let cfg = ConfigOps::current_subnet().expect("subnet config must be available on root");

            RootSubnetDirectoryBuilder::build(&registry, &cfg.subnet_directory)
        } else {
            Ok(SubnetDirectoryOps::snapshot())
        }
    }
}
