use crate::ledger;
use crate::{
    cdk::structures::{
        DefaultMemoryImpl,
        memory::{MemoryId, VirtualMemory},
    },
    manager::MEMORY_MANAGER,
    registry::{
        MemoryRange, MemoryRangeAuthority, MemoryRegistry, MemoryRegistryError, defer_register,
        defer_register_with_key_metadata,
    },
    runtime::{MemoryRuntimeApi, registry::MemoryRegistryRuntime},
};

///
/// MemoryApi
///
/// Supported facade for memory bootstrap, dynamic slot registration, and
/// registry inspection.

pub struct MemoryApi;

///
/// MemoryInspection
///
/// Read-only description of the owner range for one memory ID.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryInspection {
    /// Stable-memory ID being inspected.
    pub id: u8,
    /// Crate name that reserved the range containing `id`.
    pub owner: String,
    /// Reserved range containing `id`.
    pub range: MemoryRange,
    /// Registered slot label for `id`, when the ID has already been registered.
    pub label: Option<String>,
    /// ABI-stable key for `id`, when the ID has already been registered.
    pub stable_key: Option<String>,
    /// Optional in-place schema version metadata for diagnostics.
    pub schema_version: Option<u32>,
    /// Optional opaque schema fingerprint metadata for diagnostics.
    pub schema_fingerprint: Option<String>,
}

///
/// RegisteredMemory
///
/// Read-only description of one registered stable-memory slot.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisteredMemory {
    /// Registered stable-memory ID.
    pub id: u8,
    /// Crate name that owns the slot's reserved range.
    pub owner: String,
    /// Reserved range containing `id`.
    pub range: MemoryRange,
    /// Human-readable slot label supplied by the registering crate.
    pub label: String,
    /// ABI-stable key that owns this memory ID permanently.
    pub stable_key: String,
    /// Optional in-place schema version metadata for diagnostics.
    pub schema_version: Option<u32>,
    /// Optional opaque schema fingerprint metadata for diagnostics.
    pub schema_fingerprint: Option<String>,
}

///
/// LedgerSnapshot
///
/// Read-only snapshot of the persisted ABI ledger.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LedgerSnapshot {
    /// Ledger magic value from the physical header.
    pub magic: u64,
    /// Ledger physical format identifier from the header.
    pub format_id: u32,
    /// Ledger schema version from the header.
    pub schema_version: u32,
    /// Compiled layout epoch validated against the persisted header.
    pub layout_epoch: u32,
    /// Encoded ledger header length.
    pub header_len: u32,
    /// Header checksum covering the persisted header fields.
    pub header_checksum: u64,
    /// Authoritative committed generation selected by recovery validation.
    pub current_generation: u64,
    /// Canonical allocation authority ranges recorded by the persisted ABI ledger.
    pub authorities: Vec<MemoryRangeAuthority>,
    /// Historical owner ranges recorded by the persisted ABI ledger.
    pub ranges: Vec<(String, MemoryRange)>,
    /// Historical memory ID records recorded by the persisted ABI ledger.
    pub entries: Vec<(u8, crate::registry::MemoryRegistryEntry)>,
}

impl MemoryApi {
    /// Bootstrap eager TLS, eager-init hooks, and the caller's initial reserved range.
    pub fn bootstrap_owner_range(
        crate_name: &'static str,
        start: u8,
        end: u8,
    ) -> Result<(), MemoryRegistryError> {
        let _ = MemoryRuntimeApi::bootstrap_registry(crate_name, start, end)?;
        Ok(())
    }

    /// Bootstrap eager TLS, eager-init hooks, and flush deferred registry state
    /// without reserving a new owner range.
    pub fn bootstrap_pending() -> Result<(), MemoryRegistryError> {
        let _ = MemoryRuntimeApi::bootstrap_registry_without_range()?;
        Ok(())
    }

    /// Declare one legacy-key stable-memory ID for bootstrap validation.
    ///
    /// This queues metadata only. It does not open the underlying virtual memory.
    pub fn declare(id: u8, crate_name: &str, label: &str) -> Result<(), MemoryRegistryError> {
        if MemoryRegistryRuntime::is_initialized() {
            return Err(MemoryRegistryError::RegistrationAfterBootstrap {
                ranges: 0,
                registrations: 1,
            });
        }

        defer_register(id, crate_name, label)
    }

    /// Declare one explicit-key stable-memory ID for bootstrap validation.
    ///
    /// This queues metadata only. It does not open the underlying virtual memory.
    pub fn declare_with_key(
        id: u8,
        crate_name: &str,
        label: &str,
        stable_key: &str,
    ) -> Result<(), MemoryRegistryError> {
        Self::declare_with_key_metadata(id, crate_name, label, stable_key, None, None)
    }

    /// Declare one explicit-key stable-memory ID with optional schema metadata.
    ///
    /// Schema metadata is informational in 0.38 and does not affect allocation
    /// ownership. This queues metadata only. It does not open virtual memory.
    pub fn declare_with_key_metadata(
        id: u8,
        crate_name: &str,
        label: &str,
        stable_key: &str,
        schema_version: Option<u32>,
        schema_fingerprint: Option<&str>,
    ) -> Result<(), MemoryRegistryError> {
        if MemoryRegistryRuntime::is_initialized() {
            return Err(MemoryRegistryError::RegistrationAfterBootstrap {
                ranges: 0,
                registrations: 1,
            });
        }

        defer_register_with_key_metadata(
            id,
            crate_name,
            label,
            stable_key,
            schema_version,
            schema_fingerprint,
        )
    }

    /// Open one already-validated stable-memory ID and return its virtual memory handle.
    ///
    /// The ID must have been declared before bootstrap and accepted by the
    /// sealed runtime declaration snapshot. This is not a dynamic allocation API.
    pub fn register(
        id: u8,
        crate_name: &str,
        label: &str,
    ) -> Result<VirtualMemory<DefaultMemoryImpl>, MemoryRegistryError> {
        if !MemoryRegistryRuntime::is_initialized() {
            return Err(MemoryRegistryError::RegistryNotBootstrapped);
        }

        if let Some(entry) = MemoryRegistry::get(id)
            && entry.crate_name == crate_name
            && entry.label == label
        {
            return Ok(open_memory(id));
        }

        Err(MemoryRegistryError::RegistrationAfterBootstrap {
            ranges: 0,
            registrations: 1,
        })
    }

    /// Open one already-validated stable-memory ID using its explicit ABI-stable key.
    pub fn register_with_key(
        id: u8,
        _crate_name: &str,
        _label: &str,
        stable_key: &str,
    ) -> Result<VirtualMemory<DefaultMemoryImpl>, MemoryRegistryError> {
        if !MemoryRegistryRuntime::is_initialized() {
            return Err(MemoryRegistryError::RegistryNotBootstrapped);
        }

        if let Some(entry) = MemoryRegistry::get(id)
            && entry.stable_key == stable_key
        {
            return Ok(open_memory(id));
        }

        Err(MemoryRegistryError::RegistrationAfterBootstrap {
            ranges: 0,
            registrations: 1,
        })
    }

    /// Inspect who currently owns one memory id and whether it is registered.
    #[must_use]
    pub fn inspect(id: u8) -> Option<MemoryInspection> {
        let range = MemoryRegistry::export_range_entries()
            .into_iter()
            .find(|entry| entry.range.contains(id))?;
        let entry = MemoryRegistry::get(id);
        let label = entry.as_ref().map(|entry| entry.label.clone());
        let stable_key = entry.as_ref().map(|entry| entry.stable_key.clone());
        let schema_version = entry.as_ref().and_then(|entry| entry.schema_version);
        let schema_fingerprint = entry.and_then(|entry| entry.schema_fingerprint);

        Some(MemoryInspection {
            id,
            owner: range.owner,
            range: range.range,
            label,
            stable_key,
            schema_version,
            schema_fingerprint,
        })
    }

    /// List every registered memory slot with owner/range/label context.
    #[must_use]
    pub fn registered() -> Vec<RegisteredMemory> {
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
                        stable_key: entry.stable_key,
                        schema_version: entry.schema_version,
                        schema_fingerprint: entry.schema_fingerprint,
                    })
            })
            .collect()
    }

    /// List all registered memory slots for one owner.
    #[must_use]
    pub fn registered_for_owner(owner: &str) -> Vec<RegisteredMemory> {
        Self::registered()
            .into_iter()
            .filter(|entry| entry.owner == owner)
            .collect()
    }

    /// Find one registered memory slot by owner and label.
    #[must_use]
    pub fn find(owner: &str, label: &str) -> Option<RegisteredMemory> {
        Self::registered()
            .into_iter()
            .find(|entry| entry.owner == owner && entry.label == label)
    }

    /// Read the persisted ABI ledger without relying on current registry reconstruction.
    pub fn ledger_snapshot() -> Result<LedgerSnapshot, MemoryRegistryError> {
        #[cfg(target_arch = "wasm32")]
        {
            let snapshot = ledger::try_diagnostic_snapshot()?;
            Ok(LedgerSnapshot::from(snapshot))
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let snapshot = ledger::try_snapshot()?;
            Ok(LedgerSnapshot::from(snapshot))
        }
    }

    /// Project the persisted Canic ABI ledger into the generic `ic-memory`
    /// allocation ledger model.
    ///
    /// This is a read-only diagnostic bridge. It does not change the persisted
    /// Canic ledger format or open application/framework memory slots.
    pub fn allocation_ledger_snapshot() -> Result<ic_memory::AllocationLedger, MemoryRegistryError>
    {
        ledger::try_allocation_ledger()
    }

    /// Export the persisted Canic ABI ledger through the generic `ic-memory`
    /// diagnostic shape.
    ///
    /// Authorization for exposing this export is owned by the embedding runtime.
    pub fn allocation_diagnostic_export() -> Result<ic_memory::DiagnosticExport, MemoryRegistryError>
    {
        let ledger = Self::allocation_ledger_snapshot()?;
        Ok(ic_memory::DiagnosticExport::from_ledger(
            &ledger,
            ic_memory::AllocationSlotDescriptor::memory_manager(ledger::MEMORY_LAYOUT_LEDGER_ID),
        ))
    }
}

impl From<ledger::MemoryLayoutLedgerSnapshot> for LedgerSnapshot {
    fn from(snapshot: ledger::MemoryLayoutLedgerSnapshot) -> Self {
        Self {
            magic: snapshot.magic,
            format_id: snapshot.format_id,
            schema_version: snapshot.schema_version,
            layout_epoch: snapshot.layout_epoch,
            header_len: snapshot.header_len,
            header_checksum: snapshot.header_checksum,
            current_generation: snapshot.current_generation,
            authorities: snapshot.authorities,
            ranges: snapshot.ranges,
            entries: snapshot.entries,
        }
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
    fn register_memory_opens_validated_memory_for_reserved_slot() {
        reset_for_tests();
        defer_reserve_range("crate_a", 100, 102).expect("defer range");
        defer_register(101, "crate_a", "slot").expect("defer register");
        MemoryApi::bootstrap_pending().expect("bootstrap registry");

        let _memory = MemoryApi::register(101, "crate_a", "slot").expect("open memory");
    }

    #[test]
    fn register_memory_is_idempotent_for_same_entry() {
        reset_for_tests();
        defer_reserve_range("crate_a", 100, 102).expect("defer range");
        defer_register(101, "crate_a", "slot").expect("defer register");
        MemoryApi::bootstrap_pending().expect("bootstrap registry");
        let _ = MemoryApi::register(101, "crate_a", "slot").expect("first open succeeds");

        let _ = MemoryApi::register(101, "crate_a", "slot").expect("second open succeeds");
    }

    #[test]
    fn register_with_key_opens_validated_explicit_key() {
        reset_for_tests();
        defer_reserve_range("crate_a", 100, 102).expect("defer range");
        MemoryApi::declare_with_key(101, "crate_a", "slot", "app.crate_a.slot.v1")
            .expect("defer register");
        MemoryApi::bootstrap_pending().expect("bootstrap registry");

        let _memory = MemoryApi::register_with_key(101, "crate_a", "slot", "app.crate_a.slot.v1")
            .expect("open memory");
    }

    #[test]
    fn declare_with_key_metadata_records_schema_metadata() {
        reset_for_tests();
        defer_reserve_range("crate_a", 100, 102).expect("defer range");
        MemoryApi::declare_with_key_metadata(
            101,
            "crate_a",
            "slot",
            "app.crate_a.slot.v1",
            Some(3),
            Some("sha256:abc123"),
        )
        .expect("defer register");
        MemoryApi::bootstrap_pending().expect("bootstrap registry");

        let registered = MemoryApi::find("crate_a", "slot").expect("registered memory");
        assert_eq!(registered.schema_version, Some(3));
        assert_eq!(
            registered.schema_fingerprint.as_deref(),
            Some("sha256:abc123")
        );

        let snapshot = MemoryApi::ledger_snapshot().expect("ledger snapshot");
        assert_eq!(snapshot.format_id, 1);
        assert_eq!(snapshot.schema_version, 1);
        assert_eq!(snapshot.layout_epoch, 1);
        assert!(snapshot.current_generation > 0);
        let (_, entry) = snapshot
            .entries
            .into_iter()
            .find(|(id, _)| *id == 101)
            .expect("ledger entry");
        assert_eq!(entry.schema_version, Some(3));
        assert_eq!(entry.schema_fingerprint.as_deref(), Some("sha256:abc123"));
    }

    #[test]
    fn declare_memory_does_not_open_before_bootstrap() {
        reset_for_tests();

        MemoryApi::declare_with_key(101, "crate_a", "slot", "app.crate_a.slot.v1")
            .expect("declare memory");

        assert!(MemoryRegistry::get(101).is_none());
    }

    #[test]
    fn declare_memory_rejects_after_bootstrap_seal() {
        reset_for_tests();
        MemoryApi::bootstrap_owner_range("crate_a", 100, 102).expect("bootstrap registry");

        let err = MemoryApi::declare_with_key(101, "crate_a", "slot", "app.crate_a.slot.v1")
            .expect_err("late declaration should fail");
        assert!(matches!(
            err,
            MemoryRegistryError::RegistrationAfterBootstrap {
                ranges: 0,
                registrations: 1,
            }
        ));
    }

    #[test]
    fn register_memory_rejects_before_bootstrap_validation() {
        reset_for_tests();

        let Err(err) = MemoryApi::register(100, "crate_a", "slot") else {
            panic!("opening before bootstrap must fail")
        };
        assert!(matches!(err, MemoryRegistryError::RegistryNotBootstrapped));
    }

    #[test]
    fn register_memory_rejects_new_claim_after_bootstrap_seal() {
        reset_for_tests();
        MemoryApi::bootstrap_owner_range("crate_a", 100, 102).expect("bootstrap registry");

        let Err(err) = MemoryApi::register(101, "crate_a", "slot") else {
            panic!("new registration after bootstrap must fail")
        };
        assert!(matches!(
            err,
            MemoryRegistryError::RegistrationAfterBootstrap {
                ranges: 0,
                registrations: 1,
            }
        ));
    }

    #[test]
    fn bootstrap_pending_flushes_deferred_state() {
        reset_for_tests();
        defer_reserve_range("crate_a", 100, 102).expect("defer range");
        defer_register(101, "crate_a", "slot").expect("defer register");

        MemoryApi::bootstrap_pending().expect("bootstrap pending");

        assert!(MemoryRegistry::export_ranges().contains(&(
            "crate_a".to_string(),
            MemoryRange {
                start: 100,
                end: 102
            }
        )));
        let entries = MemoryRegistry::export();
        assert!(entries.iter().any(|(id, entry)| {
            *id == 101 && entry.crate_name == "crate_a" && entry.label == "slot"
        }));
    }

    #[test]
    fn inspect_memory_returns_reserved_owner_without_label() {
        reset_for_tests();
        MemoryApi::bootstrap_owner_range("crate_a", 100, 102).expect("bootstrap registry");

        let inspection = MemoryApi::inspect(101).expect("reserved slot should inspect");
        assert_eq!(inspection.owner, "crate_a");
        assert_eq!(
            inspection.range,
            MemoryRange {
                start: 100,
                end: 102
            }
        );
        assert_eq!(inspection.label, None);
    }

    #[test]
    fn inspect_memory_returns_registered_label() {
        reset_for_tests();
        defer_reserve_range("crate_a", 100, 102).expect("defer range");
        defer_register(101, "crate_a", "slot").expect("defer register");
        MemoryApi::bootstrap_pending().expect("bootstrap registry");

        let inspection = MemoryApi::inspect(101).expect("registered slot should inspect");
        assert_eq!(inspection.owner, "crate_a");
        assert_eq!(
            inspection.range,
            MemoryRange {
                start: 100,
                end: 102
            }
        );
        assert_eq!(inspection.label.as_deref(), Some("slot"));
        assert_eq!(
            inspection.stable_key.as_deref(),
            Some("legacy.crate_a.slot.v1")
        );
    }

    #[test]
    fn inspect_memory_returns_none_for_unowned_id() {
        reset_for_tests();
        assert_eq!(MemoryApi::inspect(99), None);
    }

    #[test]
    fn registered_memories_lists_registered_slots_with_owner_context() {
        reset_for_tests();
        defer_reserve_range("crate_a", 100, 102).expect("defer range A");
        defer_reserve_range("crate_b", 110, 112).expect("defer range B");
        defer_register(101, "crate_a", "slot_a").expect("defer register A");
        defer_register(111, "crate_b", "slot_b").expect("defer register B");
        MemoryApi::bootstrap_pending().expect("bootstrap registry");

        let registrations = MemoryApi::registered();
        assert_eq!(registrations.len(), 3);
        assert!(registrations.contains(&RegisteredMemory {
            id: 101,
            owner: "crate_a".to_string(),
            range: MemoryRange {
                start: 100,
                end: 102
            },
            label: "slot_a".to_string(),
            stable_key: "legacy.crate_a.slot_a.v1".to_string(),
            schema_version: None,
            schema_fingerprint: None,
        }));
        assert!(registrations.contains(&RegisteredMemory {
            id: 111,
            owner: "crate_b".to_string(),
            range: MemoryRange {
                start: 110,
                end: 112
            },
            label: "slot_b".to_string(),
            stable_key: "legacy.crate_b.slot_b.v1".to_string(),
            schema_version: None,
            schema_fingerprint: None,
        }));
    }

    #[test]
    fn registered_memories_for_owner_filters_to_owner() {
        reset_for_tests();
        defer_reserve_range("crate_a", 100, 102).expect("defer range A");
        defer_reserve_range("crate_b", 110, 112).expect("defer range B");
        defer_register(101, "crate_a", "slot_a").expect("defer register A");
        defer_register(111, "crate_b", "slot_b").expect("defer register B");
        MemoryApi::bootstrap_pending().expect("bootstrap registry");

        let registrations = MemoryApi::registered_for_owner("crate_a");
        assert_eq!(
            registrations,
            vec![RegisteredMemory {
                id: 101,
                owner: "crate_a".to_string(),
                range: MemoryRange {
                    start: 100,
                    end: 102
                },
                label: "slot_a".to_string(),
                stable_key: "legacy.crate_a.slot_a.v1".to_string(),
                schema_version: None,
                schema_fingerprint: None,
            }]
        );
    }

    #[test]
    fn find_registered_memory_returns_match_for_owner_and_label() {
        reset_for_tests();
        defer_reserve_range("crate_a", 100, 102).expect("defer range");
        defer_register(101, "crate_a", "slot_a").expect("defer register");
        MemoryApi::bootstrap_pending().expect("bootstrap registry");

        let registration = MemoryApi::find("crate_a", "slot_a").expect("slot should exist");
        assert_eq!(
            registration,
            RegisteredMemory {
                id: 101,
                owner: "crate_a".to_string(),
                range: MemoryRange {
                    start: 100,
                    end: 102
                },
                label: "slot_a".to_string(),
                stable_key: "legacy.crate_a.slot_a.v1".to_string(),
                schema_version: None,
                schema_fingerprint: None,
            }
        );
    }

    #[test]
    fn find_registered_memory_returns_none_when_missing() {
        reset_for_tests();
        MemoryApi::bootstrap_owner_range("crate_a", 100, 102).expect("bootstrap registry");
        assert_eq!(MemoryApi::find("crate_a", "slot_a"), None);
    }

    #[test]
    fn ledger_snapshot_reads_historical_records() {
        reset_for_tests();
        defer_reserve_range("crate_a", 100, 102).expect("defer range");
        defer_register(101, "crate_a", "slot").expect("defer register");
        MemoryApi::bootstrap_pending().expect("bootstrap registry");

        let snapshot = MemoryApi::ledger_snapshot().expect("ledger snapshot");
        assert_eq!(snapshot.format_id, 1);
        assert_eq!(snapshot.schema_version, 1);
        assert_eq!(snapshot.layout_epoch, 1);
        assert!(snapshot.authorities.iter().any(|authority| {
            authority.owner == "canic.framework"
                && authority.range == MemoryRange { start: 0, end: 99 }
        }));
        assert!(snapshot.authorities.iter().any(|authority| {
            authority.owner == "applications"
                && authority.range
                    == MemoryRange {
                        start: 100,
                        end: 254,
                    }
        }));
        assert!(snapshot.ranges.iter().any(|(owner, range)| {
            owner == "crate_a"
                && *range
                    == MemoryRange {
                        start: 100,
                        end: 102,
                    }
        }));
        assert!(snapshot.entries.iter().any(|(id, entry)| {
            *id == 101
                && entry.crate_name == "crate_a"
                && entry.label == "slot"
                && entry.stable_key == "legacy.crate_a.slot.v1"
        }));
    }

    #[test]
    fn allocation_ledger_snapshot_projects_generic_records() {
        reset_for_tests();
        defer_reserve_range("crate_a", 100, 102).expect("defer range");
        defer_register(101, "crate_a", "slot").expect("defer register");
        MemoryApi::bootstrap_pending().expect("bootstrap registry");

        let snapshot = MemoryApi::allocation_ledger_snapshot().expect("allocation ledger snapshot");

        assert!(snapshot.allocation_history.records.iter().any(|record| {
            record.stable_key.as_str() == "legacy.crate_a.slot.v1"
                && record.slot == ic_memory::AllocationSlotDescriptor::memory_manager(101)
        }));
    }

    #[test]
    fn allocation_diagnostic_export_uses_generic_shape() {
        reset_for_tests();
        defer_reserve_range("crate_a", 100, 102).expect("defer range");
        defer_register(101, "crate_a", "slot").expect("defer register");
        MemoryApi::bootstrap_pending().expect("bootstrap registry");

        let export =
            MemoryApi::allocation_diagnostic_export().expect("allocation diagnostic export");

        assert_eq!(
            export.ledger_anchor,
            ic_memory::AllocationSlotDescriptor::memory_manager(ledger::MEMORY_LAYOUT_LEDGER_ID)
        );
        assert!(export.records.iter().any(|record| {
            record.allocation.stable_key.as_str() == "legacy.crate_a.slot.v1"
                && record.allocation.slot
                    == ic_memory::AllocationSlotDescriptor::memory_manager(101)
        }));
    }
}
