use crate::{
    dto::topology::{AppRegistryView, SubnetRegistryView},
    workflow,
};

///
/// Registry API
///

#[must_use]
pub fn app_registry() -> AppRegistryView {
    workflow::topology::registry::query::app_registry_view()
}

#[must_use]
pub fn subnet_registry() -> SubnetRegistryView {
    workflow::topology::registry::query::subnet_registry_view()
}
