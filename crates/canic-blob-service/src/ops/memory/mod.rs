//! Module: blob_service::ops::memory
//!
//! Responsibility: contribute service requests and open handles from the Canic-owned runtime.
//! Does not own: memory bootstrap, placement policy or storage schemas.
//! Boundary: Canic bootstraps once before the lifecycle participant opens the grants.

use ic_blob_storage::ops::service::{
    installation::{self, INSTALLATION_MEMORY_KEY, ServiceInstallationMemories},
    stores::grants,
};
use ic_storage::{DefaultMemoryImpl, RuntimeMemory, StableKey};

pub(super) type Memory = DefaultMemoryImpl;

/// Complete service requests for the composing host; no registration or bootstrap.
pub fn requests() -> Result<Vec<ic_storage::MemoryRequest>, ic_storage::AdmissionError> {
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
    input.grants = canic::memory::grants::framework_grants(vec![
        ic_storage::MemoryAuthority::new(crate::MEMORY_AUTHORITY, crate::MEMORY_KEY_PREFIX)
            .expect("service authority"),
    ])
    .expect("service grants");
    input
}

pub(super) fn open() -> ServiceInstallationMemories<RuntimeMemory<Memory>> {
    let runtime = canic::memory::runtime::shared_runtime().expect("bootstrapped host runtime");
    let lookup = |key: &StableKey| runtime.open_memory(key);
    let installation_key = StableKey::parse(INSTALLATION_MEMORY_KEY).expect("configuration key");
    ServiceInstallationMemories {
        configuration: lookup(&installation_key).expect("configuration grant"),
        stores: grants::open(lookup).expect("service grants"),
    }
}
