pub use crate::model::memory::topology::AppSubnetRegistryView;

use crate::model::memory::topology::AppSubnetRegistry;

//
// Stable-memory adapter
//

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
