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

///
/// RegisteredMemory
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisteredMemory {
    pub id: u8,
    pub owner: String,
    pub range: MemoryRange,
    pub label: String,
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

    /// Bootstrap eager TLS, eager-init hooks, and flush deferred registry state
    /// without reserving a new owner range.
    pub fn bootstrap_registry_without_range()
    -> Result<MemoryRegistryInitSummary, MemoryRegistryError> {
        MemoryRuntimeApi::bootstrap_registry_without_range()
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

    /// List every registered memory slot with owner/range/label context.
    #[must_use]
    pub fn registered_memories() -> Vec<RegisteredMemory> {
        MemoryRegistry::export_ids_by_range()
            .into_iter()
            .flat_map(|snapshot| {
                snapshot
                    .entries
                    .into_iter()
                    .map(move |(id, entry)| RegisteredMemory {
                        id,
                        owner: snapshot.owner.clone(),
                        range: snapshot.range,
                        label: entry.label,
                    })
            })
            .collect()
    }

    /// List all registered memory slots for one owner.
    #[must_use]
    pub fn registered_memories_for_owner(owner: &str) -> Vec<RegisteredMemory> {
        Self::registered_memories()
            .into_iter()
            .filter(|entry| entry.owner == owner)
            .collect()
    }

    /// Find one registered memory slot by owner and label.
    #[must_use]
    pub fn find_registered_memory(owner: &str, label: &str) -> Option<RegisteredMemory> {
        Self::registered_memories()
            .into_iter()
            .find(|entry| entry.owner == owner && entry.label == label)
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
    use crate::registry::{
        MemoryRegistryError, defer_register, defer_reserve_range, reset_for_tests,
    };

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
    fn bootstrap_registry_without_range_flushes_deferred_state() {
        reset_for_tests();
        defer_reserve_range("crate_a", 1, 3).expect("defer range");
        defer_register(2, "crate_a", "slot").expect("defer register");

        let summary =
            MemoryApi::bootstrap_registry_without_range().expect("bootstrap without range");

        assert_eq!(
            summary.ranges,
            vec![("crate_a".to_string(), MemoryRange { start: 1, end: 3 })]
        );
        assert_eq!(summary.entries.len(), 1);
        assert_eq!(summary.entries[0].0, 2);
        assert_eq!(summary.entries[0].1.crate_name, "crate_a");
        assert_eq!(summary.entries[0].1.label, "slot");
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

    #[test]
    fn registered_memories_lists_registered_slots_with_owner_context() {
        reset_for_tests();
        let _ = MemoryApi::bootstrap_registry("crate_a", 1, 3).expect("bootstrap registry");
        let _ = MemoryApi::bootstrap_registry("crate_b", 10, 12).expect("bootstrap registry");
        let _ = MemoryApi::register_memory(2, "crate_a", "slot_a").expect("register memory");
        let _ = MemoryApi::register_memory(11, "crate_b", "slot_b").expect("register memory");

        let registrations = MemoryApi::registered_memories();
        assert_eq!(registrations.len(), 2);
        assert!(registrations.contains(&RegisteredMemory {
            id: 2,
            owner: "crate_a".to_string(),
            range: MemoryRange { start: 1, end: 3 },
            label: "slot_a".to_string(),
        }));
        assert!(registrations.contains(&RegisteredMemory {
            id: 11,
            owner: "crate_b".to_string(),
            range: MemoryRange { start: 10, end: 12 },
            label: "slot_b".to_string(),
        }));
    }

    #[test]
    fn registered_memories_for_owner_filters_to_owner() {
        reset_for_tests();
        let _ = MemoryApi::bootstrap_registry("crate_a", 1, 3).expect("bootstrap registry");
        let _ = MemoryApi::bootstrap_registry("crate_b", 10, 12).expect("bootstrap registry");
        let _ = MemoryApi::register_memory(2, "crate_a", "slot_a").expect("register memory");
        let _ = MemoryApi::register_memory(11, "crate_b", "slot_b").expect("register memory");

        let registrations = MemoryApi::registered_memories_for_owner("crate_a");
        assert_eq!(
            registrations,
            vec![RegisteredMemory {
                id: 2,
                owner: "crate_a".to_string(),
                range: MemoryRange { start: 1, end: 3 },
                label: "slot_a".to_string(),
            }]
        );
    }

    #[test]
    fn find_registered_memory_returns_match_for_owner_and_label() {
        reset_for_tests();
        let _ = MemoryApi::bootstrap_registry("crate_a", 1, 3).expect("bootstrap registry");
        let _ = MemoryApi::register_memory(2, "crate_a", "slot_a").expect("register memory");

        let registration =
            MemoryApi::find_registered_memory("crate_a", "slot_a").expect("slot should exist");
        assert_eq!(
            registration,
            RegisteredMemory {
                id: 2,
                owner: "crate_a".to_string(),
                range: MemoryRange { start: 1, end: 3 },
                label: "slot_a".to_string(),
            }
        );
    }

    #[test]
    fn find_registered_memory_returns_none_when_missing() {
        reset_for_tests();
        let _ = MemoryApi::bootstrap_registry("crate_a", 1, 3).expect("bootstrap registry");
        assert_eq!(MemoryApi::find_registered_memory("crate_a", "slot_a"), None);
    }
}
