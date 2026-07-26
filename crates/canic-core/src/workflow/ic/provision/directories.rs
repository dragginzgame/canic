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
        let registry = SubnetRegistryOps::data();
        let allow_incomplete = updated_role.is_some();
        let component_directory_roles = ConfigOps::current_subnet_directory_roles()?;

        let include_fleet =
            updated_role.is_none_or(|role| cfg.fleet_directory_roles().contains(role));
        let include_tree_directory =
            updated_role.is_none_or(|role| component_directory_roles.contains(role));

        let mut builder = StateSnapshotBuilder::new()?;

        if include_fleet {
            let fleet_data =
                RootFleetDirectoryBuilder::build(&registry, &cfg.fleet_directory_roles())?;

            if allow_incomplete {
                FleetDirectoryOps::import_trusted_partial(fleet_data)?;
            } else {
                FleetDirectoryOps::import(fleet_data)?;
            }
            builder = builder.with_fleet_directory()?;
        }

        if include_tree_directory {
            let subnet_data =
                RootSubnetDirectoryBuilder::build(&registry, &component_directory_roles)?;

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
