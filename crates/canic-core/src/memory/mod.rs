//! Canic-managed stable-memory runtime boundary.
//!
//! This module is the Canic-owned adapter around current stable-memory
//! bootstrap mechanics while durable allocation-governance primitives move
//! into `ic-memory`.

use crate::cdk::structures::{
    DefaultMemoryImpl,
    memory::{MemoryId, VirtualMemory},
};
use ic_memory::{
    AllocationSession, AllocationSessionError, AllocationSlotDescriptor, MemoryManagerSlotError,
    StableKey, StorageSubstrate,
};

pub mod api;
mod ledger;
mod manager;
mod policy;
pub mod registry;
pub mod runtime;

pub use crate::{eager_init, eager_static, ic_memory_key, ic_memory_range};

///
/// open_validated_memory
///

#[doc(hidden)]
#[must_use]
pub fn open_validated_memory(
    stable_key: &str,
    label: &str,
    id: u8,
) -> VirtualMemory<DefaultMemoryImpl> {
    runtime::assert_memory_bootstrap_ready(label, id);
    try_open_validated_memory(stable_key, id).unwrap_or_else(|err| {
        panic!(
            "stable memory slot '{label}' (id {id}, key '{stable_key}') failed validated open: {err}"
        );
    })
}

fn try_open_validated_memory(
    stable_key: &str,
    id: u8,
) -> Result<VirtualMemory<DefaultMemoryImpl>, registry::MemoryRegistryError> {
    let key = StableKey::parse(stable_key).map_err(|err| {
        registry::MemoryRegistryError::InvalidStableKey {
            stable_key: err.stable_key,
            reason: err.reason,
        }
    })?;
    let validated = runtime::registry::MemoryRegistryRuntime::validated_allocations()?;
    let slot = validated.slot_for(&key).ok_or(
        registry::MemoryRegistryError::RegistrationAfterBootstrap {
            ranges: 0,
            registrations: 1,
        },
    )?;
    let slot_id = memory_manager_id_from_slot(slot)?;
    if slot_id != id {
        return Err(registry::MemoryRegistryError::RegistrationAfterBootstrap {
            ranges: 0,
            registrations: 1,
        });
    }

    let session = AllocationSession::new(MemoryManagerSubstrate, validated);
    session
        .open(&key)
        .map_err(memory_registry_error_from_session_error)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MemoryManagerSubstrate;

impl StorageSubstrate for MemoryManagerSubstrate {
    type Slot = u8;
    type LedgerMemory = VirtualMemory<DefaultMemoryImpl>;
    type MemoryHandle = VirtualMemory<DefaultMemoryImpl>;
    type Error = registry::MemoryRegistryError;

    fn open_ledger(&self) -> Result<Self::LedgerMemory, Self::Error> {
        Ok(open_memory(ledger::MEMORY_LAYOUT_LEDGER_ID))
    }

    fn open_slot(
        &self,
        slot: &AllocationSlotDescriptor,
    ) -> Result<Self::MemoryHandle, Self::Error> {
        let id = memory_manager_id_from_slot(slot)?;
        Ok(open_memory(id))
    }

    fn describe_slot(&self, slot: &Self::Slot) -> AllocationSlotDescriptor {
        AllocationSlotDescriptor::memory_manager(*slot)
    }
}

fn open_memory(id: u8) -> VirtualMemory<DefaultMemoryImpl> {
    manager::MEMORY_MANAGER.with_borrow_mut(|mgr| mgr.get(MemoryId::new(id)))
}

fn memory_registry_error_from_session_error(
    err: AllocationSessionError<registry::MemoryRegistryError>,
) -> registry::MemoryRegistryError {
    match err {
        AllocationSessionError::UnknownStableKey(_) => {
            registry::MemoryRegistryError::RegistrationAfterBootstrap {
                ranges: 0,
                registrations: 1,
            }
        }
        AllocationSessionError::Substrate(err) => err,
    }
}

fn memory_manager_id_from_slot(
    slot: &AllocationSlotDescriptor,
) -> Result<u8, registry::MemoryRegistryError> {
    slot.memory_manager_id()
        .map_err(memory_registry_error_from_slot_error)
}

fn memory_registry_error_from_slot_error(
    err: MemoryManagerSlotError,
) -> registry::MemoryRegistryError {
    match err {
        MemoryManagerSlotError::InvalidMemoryManagerId { id } => {
            registry::MemoryRegistryError::ReservedInternalId { id }
        }
        MemoryManagerSlotError::UnsupportedSlot
        | MemoryManagerSlotError::UnsupportedSubstrate { .. }
        | MemoryManagerSlotError::UnsupportedDescriptorVersion { .. } => {
            registry::MemoryRegistryError::LedgerCorrupt {
                reason: "unsupported MemoryManager allocation slot descriptor",
            }
        }
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::registry::{defer_register_with_key, defer_reserve_range, reset_for_tests};

    #[test]
    fn validated_open_requires_declared_stable_key() {
        reset_for_tests();
        defer_reserve_range("crate_a", 100, 102).expect("defer range");
        defer_register_with_key(101, "crate_a", "slot", "app.crate_a.slot.v1")
            .expect("defer register");
        runtime::registry::MemoryRegistryRuntime::init(None).expect("bootstrap registry");

        let _memory =
            try_open_validated_memory("app.crate_a.slot.v1", 101).expect("open by stable key");
    }

    #[test]
    fn validated_open_rejects_key_id_mismatch() {
        reset_for_tests();
        defer_reserve_range("crate_a", 100, 102).expect("defer range");
        defer_register_with_key(101, "crate_a", "slot", "app.crate_a.slot.v1")
            .expect("defer register");
        runtime::registry::MemoryRegistryRuntime::init(None).expect("bootstrap registry");

        let Err(err) = try_open_validated_memory("app.crate_a.slot.v1", 102) else {
            panic!("wrong id must not open declared key");
        };
        assert!(matches!(
            err,
            registry::MemoryRegistryError::RegistrationAfterBootstrap { .. }
        ));
    }
}
