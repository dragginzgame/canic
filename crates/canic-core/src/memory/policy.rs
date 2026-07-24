//! Module: memory::policy
//!
//! Responsibility: enforce Canic memory-manager namespace and ID-range ownership.
//! Does not own: memory-manager storage, stable schemas, or diagnostics rendering.
//! Boundary: memory bootstrap passes this policy into `ic-memory` validation.

use crate::{
    memory::{
        CANIC_CONTROL_PLANE_MEMORY_AUTHORITY, CANIC_CORE_MEMORY_AUTHORITY,
        registry::MemoryRegistryError,
    },
    role_contract::allocation::{
        CANIC_CONTROL_PLANE_MAX_ID, CANIC_CONTROL_PLANE_MIN_ID, CANIC_CORE_MAX_ID,
        CANIC_CORE_MIN_ID,
    },
};
use ic_memory::{
    AllocationPolicy, AllocationSlotDescriptor, MemoryManagerAuthorityRecord, MemoryManagerIdRange,
    MemoryManagerRangeMode, MemoryManagerSlotError, StableKey,
};

pub const CANIC_CORE_AUTHORITY_PURPOSE: &str = "Canic core allocation authority";
pub const CANIC_CONTROL_PLANE_AUTHORITY_PURPOSE: &str = "Canic control-plane allocation authority";

///
/// CanicMemoryManagerPolicy
///
/// Canic policy adapter for the `ic-memory` MemoryManager substrate
/// allocation slots.
/// Owned by memory policy and supplied to memory-manager bootstrap.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CanicMemoryManagerPolicy;

impl CanicMemoryManagerPolicy {
    #[must_use]
    pub(super) const fn new() -> Self {
        Self
    }
}

impl AllocationPolicy for CanicMemoryManagerPolicy {
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
        validate_key_id_claim(id, key.as_str())
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
        validate_key_id_claim(id, key.as_str())
    }
}

/// Return the canonical memory-manager authority records for diagnostics.
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
            canic_core_range(),
            CANIC_CORE_MEMORY_AUTHORITY,
            MemoryManagerRangeMode::Reserved,
            Some(CANIC_CORE_AUTHORITY_PURPOSE.to_string()),
        )
        .expect("valid Canic core authority record"),
        MemoryManagerAuthorityRecord::new(
            canic_control_plane_range(),
            CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
            MemoryManagerRangeMode::Reserved,
            Some(CANIC_CONTROL_PLANE_AUTHORITY_PURPOSE.to_string()),
        )
        .expect("valid Canic control-plane authority record"),
    ]
}

fn validate_key_id_claim(id: u8, stable_key: &str) -> Result<(), MemoryRegistryError> {
    if ic_memory::is_ic_memory_stable_key(stable_key) {
        return Ok(());
    }

    if stable_key.starts_with("canic.core.") {
        return require_range(
            id,
            stable_key,
            canic_core_range(),
            "canic.core.* keys must use Canic core ids 11-79",
        );
    }

    if stable_key.starts_with("canic.control_plane.") {
        return require_range(
            id,
            stable_key,
            canic_control_plane_range(),
            "canic.control_plane.* keys must use Canic control-plane ids 80-99",
        );
    }

    if stable_key.starts_with("canic.") {
        return Err(MemoryRegistryError::RangeAuthorityViolation {
            stable_key: stable_key.to_string(),
            id,
            reason: "unrecognized canic.* stable key namespace",
        });
    }

    validate_application_claim(id, stable_key)
}

fn validate_application_claim(id: u8, stable_key: &str) -> Result<(), MemoryRegistryError> {
    if ic_memory::memory_manager_governance_range().contains(id)
        || canic_core_range().contains(id)
        || canic_control_plane_range().contains(id)
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

fn canic_core_range() -> MemoryManagerIdRange {
    MemoryManagerIdRange::new(CANIC_CORE_MIN_ID, CANIC_CORE_MAX_ID).expect("valid Canic core range")
}

fn canic_control_plane_range() -> MemoryManagerIdRange {
    MemoryManagerIdRange::new(CANIC_CONTROL_PLANE_MIN_ID, CANIC_CONTROL_PLANE_MAX_ID)
        .expect("valid Canic control-plane range")
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
        _ => MemoryRegistryError::InvalidDeclaration {
            stable_key: "<slot>".to_string(),
            reason: "unsupported MemoryManager slot error",
        },
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> CanicMemoryManagerPolicy {
        CanicMemoryManagerPolicy::new()
    }

    fn key(value: &str) -> StableKey {
        StableKey::parse(value).expect("stable key")
    }

    fn slot(id: u8) -> AllocationSlotDescriptor {
        AllocationSlotDescriptor::memory_manager(id).expect("usable MemoryManager id")
    }

    #[test]
    fn rejects_memory_manager_sentinel_id_through_ic_memory() {
        let err = AllocationSlotDescriptor::memory_manager(ic_memory::MEMORY_MANAGER_INVALID_ID)
            .expect_err("ID 255 is the unallocated-bucket sentinel");
        std::assert_matches!(
            err,
            MemoryManagerSlotError::InvalidMemoryManagerId { id }
                if id == ic_memory::MEMORY_MANAGER_INVALID_ID
        );
    }

    fn validate(stable_key: &str, id: u8) -> Result<(), MemoryRegistryError> {
        policy().validate_slot(&key(stable_key), &slot(id))
    }

    fn validate_reserved(stable_key: &str, id: u8) -> Result<(), MemoryRegistryError> {
        policy().validate_reserved_slot(&key(stable_key), &slot(id))
    }

    #[test]
    fn accepts_canic_framework_namespaces_in_owned_ranges() {
        validate("canic.core.canister_children.v1", CANIC_CORE_MIN_ID).expect("first core slot");
        validate("canic.core.future.v1", CANIC_CORE_MAX_ID).expect("last core slot");
        validate(
            "canic.control_plane.template_manifest.v1",
            CANIC_CONTROL_PLANE_MIN_ID,
        )
        .expect("first control-plane slot");
        validate("canic.control_plane.future.v1", CANIC_CONTROL_PLANE_MAX_ID)
            .expect("last control-plane slot");
    }

    #[test]
    fn rejects_canic_framework_namespaces_outside_owned_ranges() {
        let err = validate("canic.core.fleet_state.v1", CANIC_CONTROL_PLANE_MIN_ID)
            .expect_err("core key cannot claim control-plane range");
        std::assert_matches!(err, MemoryRegistryError::RangeAuthorityViolation { .. });

        let err = validate(
            "canic.control_plane.template_manifest.v1",
            CANIC_CORE_MIN_ID,
        )
        .expect_err("control-plane key cannot claim core range");
        std::assert_matches!(err, MemoryRegistryError::RangeAuthorityViolation { .. });

        let err = validate("canic.unknown.state.v1", CANIC_CONTROL_PLANE_MAX_ID + 1)
            .expect_err("unknown canic namespace is reserved");
        std::assert_matches!(err, MemoryRegistryError::RangeAuthorityViolation { .. });
    }

    #[test]
    fn accepts_application_keys_only_outside_reserved_ranges() {
        validate("app.users.v1", CANIC_CONTROL_PLANE_MAX_ID + 1).expect("application slot");
        validate("app.archive.v1", ic_memory::MEMORY_MANAGER_MAX_ID).expect("last app slot");

        let err = validate("app.users.v1", CANIC_CORE_MIN_ID)
            .expect_err("application key cannot claim Canic core range");
        std::assert_matches!(err, MemoryRegistryError::RangeAuthorityViolation { .. });

        let err = validate("app.users.v1", CANIC_CONTROL_PLANE_MAX_ID)
            .expect_err("application key cannot claim Canic control-plane reserve");
        std::assert_matches!(err, MemoryRegistryError::RangeAuthorityViolation { .. });

        let err = validate("app.users.v1", ic_memory::MEMORY_MANAGER_LEDGER_ID)
            .expect_err("application key cannot claim ic-memory governance range");
        std::assert_matches!(err, MemoryRegistryError::RangeAuthorityViolation { .. });
    }

    #[test]
    fn rejects_application_reservations() {
        let err = validate_reserved("app.users.v1", CANIC_CONTROL_PLANE_MAX_ID + 1)
            .expect_err("Canic does not pre-reserve application keys");
        std::assert_matches!(err, MemoryRegistryError::RangeAuthorityViolation { .. });
    }
}
