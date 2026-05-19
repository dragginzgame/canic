use super::registry::MemoryRegistryError;
use ic_memory::{
    AllocationPolicy, AllocationSlotDescriptor, MemoryManagerAuthorityRecord, MemoryManagerIdRange,
    MemoryManagerRangeMode, MemoryManagerSlotError, StableKey,
};

pub const CANIC_FRAMEWORK_MIN_ID: u8 = 10;
pub const CANIC_FRAMEWORK_MAX_ID: u8 = 99;

pub const CANIC_FRAMEWORK_AUTHORITY_OWNER: &str = "canic.framework";
pub const CANIC_FRAMEWORK_AUTHORITY_PURPOSE: &str = "Canic framework allocation authority";

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
        if !ic_memory::is_ic_memory_stable_key(key.as_str()) && !key.as_str().starts_with("canic.")
        {
            return Err(MemoryRegistryError::RangeAuthorityViolation {
                stable_key: key.as_str().to_string(),
                id,
                reason: "application stable keys may not be pre-reserved by Canic",
            });
        }
        validate_key_id_claim(id, self.declaring_crate, key.as_str())
    }
}

pub fn validate_stable_key_authority(
    id: u8,
    crate_name: &str,
    stable_key: &str,
) -> Result<(), MemoryRegistryError> {
    let key =
        StableKey::parse(stable_key).map_err(|err| MemoryRegistryError::InvalidDeclaration {
            stable_key: err.stable_key,
            reason: err.reason,
        })?;

    let slot = AllocationSlotDescriptor::memory_manager_checked(id)
        .map_err(memory_slot_error_to_registry_error)?;
    let policy = CanicMemoryManagerPolicy::for_declaring_crate(crate_name);

    AllocationPolicy::validate_slot(&policy, &key, &slot)
}

#[must_use]
pub fn canonical_authority_records() -> Vec<MemoryManagerAuthorityRecord> {
    vec![
        MemoryManagerAuthorityRecord::new(
            ic_memory::memory_manager_governance_range(),
            ic_memory::IC_MEMORY_AUTHORITY_OWNER,
            MemoryManagerRangeMode::Reserved,
            Some(ic_memory::IC_MEMORY_AUTHORITY_PURPOSE.to_string()),
        )
        .expect("valid ic-memory authority record"),
        MemoryManagerAuthorityRecord::new(
            canic_framework_range(),
            CANIC_FRAMEWORK_AUTHORITY_OWNER,
            MemoryManagerRangeMode::Reserved,
            Some(CANIC_FRAMEWORK_AUTHORITY_PURPOSE.to_string()),
        )
        .expect("valid Canic authority record"),
    ]
}

fn validate_key_id_claim(
    id: u8,
    crate_name: &str,
    stable_key: &str,
) -> Result<(), MemoryRegistryError> {
    if ic_memory::is_ic_memory_stable_key(stable_key) {
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
    if crate_name != ic_memory::IC_MEMORY_AUTHORITY_OWNER {
        return Err(MemoryRegistryError::RangeAuthorityViolation {
            stable_key: stable_key.to_string(),
            id,
            reason: "ic_memory.* keys may only be declared by ic-memory",
        });
    }

    require_range(
        id,
        stable_key,
        ic_memory::memory_manager_governance_range(),
        "ic_memory.* keys must use the ic-memory governance range",
    )
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

    require_range(
        id,
        stable_key,
        canic_framework_range(),
        "canic.* keys must use ids 10-99",
    )
}

fn validate_application_claim(id: u8, stable_key: &str) -> Result<(), MemoryRegistryError> {
    if ic_memory::memory_manager_governance_range().contains(id)
        || canic_framework_range().contains(id)
    {
        return Err(MemoryRegistryError::RangeAuthorityViolation {
            stable_key: stable_key.to_string(),
            id,
            reason: "application keys may not use reserved MemoryManager IDs",
        });
    }
    Ok(())
}

fn require_range(
    id: u8,
    stable_key: &str,
    range: MemoryManagerIdRange,
    reason: &'static str,
) -> Result<(), MemoryRegistryError> {
    if range.contains(id) {
        Ok(())
    } else {
        Err(MemoryRegistryError::RangeAuthorityViolation {
            stable_key: stable_key.to_string(),
            id,
            reason,
        })
    }
}

fn canic_framework_range() -> MemoryManagerIdRange {
    MemoryManagerIdRange::new(CANIC_FRAMEWORK_MIN_ID, CANIC_FRAMEWORK_MAX_ID)
        .expect("valid Canic framework range")
}

fn memory_slot_error_to_registry_error(err: MemoryManagerSlotError) -> MemoryRegistryError {
    match err {
        MemoryManagerSlotError::InvalidMemoryManagerId { id } => {
            MemoryRegistryError::InvalidDeclaration {
                stable_key: "<slot>".to_string(),
                reason: if id == ic_memory::MEMORY_MANAGER_INVALID_ID {
                    "MemoryManager ID 255 is not usable"
                } else {
                    "MemoryManager ID is not usable"
                },
            }
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
