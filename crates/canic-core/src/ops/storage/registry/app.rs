use crate::storage::stable::registry::app::AppRegistry;
pub use crate::storage::stable::registry::app::AppRegistryData;

///
/// AppRegistryOps
///

pub struct AppRegistryOps;

impl AppRegistryOps {
    // -------------------------------------------------------------
    // Canonical data access
    // -------------------------------------------------------------

    #[must_use]
    pub fn data() -> AppRegistryData {
        AppRegistry::export()
    }
}
