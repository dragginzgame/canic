use crate::{
    InternalError,
    ops::{
        config::ConfigOps,
        storage::{
            index::{app::AppIndexOps, subnet::SubnetIndexOps},
            registry::subnet::SubnetRegistryOps,
        },
        topology::index::builder::{RootAppIndexBuilder, RootSubnetIndexBuilder},
    },
    workflow::{
        cascade::snapshot::StateSnapshotBuilder, ic::provision::ProvisionWorkflow, prelude::*,
    },
};

impl ProvisionWorkflow {
    /// Rebuild AppIndex and SubnetIndex from the registry,
    /// import them directly, and return a builder containing the sections to sync.
    ///
    /// When `updated_role` is provided, only include the sections that list that role.
    pub fn rebuild_indexes_from_registry(
        updated_role: Option<&CanisterRole>,
    ) -> Result<StateSnapshotBuilder, InternalError> {
        let cfg = ConfigOps::get()?;
        let subnet_cfg = ConfigOps::current_subnet()?;
        let registry = SubnetRegistryOps::data();
        let allow_incomplete = updated_role.is_some();

        let include_app = updated_role.is_none_or(|role| cfg.app_index.contains(role));
        let include_subnet = updated_role.is_none_or(|role| subnet_cfg.subnet_index.contains(role));

        let mut builder = StateSnapshotBuilder::new()?;

        if include_app {
            let app_data = RootAppIndexBuilder::build(&registry, &cfg.app_index)?;

            if allow_incomplete {
                AppIndexOps::import_allow_incomplete(app_data)?;
            } else {
                AppIndexOps::import(app_data)?;
            }
            builder = builder.with_app_index()?;
        }

        if include_subnet {
            let subnet_data = RootSubnetIndexBuilder::build(&registry, &subnet_cfg.subnet_index)?;

            if allow_incomplete {
                SubnetIndexOps::import_allow_incomplete(subnet_data)?;
            } else {
                SubnetIndexOps::import(subnet_data)?;
            }
            builder = builder.with_subnet_index()?;
        }

        Ok(builder)
    }
}
