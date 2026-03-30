pub mod install;

use crate::{
    CRATE_NAME,
    dto::error::Error,
    ops::runtime::memory::MemoryRegistryOpsError,
    storage::stable::{CANIC_MEMORY_MAX, CANIC_MEMORY_MIN},
};
use canic_memory::runtime::MemoryRuntimeApi as MemoryBootstrapApi;

///
/// MemoryRuntimeApi
///

pub struct MemoryRuntimeApi;

impl MemoryRuntimeApi {
    /// Bootstrap Canic's reserved stable-memory range and flush deferred registrations.
    pub fn bootstrap_registry() -> Result<(), Error> {
        let _ =
            MemoryBootstrapApi::bootstrap_registry(CRATE_NAME, CANIC_MEMORY_MIN, CANIC_MEMORY_MAX)
                .map_err(MemoryRegistryOpsError::from)
                .map_err(crate::InternalError::from)
                .map_err(Error::from)?;

        Ok(())
    }
}
