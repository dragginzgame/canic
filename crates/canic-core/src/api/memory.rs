use crate::{dto::memory::MemoryRegistryView, workflow};

///
/// Memory API
///

#[must_use]
pub fn memory_registry() -> MemoryRegistryView {
    workflow::memory::query::memory_registry_view()
}
