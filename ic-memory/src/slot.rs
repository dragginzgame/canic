use serde::{Deserialize, Serialize};

///
/// AllocationSlot
///
/// Durable physical allocation identity interpreted by a storage substrate.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum AllocationSlot {
    /// `ic-stable-structures::MemoryManager` virtual memory ID.
    MemoryManagerId(u8),
    /// Named substrate partition.
    NamedPartition(String),
    /// Forward-compatible custom slot descriptor.
    Custom {
        /// Substrate-defined slot kind.
        kind: String,
        /// Slot descriptor version.
        version: u32,
        /// Canonical substrate-defined value.
        value: Vec<u8>,
    },
}

///
/// AllocationSlotDescriptor
///
/// Encoded allocation slot persisted in the ledger.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct AllocationSlotDescriptor {
    /// Durable allocation slot.
    pub slot: AllocationSlot,
    /// Substrate identifier that interprets the slot.
    pub substrate: String,
    /// Descriptor encoding version.
    pub descriptor_version: u32,
}

impl AllocationSlotDescriptor {
    /// Construct a descriptor for a `MemoryManager` virtual memory ID.
    #[must_use]
    pub fn memory_manager(id: u8) -> Self {
        Self {
            slot: AllocationSlot::MemoryManagerId(id),
            substrate: "ic-stable-structures.memory_manager".to_string(),
            descriptor_version: 1,
        }
    }
}
