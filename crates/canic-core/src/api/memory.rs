use crate::{dto::memory::MemoryRegistryView, workflow};

///
/// MemoryApi
///

pub struct MemoryApi;

impl MemoryApi {
    #[must_use]
    pub fn registry_view() -> MemoryRegistryView {
        workflow::memory::query::MemoryQuery::registry_view()
    }
}
