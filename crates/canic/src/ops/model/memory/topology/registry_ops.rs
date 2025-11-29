use crate::model::memory::topology::{
    AppSubnetRegistry, AppSubnetRegistryView, SubnetCanisterRegistry,
};

pub struct TopologyRegistryOps;

impl TopologyRegistryOps {
    #[must_use]
    pub fn export_app_subnet_registry() -> AppSubnetRegistryView {
        AppSubnetRegistry::export()
    }

    #[must_use]
    pub fn export_subnet_canister_registry() -> Vec<crate::model::memory::CanisterEntry> {
        SubnetCanisterRegistry::export()
    }
}
