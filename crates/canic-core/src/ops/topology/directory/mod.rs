//! Module: ops::topology::directory
//!
//! Responsibility: resolve Fleet Directory snapshots for the current role.
//! Does not own: Directory storage, topology policy, or endpoint DTO schemas.
//! Boundary: ops resolver between workflow queries and storage/root registry state.

pub mod builder;

use crate::{
    InternalError,
    dto::topology::{DirectoryProvenance, FleetDirectoryInput},
    ops::{
        config::ConfigOps,
        runtime::env::EnvOps,
        storage::{
            StorageOpsError,
            directory::{ensure_provenance, fleet::FleetDirectoryOps, records_to_input_entries},
            fleet_activation::FleetActivationOps,
            registry::subnet::SubnetRegistryOps,
        },
    },
    storage::stable::directory::fleet::FleetDirectoryData,
};

use self::builder::RootFleetDirectoryBuilder;

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
        let data = Self::resolve()?;
        Ok(FleetDirectoryInput {
            provenance: current_provenance()?,
            entries: records_to_input_entries(data.entries),
        })
    }
}

pub fn current_provenance() -> Result<DirectoryProvenance, InternalError> {
    Ok(DirectoryProvenance {
        fleet: FleetActivationOps::fleet_binding().map_err(StorageOpsError::from)?,
        source_root: EnvOps::fleet_subnet_root_pid()?,
    })
}

pub fn validate_provenance(provenance: &DirectoryProvenance) -> Result<(), InternalError> {
    let expected_fleet = FleetActivationOps::fleet_binding().map_err(StorageOpsError::from)?;
    let expected_source_root = EnvOps::fleet_subnet_root_pid()?;
    ensure_provenance(provenance, &expected_fleet, expected_source_root)?;
    Ok(())
}
