use serde::{Deserialize, Serialize};

/// Substrate identifier for `ic-stable-structures::MemoryManager` slots.
pub const MEMORY_MANAGER_SUBSTRATE: &str = "ic-stable-structures.memory_manager";

/// Descriptor version for current `MemoryManagerId` slots.
pub const MEMORY_MANAGER_DESCRIPTOR_VERSION: u32 = 1;

/// First usable `MemoryManager` virtual memory ID.
pub const MEMORY_MANAGER_MIN_ID: u8 = 0;

/// Last usable `MemoryManager` virtual memory ID.
pub const MEMORY_MANAGER_MAX_ID: u8 = 254;

/// `MemoryManager` unallocated-bucket sentinel. This is not a usable slot.
pub const MEMORY_MANAGER_INVALID_ID: u8 = u8::MAX;

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
            substrate: MEMORY_MANAGER_SUBSTRATE.to_string(),
            descriptor_version: MEMORY_MANAGER_DESCRIPTOR_VERSION,
        }
    }

    /// Construct a descriptor for a usable `MemoryManager` virtual memory ID.
    pub fn memory_manager_checked(id: u8) -> Result<Self, MemoryManagerSlotError> {
        validate_memory_manager_id(id)?;
        Ok(Self::memory_manager(id))
    }

    /// Return the usable `MemoryManager` virtual memory ID represented by this descriptor.
    pub fn memory_manager_id(&self) -> Result<u8, MemoryManagerSlotError> {
        if self.substrate != MEMORY_MANAGER_SUBSTRATE {
            return Err(MemoryManagerSlotError::UnsupportedSubstrate {
                substrate: self.substrate.clone(),
            });
        }
        if self.descriptor_version != MEMORY_MANAGER_DESCRIPTOR_VERSION {
            return Err(MemoryManagerSlotError::UnsupportedDescriptorVersion {
                version: self.descriptor_version,
            });
        }

        let AllocationSlot::MemoryManagerId(id) = self.slot else {
            return Err(MemoryManagerSlotError::UnsupportedSlot);
        };

        validate_memory_manager_id(id)?;
        Ok(id)
    }
}

///
/// MemoryManagerSlotError
///
/// Invalid or unsupported `MemoryManager` allocation slot descriptor.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum MemoryManagerSlotError {
    /// Descriptor is not a `MemoryManagerId` slot.
    #[error("allocation slot is not a MemoryManager virtual memory ID")]
    UnsupportedSlot,
    /// Descriptor is attached to another substrate.
    #[error("allocation slot substrate '{substrate}' is not supported as a MemoryManager slot")]
    UnsupportedSubstrate {
        /// Unsupported substrate identifier.
        substrate: String,
    },
    /// Descriptor uses an unsupported encoding version.
    #[error("MemoryManager slot descriptor version {version} is unsupported")]
    UnsupportedDescriptorVersion {
        /// Unsupported descriptor version.
        version: u32,
    },
    /// ID 255 is the unallocated-bucket sentinel.
    #[error("MemoryManager ID {id} is not a usable allocation slot")]
    InvalidMemoryManagerId {
        /// Invalid MemoryManager ID.
        id: u8,
    },
}

///
/// MemoryManagerIdRange
///
/// Inclusive range of usable `MemoryManager` virtual memory IDs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryManagerIdRange {
    start: u8,
    end: u8,
}

impl MemoryManagerIdRange {
    /// Construct and validate an inclusive `MemoryManager` ID range.
    pub const fn new(start: u8, end: u8) -> Result<Self, MemoryManagerRangeError> {
        if start > end {
            return Err(MemoryManagerRangeError::InvalidRange { start, end });
        }
        if start == MEMORY_MANAGER_INVALID_ID {
            return Err(MemoryManagerRangeError::InvalidMemoryManagerId { id: start });
        }
        if end == MEMORY_MANAGER_INVALID_ID {
            return Err(MemoryManagerRangeError::InvalidMemoryManagerId { id: end });
        }
        Ok(Self { start, end })
    }

    /// Return true when `id` is inside this inclusive range.
    #[must_use]
    pub const fn contains(&self, id: u8) -> bool {
        id >= self.start && id <= self.end
    }

    /// First usable ID in the range.
    #[must_use]
    pub const fn start(&self) -> u8 {
        self.start
    }

    /// Last usable ID in the range.
    #[must_use]
    pub const fn end(&self) -> u8 {
        self.end
    }
}

///
/// MemoryManagerRangeError
///
/// Invalid `MemoryManager` virtual memory ID range.
#[derive(Clone, Copy, Debug, Eq, thiserror::Error, PartialEq)]
pub enum MemoryManagerRangeError {
    /// Range bounds are reversed.
    #[error("MemoryManager ID range is invalid: start={start} end={end}")]
    InvalidRange {
        /// Requested first ID.
        start: u8,
        /// Requested last ID.
        end: u8,
    },
    /// ID 255 is the unallocated-bucket sentinel.
    #[error("MemoryManager ID {id} is not a usable allocation slot")]
    InvalidMemoryManagerId {
        /// Invalid MemoryManager ID.
        id: u8,
    },
}

/// Validate that a `MemoryManager` ID is usable as an allocation slot.
pub const fn validate_memory_manager_id(id: u8) -> Result<(), MemoryManagerSlotError> {
    if id == MEMORY_MANAGER_INVALID_ID {
        return Err(MemoryManagerSlotError::InvalidMemoryManagerId { id });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_manager_checked_accepts_usable_ids() {
        assert!(AllocationSlotDescriptor::memory_manager_checked(MEMORY_MANAGER_MIN_ID).is_ok());
        assert!(AllocationSlotDescriptor::memory_manager_checked(MEMORY_MANAGER_MAX_ID).is_ok());
    }

    #[test]
    fn memory_manager_checked_rejects_sentinel() {
        let err = AllocationSlotDescriptor::memory_manager_checked(MEMORY_MANAGER_INVALID_ID)
            .expect_err("sentinel must fail");

        assert_eq!(
            err,
            MemoryManagerSlotError::InvalidMemoryManagerId {
                id: MEMORY_MANAGER_INVALID_ID
            }
        );
    }

    #[test]
    fn memory_manager_id_validates_descriptor_shape() {
        let slot = AllocationSlotDescriptor::memory_manager(42);
        assert_eq!(slot.memory_manager_id().expect("usable ID"), 42);

        let err = AllocationSlotDescriptor {
            slot: AllocationSlot::NamedPartition("ledger".to_string()),
            substrate: MEMORY_MANAGER_SUBSTRATE.to_string(),
            descriptor_version: MEMORY_MANAGER_DESCRIPTOR_VERSION,
        }
        .memory_manager_id()
        .expect_err("slot kind should fail");
        assert_eq!(err, MemoryManagerSlotError::UnsupportedSlot);

        let err = AllocationSlotDescriptor {
            slot: AllocationSlot::MemoryManagerId(42),
            substrate: "other".to_string(),
            descriptor_version: MEMORY_MANAGER_DESCRIPTOR_VERSION,
        }
        .memory_manager_id()
        .expect_err("substrate should fail");
        assert_eq!(
            err,
            MemoryManagerSlotError::UnsupportedSubstrate {
                substrate: "other".to_string()
            }
        );

        let err = AllocationSlotDescriptor {
            slot: AllocationSlot::MemoryManagerId(42),
            substrate: MEMORY_MANAGER_SUBSTRATE.to_string(),
            descriptor_version: MEMORY_MANAGER_DESCRIPTOR_VERSION + 1,
        }
        .memory_manager_id()
        .expect_err("version should fail");
        assert_eq!(
            err,
            MemoryManagerSlotError::UnsupportedDescriptorVersion {
                version: MEMORY_MANAGER_DESCRIPTOR_VERSION + 1
            }
        );
    }

    #[test]
    fn memory_manager_range_accepts_usable_ranges() {
        let range = MemoryManagerIdRange::new(MEMORY_MANAGER_MIN_ID, MEMORY_MANAGER_MAX_ID)
            .expect("usable full range");

        assert!(range.contains(MEMORY_MANAGER_MIN_ID));
        assert!(range.contains(MEMORY_MANAGER_MAX_ID));
        assert!(!range.contains(MEMORY_MANAGER_INVALID_ID));
    }

    #[test]
    fn memory_manager_range_rejects_reversed_bounds() {
        let err = MemoryManagerIdRange::new(10, 9).expect_err("reversed range");

        assert_eq!(
            err,
            MemoryManagerRangeError::InvalidRange { start: 10, end: 9 }
        );
    }

    #[test]
    fn memory_manager_range_rejects_sentinel_bounds() {
        let err =
            MemoryManagerIdRange::new(240, MEMORY_MANAGER_INVALID_ID).expect_err("sentinel range");

        assert_eq!(
            err,
            MemoryManagerRangeError::InvalidMemoryManagerId {
                id: MEMORY_MANAGER_INVALID_ID
            }
        );
    }
}
