pub use crate::model::memory::topology::AppSubnetRegistryView;

use crate::model::memory::topology::AppSubnetRegistry;

///
/// AppSubnetRegistryOps
///

pub struct AppSubnetRegistryOps;

impl AppSubnetRegistryOps {
    #[must_use]
    pub fn export() -> AppSubnetRegistryView {
        AppSubnetRegistry::export()
    }
}
