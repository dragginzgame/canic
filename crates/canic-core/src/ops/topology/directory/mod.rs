//! Module: ops::topology::directory
//!
//! Responsibility: resolve Fleet and Subnet Directory snapshots for the current role.
//! Does not own: Directory storage, topology policy, or endpoint DTO schemas.
//! Boundary: ops resolver between workflow queries and storage/root registry state.

pub mod builder;

use crate::{
    InternalError,
    dto::topology::{DirectoryProvenance, FleetDirectoryInput, SubnetDirectoryInput},
    ops::{
        config::ConfigOps,
        runtime::env::EnvOps,
        storage::{
            StorageOpsError,
            directory::{
                ensure_provenance,
                fleet::FleetDirectoryOps,
                mapper::{FleetDirectoryDataMapper, SubnetDirectoryDataMapper},
                subnet::SubnetDirectoryOps,
            },
            fleet_activation::FleetActivationOps,
            registry::subnet::SubnetRegistryOps,
        },
    },
    storage::stable::directory::{fleet::FleetDirectoryData, subnet::SubnetDirectoryData},
};

use self::builder::{RootFleetDirectoryBuilder, RootSubnetDirectoryBuilder};

///
/// FleetDirectoryResolver
///
/// Operations-layer resolver for Fleet Directory snapshots.
///

pub struct FleetDirectoryResolver;

impl FleetDirectoryResolver {
    pub fn resolve() -> Result<FleetDirectoryData, InternalError> {
        if EnvOps::is_root() {
            let registry = SubnetRegistryOps::data();
            let cfg = ConfigOps::get()?;

            RootFleetDirectoryBuilder::build(&registry, &cfg.fleet_directory_roles())
        } else {
            Ok(FleetDirectoryOps::data())
        }
    }

    pub fn resolve_input() -> Result<FleetDirectoryInput, InternalError> {
        Ok(FleetDirectoryDataMapper::data_to_input(
            Self::resolve()?,
            current_provenance()?,
        ))
    }
}

///
/// SubnetDirectoryResolver
///
/// Operations-layer resolver for Subnet Directory snapshots.
///

pub struct SubnetDirectoryResolver;

impl SubnetDirectoryResolver {
    pub fn resolve() -> Result<SubnetDirectoryData, InternalError> {
        if EnvOps::is_root() {
            let registry = SubnetRegistryOps::data();
            let cfg = ConfigOps::current_component_spec()?;

            RootSubnetDirectoryBuilder::build(&registry, &cfg.component_directory_roles())
        } else {
            Ok(SubnetDirectoryOps::data())
        }
    }

    pub fn resolve_input() -> Result<SubnetDirectoryInput, InternalError> {
        Ok(SubnetDirectoryDataMapper::data_to_input(
            Self::resolve()?,
            current_provenance()?,
        ))
    }
}

pub fn current_provenance() -> Result<DirectoryProvenance, InternalError> {
    Ok(DirectoryProvenance {
        fleet: FleetActivationOps::fleet_binding().map_err(StorageOpsError::from)?,
        source_root: EnvOps::fleet_root_pid()?,
    })
}

pub fn validate_provenance(provenance: &DirectoryProvenance) -> Result<(), InternalError> {
    let expected_fleet = FleetActivationOps::fleet_binding().map_err(StorageOpsError::from)?;
    let expected_source_root = EnvOps::fleet_root_pid()?;
    ensure_provenance(provenance, &expected_fleet, expected_source_root)?;
    Ok(())
}
