use crate::{
    dto::registry::AppRegistryView, ops::adapter::registry::app_registry_to_view,
    storage::memory::registry::app::AppRegistry,
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
