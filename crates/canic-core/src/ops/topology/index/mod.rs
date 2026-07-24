//! Module: ops::topology::index
//!
//! Responsibility: resolve app and subnet index snapshots for the current role.
//! Does not own: index storage, topology policy, or endpoint DTO schemas.
//! Boundary: ops resolver between workflow queries and storage/root registry state.

pub mod builder;

use crate::{
    InternalError,
    dto::topology::{FleetDirectoryInput, SubnetDirectoryInput},
    ops::{
        config::ConfigOps,
        runtime::env::EnvOps,
        storage::{
            index::{
                app::AppIndexOps,
                mapper::{AppIndexDataMapper, SubnetIndexDataMapper},
                subnet::SubnetIndexOps,
            },
            registry::subnet::SubnetRegistryOps,
        },
    },
    storage::stable::index::{app::AppIndexData, subnet::SubnetIndexData},
};

use self::builder::{RootAppIndexBuilder, RootSubnetIndexBuilder};

///
/// AppIndexResolver
///
/// Operations-layer resolver for app index snapshots.
///

pub struct AppIndexResolver;

impl AppIndexResolver {
    pub fn resolve() -> Result<AppIndexData, InternalError> {
        if EnvOps::is_root() {
            let registry = SubnetRegistryOps::data();
            let cfg = ConfigOps::get()?;

            RootAppIndexBuilder::build(&registry, &cfg.services.fleet.roles)
        } else {
            Ok(AppIndexOps::data())
        }
    }

    pub fn resolve_input() -> Result<FleetDirectoryInput, InternalError> {
        Self::resolve().map(AppIndexDataMapper::data_to_input)
    }
}

///
/// SubnetIndexResolver
///
/// Operations-layer resolver for subnet index snapshots.
///

pub struct SubnetIndexResolver;

impl SubnetIndexResolver {
    pub fn resolve() -> Result<SubnetIndexData, InternalError> {
        if EnvOps::is_root() {
            let registry = SubnetRegistryOps::data();
            let cfg = ConfigOps::current_subnet()?;

            RootSubnetIndexBuilder::build(&registry, &cfg.subnet_index_roles())
        } else {
            Ok(SubnetIndexOps::data())
        }
    }

    pub fn resolve_input() -> Result<SubnetDirectoryInput, InternalError> {
        Self::resolve().map(SubnetIndexDataMapper::data_to_input)
    }
}
