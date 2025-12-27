pub use crate::model::memory::registry::AppRegistryView;

use crate::model::memory::registry::AppRegistry;

///
/// AppRegistryOps
///

pub struct AppRegistryOps;

impl AppRegistryOps {
    #[must_use]
    pub fn export() -> AppRegistryView {
        AppRegistry::export()
    }
}
