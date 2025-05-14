use crate::memory::{
    MEMORY_REGISTRY,
    registry::{Registry, RegistryData},
};

// get_data
#[must_use]
pub fn get_data() -> RegistryData {
    MEMORY_REGISTRY.with_borrow(Registry::get_data)
}
