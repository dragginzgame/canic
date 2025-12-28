use crate::{
    dto::registry::AppRegistryView, model::memory::registry::AppRegistry,
    ops::adapter::registry::app_registry_to_view,
};

///
/// AppRegistryOps
///

pub struct AppRegistryOps;

impl AppRegistryOps {
    #[must_use]
    pub fn export_view() -> AppRegistryView {
        let data = AppRegistry::export();

        app_registry_to_view(data)
    }
}
