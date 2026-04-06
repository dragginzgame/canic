use crate::{
    cdk::structures::{
        DefaultMemoryImpl,
        memory::{MemoryId, VirtualMemory},
    },
    manager::MEMORY_MANAGER,
    registry::{MemoryRange, MemoryRegistry, MemoryRegistryError},
    runtime::{MemoryRuntimeApi, registry::MemoryRegistryInitSummary},
};

///
/// MemoryApi
///

pub struct MemoryApi;

///
/// MemoryInspection
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryInspection {
    pub id: u8,
    pub owner: String,
    pub range: MemoryRange,
    pub label: Option<String>,
}

impl MemoryApi {
    /// Bootstrap eager TLS, eager-init hooks, and the caller's initial reserved range.
    pub fn bootstrap_registry(
        crate_name: &'static str,
        start: u8,
        end: u8,
    ) -> Result<MemoryRegistryInitSummary, MemoryRegistryError> {
        MemoryRuntimeApi::bootstrap_registry(crate_name, start, end)
    }

    /// Register one stable-memory ID and return its opened virtual memory handle.
    ///
    /// Call `bootstrap_registry(...)` first so the caller's owned range is reserved.
    pub fn register_memory(
        id: u8,
        crate_name: &str,
        label: &str,
    ) -> Result<VirtualMemory<DefaultMemoryImpl>, MemoryRegistryError> {
        if let Some(entry) = MemoryRegistry::get(id)
            && entry.crate_name == crate_name
            && entry.label == label
        {
            return Ok(open_memory(id));
        }

        MemoryRegistry::register(id, crate_name, label)?;
        Ok(open_memory(id))
    }

    /// Inspect who currently owns one memory id and whether it is registered.
    #[must_use]
    pub fn inspect_memory(id: u8) -> Option<MemoryInspection> {
        let range = MemoryRegistry::export_range_entries()
            .into_iter()
            .find(|entry| entry.range.contains(id))?;
        let label = MemoryRegistry::get(id).map(|entry| entry.label);

        Some(MemoryInspection {
            id,
            owner: range.owner,
            range: range.range,
            label,
        })
    }
}

// Open a registered virtual memory slot through the shared manager.
fn open_memory(id: u8) -> VirtualMemory<DefaultMemoryImpl> {
    MEMORY_MANAGER.with_borrow_mut(|mgr| mgr.get(MemoryId::new(id)))
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{MemoryRegistryError, reset_for_tests};

    #[test]
    fn register_memory_returns_opened_memory_for_reserved_slot() {
        reset_for_tests();
        let _ = MemoryApi::bootstrap_registry("crate_a", 1, 3).expect("bootstrap registry");

        let _memory = MemoryApi::register_memory(2, "crate_a", "slot").expect("register memory");
    }

    #[test]
    fn register_memory_is_idempotent_for_same_entry() {
        reset_for_tests();
        let _ = MemoryApi::bootstrap_registry("crate_a", 1, 3).expect("bootstrap registry");
        let _ = MemoryApi::register_memory(2, "crate_a", "slot").expect("first register succeeds");

        let _ = MemoryApi::register_memory(2, "crate_a", "slot").expect("second register succeeds");
    }

    #[test]
    fn register_memory_rejects_unreserved_id() {
        reset_for_tests();

        let Err(err) = MemoryApi::register_memory(9, "crate_a", "slot") else {
            panic!("unreserved slot must fail")
        };
        assert!(matches!(err, MemoryRegistryError::NoReservedRange { .. }));
    }

    #[test]
    fn register_memory_preserves_duplicate_id_error_for_conflicts() {
        reset_for_tests();
        let _ = MemoryApi::bootstrap_registry("crate_a", 1, 3).expect("bootstrap registry");
        MemoryApi::register_memory(2, "crate_a", "slot").expect("first register succeeds");

        let Err(err) = MemoryApi::register_memory(2, "crate_a", "other") else {
            panic!("conflicting duplicate register must fail")
        };
        assert!(matches!(err, MemoryRegistryError::DuplicateId(2)));
    }

    #[test]
    fn inspect_memory_returns_reserved_owner_without_label() {
        reset_for_tests();
        let _ = MemoryApi::bootstrap_registry("crate_a", 1, 3).expect("bootstrap registry");

        let inspection = MemoryApi::inspect_memory(2).expect("reserved slot should inspect");
        assert_eq!(inspection.owner, "crate_a");
        assert_eq!(inspection.range, MemoryRange { start: 1, end: 3 });
        assert_eq!(inspection.label, None);
    }

    #[test]
    fn inspect_memory_returns_registered_label() {
        reset_for_tests();
        let _ = MemoryApi::bootstrap_registry("crate_a", 1, 3).expect("bootstrap registry");
        let _ = MemoryApi::register_memory(2, "crate_a", "slot").expect("register memory");

        let inspection = MemoryApi::inspect_memory(2).expect("registered slot should inspect");
        assert_eq!(inspection.owner, "crate_a");
        assert_eq!(inspection.range, MemoryRange { start: 1, end: 3 });
        assert_eq!(inspection.label.as_deref(), Some("slot"));
    }

    #[test]
    fn inspect_memory_returns_none_for_unowned_id() {
        reset_for_tests();
        assert_eq!(MemoryApi::inspect_memory(9), None);
    }
}
