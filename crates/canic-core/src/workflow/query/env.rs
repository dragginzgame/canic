use crate::{
    dto::{env::EnvView, memory::MemoryRegistryView},
    ops::runtime::{env::EnvOps, memory::MemoryOps},
};

pub(crate) fn env_view() -> EnvView {
    EnvOps::export_view()
}

pub(crate) fn memory_registry() -> MemoryRegistryView {
    MemoryOps::export_view()
}
