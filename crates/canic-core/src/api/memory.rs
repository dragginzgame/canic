use crate::{PublicError, dto::memory::MemoryRegistryView, workflow};

pub fn canic_memory_registry() -> Result<MemoryRegistryView, PublicError> {
    Ok(workflow::memory::query::memory_registry_view())
}
