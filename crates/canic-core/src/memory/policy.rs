use super::registry::{MemoryRange, MemoryRegistryError};
use ic_memory::{
    AllocationPolicy, AllocationSlotDescriptor, MemoryManagerIdRange, MemoryManagerSlotError,
    RangeAuthority, StableKey,
};

pub const IC_MEMORY_INTERNAL_MIN_ID: u8 = 0;
pub const IC_MEMORY_INTERNAL_MAX_ID: u8 = 9;
pub const CANIC_FRAMEWORK_MIN_ID: u8 = 10;
pub const CANIC_FRAMEWORK_MAX_ID: u8 = 99;
pub const APPLICATION_MIN_ID: u8 = 100;
pub const APPLICATION_MAX_ID: u8 = ic_memory::MEMORY_MANAGER_MAX_ID;

pub const IC_MEMORY_AUTHORITY_OWNER: &str = "ic-memory";
pub const IC_MEMORY_AUTHORITY_PURPOSE: &str = "ic-memory allocation-governance authority";
pub const CANIC_FRAMEWORK_AUTHORITY_OWNER: &str = "canic.framework";
pub const CANIC_FRAMEWORK_AUTHORITY_PURPOSE: &str = "Canic framework allocation authority";
pub const APPLICATION_AUTHORITY_OWNER: &str = "applications";
pub const APPLICATION_AUTHORITY_PURPOSE: &str = "downstream application allocation authority";

///
/// CanicMemoryManagerPolicy
///
/// Canic policy adapter for current `ic-stable-structures::MemoryManager`
/// allocation slots.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CanicMemoryManagerPolicy<'a> {
    declaring_crate: &'a str,
}

impl<'a> CanicMemoryManagerPolicy<'a> {
    #[must_use]
    pub(super) const fn for_declaring_crate(declaring_crate: &'a str) -> Self {
        Self { declaring_crate }
    }
}

impl AllocationPolicy for CanicMemoryManagerPolicy<'_> {
    type Error = MemoryRegistryError;

    fn validate_key(&self, _key: &StableKey) -> Result<(), Self::Error> {
        Ok(())
    }

    fn validate_slot(
        &self,
        key: &StableKey,
        slot: &AllocationSlotDescriptor,
    ) -> Result<(), Self::Error> {
        let id = slot
            .memory_manager_id()
            .map_err(memory_slot_error_to_registry_error)?;
        validate_key_id_claim(id, self.declaring_crate, key.as_str())
    }

    fn validate_reserved_slot(
        &self,
        key: &StableKey,
        slot: &AllocationSlotDescriptor,
    ) -> Result<(), Self::Error> {
        let id = slot
            .memory_manager_id()
            .map_err(memory_slot_error_to_registry_error)?;
        if !key.as_str().starts_with("ic_memory.") && !key.as_str().starts_with("canic.") {
            return Err(MemoryRegistryError::RangeAuthorityViolation {
                stable_key: key.as_str().to_string(),
                id,
                reason: "application stable keys may not be pre-reserved by Canic",
            });
        }
        validate_key_id_claim(id, self.declaring_crate, key.as_str())
    }
}

impl RangeAuthority for CanicMemoryManagerPolicy<'_> {
    type Error = MemoryRegistryError;

    fn validate_slot(&self, slot: &AllocationSlotDescriptor) -> Result<(), Self::Error> {
        slot.memory_manager_id()
            .map(drop)
            .map_err(memory_slot_error_to_registry_error)
    }
}

pub fn validate_stable_key_authority(
    id: u8,
    crate_name: &str,
    stable_key: &str,
) -> Result<(), MemoryRegistryError> {
    let key =
        StableKey::parse(stable_key).map_err(|err| MemoryRegistryError::InvalidStableKey {
            stable_key: err.stable_key,
            reason: err.reason,
        })?;

    let slot = AllocationSlotDescriptor::memory_manager_checked(id)
        .map_err(memory_slot_error_to_registry_error)?;
    let policy = CanicMemoryManagerPolicy::for_declaring_crate(crate_name);

    AllocationPolicy::validate_slot(&policy, &key, &slot)
}

#[must_use]
pub fn canonical_authority_ranges() -> Vec<(&'static str, MemoryRange, &'static str)> {
    vec![
        (
            IC_MEMORY_AUTHORITY_OWNER,
            MemoryRange {
                start: IC_MEMORY_INTERNAL_MIN_ID,
                end: IC_MEMORY_INTERNAL_MAX_ID,
            },
            IC_MEMORY_AUTHORITY_PURPOSE,
        ),
        (
            CANIC_FRAMEWORK_AUTHORITY_OWNER,
            MemoryRange {
                start: CANIC_FRAMEWORK_MIN_ID,
                end: CANIC_FRAMEWORK_MAX_ID,
            },
            CANIC_FRAMEWORK_AUTHORITY_PURPOSE,
        ),
        (
            APPLICATION_AUTHORITY_OWNER,
            MemoryRange {
                start: APPLICATION_MIN_ID,
                end: APPLICATION_MAX_ID,
            },
            APPLICATION_AUTHORITY_PURPOSE,
        ),
    ]
}

fn validate_key_id_claim(
    id: u8,
    crate_name: &str,
    stable_key: &str,
) -> Result<(), MemoryRegistryError> {
    if stable_key.starts_with("ic_memory.") {
        return validate_ic_memory_claim(id, crate_name, stable_key);
    }

    if stable_key.starts_with("canic.") {
        return validate_canic_claim(id, crate_name, stable_key);
    }

    validate_application_claim(id, stable_key)
}

fn validate_ic_memory_claim(
    id: u8,
    crate_name: &str,
    stable_key: &str,
) -> Result<(), MemoryRegistryError> {
    if crate_name != IC_MEMORY_AUTHORITY_OWNER {
        return Err(MemoryRegistryError::RangeAuthorityViolation {
            stable_key: stable_key.to_string(),
            id,
            reason: "ic_memory.* keys may only be declared by ic-memory",
        });
    }

    let range = MemoryManagerIdRange::new(IC_MEMORY_INTERNAL_MIN_ID, IC_MEMORY_INTERNAL_MAX_ID)
        .expect("valid ic-memory internal range");
    if range.contains(id) {
        return Ok(());
    }

    Err(MemoryRegistryError::RangeAuthorityViolation {
        stable_key: stable_key.to_string(),
        id,
        reason: "ic_memory.* keys must use ids 0-9",
    })
}

fn validate_canic_claim(
    id: u8,
    crate_name: &str,
    stable_key: &str,
) -> Result<(), MemoryRegistryError> {
    if !crate_name.starts_with("canic") {
        return Err(MemoryRegistryError::RangeAuthorityViolation {
            stable_key: stable_key.to_string(),
            id,
            reason: "canic.* keys may only be declared by Canic framework crates",
        });
    }

    let range = MemoryManagerIdRange::new(CANIC_FRAMEWORK_MIN_ID, CANIC_FRAMEWORK_MAX_ID)
        .expect("valid Canic framework range");
    if range.contains(id) {
        return Ok(());
    }

    Err(MemoryRegistryError::RangeAuthorityViolation {
        stable_key: stable_key.to_string(),
        id,
        reason: "canic.* keys must use ids 10-99",
    })
}

fn validate_application_claim(id: u8, stable_key: &str) -> Result<(), MemoryRegistryError> {
    let range =
        MemoryManagerIdRange::new(APPLICATION_MIN_ID, APPLICATION_MAX_ID).expect("valid app range");
    if range.contains(id) {
        return Ok(());
    }

    Err(MemoryRegistryError::RangeAuthorityViolation {
        stable_key: stable_key.to_string(),
        id,
        reason: "application keys must use ids 100-254",
    })
}

fn memory_slot_error_to_registry_error(err: MemoryManagerSlotError) -> MemoryRegistryError {
    match err {
        MemoryManagerSlotError::InvalidMemoryManagerId { id } => {
            MemoryRegistryError::ReservedInternalId { id }
        }
        MemoryManagerSlotError::UnsupportedSlot
        | MemoryManagerSlotError::UnsupportedSubstrate { .. }
        | MemoryManagerSlotError::UnsupportedDescriptorVersion { .. } => {
            MemoryRegistryError::LedgerCorrupt {
                reason: "unsupported MemoryManager allocation slot descriptor",
            }
        }
    }
}
