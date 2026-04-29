pub mod builder;

use crate::{
    InternalError,
    dto::topology::{AppIndexArgs, SubnetIndexArgs},
    ops::{
        config::ConfigOps,
        runtime::env::EnvOps,
        storage::{
            index::{
                app::AppIndexOps,
                mapper::{AppIndexRecordMapper, SubnetIndexRecordMapper},
                subnet::SubnetIndexOps,
            },
            registry::subnet::SubnetRegistryOps,
        },
    },
    storage::stable::index::{app::AppIndexRecord, subnet::SubnetIndexRecord},
};

use self::builder::{RootAppIndexBuilder, RootSubnetIndexBuilder};

///
/// AppIndexResolver
///

pub struct AppIndexResolver;

impl AppIndexResolver {
    pub fn resolve() -> Result<AppIndexRecord, InternalError> {
        if EnvOps::is_root() {
            let registry = SubnetRegistryOps::data();
            let cfg = ConfigOps::get()?;

            RootAppIndexBuilder::build(&registry, &cfg.app_index)
        } else {
            Ok(AppIndexOps::data())
        }
    }

    pub fn resolve_input() -> Result<AppIndexArgs, InternalError> {
        Self::resolve().map(AppIndexRecordMapper::record_to_input)
    }
}

///
/// SubnetIndexResolver
///

pub struct SubnetIndexResolver;

impl SubnetIndexResolver {
    pub fn resolve() -> Result<SubnetIndexRecord, InternalError> {
        if EnvOps::is_root() {
            let registry = SubnetRegistryOps::data();
            let cfg = ConfigOps::current_subnet()?;

            RootSubnetIndexBuilder::build(&registry, &cfg.subnet_index)
        } else {
            Ok(SubnetIndexOps::data())
        }
    }

    pub fn resolve_input() -> Result<SubnetIndexArgs, InternalError> {
        Self::resolve().map(SubnetIndexRecordMapper::record_to_input)
    }
}
