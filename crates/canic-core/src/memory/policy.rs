use super::registry::{MemoryRange, MemoryRegistryError};
use ic_memory::{MemoryManagerIdRange, StableKey};

pub const IC_MEMORY_INTERNAL_MIN_ID: u8 = 0;
pub const IC_MEMORY_INTERNAL_MAX_ID: u8 = 9;
pub const CANIC_FRAMEWORK_MIN_ID: u8 = 10;
pub const CANIC_FRAMEWORK_MAX_ID: u8 = 99;
pub const APPLICATION_MIN_ID: u8 = 100;
pub const APPLICATION_MAX_ID: u8 = ic_memory::MEMORY_MANAGER_MAX_ID;

pub const IC_MEMORY_AUTHORITY_OWNER: &str = "ic_memory.internal";
pub const IC_MEMORY_AUTHORITY_PURPOSE: &str = "ic-memory allocation-governance authority";
pub const CANIC_FRAMEWORK_AUTHORITY_OWNER: &str = "canic.framework";
pub const CANIC_FRAMEWORK_AUTHORITY_PURPOSE: &str = "Canic framework allocation authority";
pub const APPLICATION_AUTHORITY_OWNER: &str = "applications";
pub const APPLICATION_AUTHORITY_PURPOSE: &str = "downstream application allocation authority";

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

    if key.as_str().starts_with("ic_memory.") {
        return validate_ic_memory_claim(id, stable_key);
    }

    if key.as_str().starts_with("canic.") {
        return validate_canic_claim(id, crate_name, stable_key);
    }

    validate_application_claim(id, stable_key)
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

fn validate_ic_memory_claim(id: u8, stable_key: &str) -> Result<(), MemoryRegistryError> {
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
