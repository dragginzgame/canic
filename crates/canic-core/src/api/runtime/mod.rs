use crate::{dto::error::Error, ops::runtime::memory::MemoryRegistryOps};

///
/// MemoryRuntimeApi
///

pub struct MemoryRuntimeApi;

impl MemoryRuntimeApi {
    /// Bootstrap Canic's reserved stable-memory range and flush deferred registrations.
    pub fn bootstrap_registry() -> Result<(), Error> {
        let _ = MemoryRegistryOps::bootstrap_registry().map_err(Error::from)?;

        Ok(())
    }
}
