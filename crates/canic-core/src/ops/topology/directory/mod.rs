pub mod builder;

use crate::{
    InternalError,
    ops::{
        config::ConfigOps,
        runtime::env::EnvOps,
        storage::{
            directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
            registry::subnet::SubnetRegistryOps,
        },
    },
    storage::stable::directory::{app::AppDirectoryData, subnet::SubnetDirectoryData},
};

use self::builder::{RootAppDirectoryBuilder, RootSubnetDirectoryBuilder};

///
/// AppDirectoryResolver
///

pub struct AppDirectoryResolver;

impl AppDirectoryResolver {
    pub fn resolve() -> Result<AppDirectoryData, InternalError> {
        if EnvOps::is_root() {
            let registry = SubnetRegistryOps::data();
            let cfg = ConfigOps::get().expect("config must be available on root");

            RootAppDirectoryBuilder::build(&registry, &cfg.app_directory)
        } else {
            Ok(AppDirectoryOps::data())
        }
    }
}

///
/// SubnetDirectoryResolver
///

pub struct SubnetDirectoryResolver;

impl SubnetDirectoryResolver {
    pub fn resolve() -> Result<SubnetDirectoryData, InternalError> {
        if EnvOps::is_root() {
            let registry = SubnetRegistryOps::data();
            let cfg = ConfigOps::current_subnet().expect("subnet config must be available on root");

            RootSubnetDirectoryBuilder::build(&registry, &cfg.subnet_directory)
        } else {
            Ok(SubnetDirectoryOps::data())
        }
    }
}
