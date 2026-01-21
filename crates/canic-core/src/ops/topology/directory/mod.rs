pub mod builder;

use crate::{
    InternalError,
    dto::topology::{AppDirectoryArgs, SubnetDirectoryArgs},
    ops::{
        config::ConfigOps,
        runtime::env::EnvOps,
        storage::{
            directory::{
                app::AppDirectoryOps,
                mapper::{AppDirectoryRecordMapper, SubnetDirectoryRecordMapper},
                subnet::SubnetDirectoryOps,
            },
            registry::subnet::SubnetRegistryOps,
        },
    },
    storage::stable::directory::{app::AppDirectoryRecord, subnet::SubnetDirectoryRecord},
};

use self::builder::{RootAppDirectoryBuilder, RootSubnetDirectoryBuilder};

///
/// AppDirectoryResolver
///

pub struct AppDirectoryResolver;

impl AppDirectoryResolver {
    pub fn resolve() -> Result<AppDirectoryRecord, InternalError> {
        if EnvOps::is_root() {
            let registry = SubnetRegistryOps::data();
            let cfg = ConfigOps::get().expect("config must be available on root");

            RootAppDirectoryBuilder::build(&registry, &cfg.app_directory)
        } else {
            Ok(AppDirectoryOps::data())
        }
    }

    pub fn resolve_input() -> Result<AppDirectoryArgs, InternalError> {
        Self::resolve().map(AppDirectoryRecordMapper::record_to_view)
    }
}

///
/// SubnetDirectoryResolver
///

pub struct SubnetDirectoryResolver;

impl SubnetDirectoryResolver {
    pub fn resolve() -> Result<SubnetDirectoryRecord, InternalError> {
        if EnvOps::is_root() {
            let registry = SubnetRegistryOps::data();
            let cfg = ConfigOps::current_subnet().expect("subnet config must be available on root");

            RootSubnetDirectoryBuilder::build(&registry, &cfg.subnet_directory)
        } else {
            Ok(SubnetDirectoryOps::data())
        }
    }

    pub fn resolve_input() -> Result<SubnetDirectoryArgs, InternalError> {
        Self::resolve().map(SubnetDirectoryRecordMapper::record_to_view)
    }
}
