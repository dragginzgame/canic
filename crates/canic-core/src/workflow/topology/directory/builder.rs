use crate::{
    domain::policy::topology::TopologyPolicy,
    ops::storage::{
        directory::{app::AppDirectorySnapshot, subnet::SubnetDirectorySnapshot},
        registry::subnet::SubnetRegistryOps,
    },
};

///
/// RootAppDirectoryBuilder
///

pub struct RootAppDirectoryBuilder;

impl RootAppDirectoryBuilder {
    #[must_use]
    pub fn build_from_registry() -> AppDirectorySnapshot {
        let registry = SubnetRegistryOps::snapshot();
        TopologyPolicy::app_directory_from_registry(&registry)
    }
}
///
/// RootSubnetDirectoryBuilder
///

pub struct RootSubnetDirectoryBuilder;

impl RootSubnetDirectoryBuilder {
    #[must_use]
    pub fn build_from_registry() -> SubnetDirectorySnapshot {
        let registry = SubnetRegistryOps::snapshot();
        TopologyPolicy::subnet_directory_from_registry(&registry)
    }
}
