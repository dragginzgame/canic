//! Module: blob_service::ops::memory
//!
//! Responsibility: contribute service requests and open handles from the Canic-owned runtime.
//! Does not own: memory bootstrap, placement policy or storage schemas.
//! Boundary: Canic bootstraps once before the lifecycle participant opens the grants.

use ic_blob_storage::ops::service::{
    installation::{self, INSTALLATION_MEMORY_KEY, ServiceInstallationMemories},
    stores::grants,
};
use ic_memory::{DefaultMemoryImpl, RuntimeMemory};

pub(super) type Memory = DefaultMemoryImpl;

/// Complete service requests for the composing host; no registration or bootstrap.
pub fn requests() -> Result<Vec<ic_memory::MemoryRequest>, ic_memory::MemoryDeclarationError> {
    installation::requests(crate::MEMORY_AUTHORITY)
}

/// Dedicated service composition, including the selected Canic framework.
///
/// # Panics
/// Panics if authored service requests or grants are invalid.
#[must_use]
pub fn host() -> canic::memory::MemoryBootstrap {
    let mut input = canic::memory::framework();
    input.requests.extend(requests().expect("service requests"));
    input.pool = canic::memory::pool::framework_pool(
        vec![
            ic_memory::MemoryAuthority::new(crate::MEMORY_AUTHORITY, crate::MEMORY_KEY_PREFIX)
                .expect("service authority"),
        ],
        Vec::new(),
    )
    .expect("service grants");
    input
}

pub(super) fn open() -> ServiceInstallationMemories<RuntimeMemory<Memory>> {
    let lookup = |key: &str| Ok::<_, ic_memory::RuntimeOpenError>(canic::memory::open(key));
    ServiceInstallationMemories {
        configuration: lookup(INSTALLATION_MEMORY_KEY).expect("configuration grant"),
        stores: grants::open(lookup).expect("service grants"),
    }
}
