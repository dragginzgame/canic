use crate::storage::stable::registry::app::{AppRegistry, AppRegistryRecord};

///
/// AppRegistryOps
///

pub struct AppRegistryOps;

impl AppRegistryOps {
    // -------------------------------------------------------------
    // Canonical data access
    // -------------------------------------------------------------

    #[must_use]
    pub fn data() -> AppRegistryRecord {
        AppRegistry::export()
    }
}
