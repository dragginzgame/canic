//! Module: workflow::ic::provision::directories
//!
//! Responsibility: rebuild topology Directories after provisioning registry changes.
//! Does not own: Directory storage schemas, registry mutation, or cascade sync execution.
//! Boundary: imports rebuilt Directory snapshots and returns cascade sections to synchronize.

use crate::{
    InternalError,
    ids::CanisterRole,
    ops::{
        config::ConfigOps,
        storage::{
            directory::{fleet::FleetDirectoryOps, subnet::SubnetDirectoryOps},
            registry::subnet::SubnetRegistryOps,
        },
        topology::directory::builder::{RootFleetDirectoryBuilder, RootSubnetDirectoryBuilder},
    },
    workflow::{cascade::snapshot::StateSnapshotBuilder, ic::provision::ProvisionWorkflow},
};

impl ProvisionWorkflow {
    /// Rebuild FleetDirectory and SubnetDirectory from the registry,
    /// import them directly, and return a builder containing the sections to sync.
    ///
    /// When `updated_role` is provided, only include the sections that list that role.
    pub fn rebuild_directories_from_registry(
        updated_role: Option<&CanisterRole>,
    ) -> Result<StateSnapshotBuilder, InternalError> {
        let cfg = ConfigOps::get()?;
        let subnet_cfg = ConfigOps::current_subnet()?;
        let registry = SubnetRegistryOps::data();
        let allow_incomplete = updated_role.is_some();
        let subnet_directory_roles = subnet_cfg.subnet_directory_roles();

        let include_fleet = updated_role.is_none_or(|role| cfg.services.fleet.roles.contains(role));
        let include_subnet = updated_role.is_none_or(|role| subnet_directory_roles.contains(role));

        let mut builder = StateSnapshotBuilder::new()?;

        if include_fleet {
            let fleet_data =
                RootFleetDirectoryBuilder::build(&registry, &cfg.services.fleet.roles)?;

            if allow_incomplete {
                FleetDirectoryOps::import_trusted_partial(fleet_data)?;
            } else {
                FleetDirectoryOps::import(fleet_data)?;
            }
            builder = builder.with_fleet_directory()?;
        }

        if include_subnet {
            let subnet_data =
                RootSubnetDirectoryBuilder::build(&registry, &subnet_directory_roles)?;

            if allow_incomplete {
                SubnetDirectoryOps::import_trusted_partial(subnet_data)?;
            } else {
                SubnetDirectoryOps::import(subnet_data)?;
            }
            builder = builder.with_subnet_directory()?;
        }

        Ok(builder)
    }
}
