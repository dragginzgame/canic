use super::super::manager::{self, RawStableMemoryState};
use super::super::registry::{
    MemoryRange, MemoryRegistry, MemoryRegistryEntry, MemoryRegistryError, PendingRegistration,
    drain_pending_ranges, drain_pending_registrations,
};
use super::super::{ledger, policy::CanicMemoryManagerPolicy};
use ic_memory::{
    AllocationDeclaration, AllocationPolicy, AllocationSlotDescriptor, AllocationValidationError,
    DeclarationSnapshot, DeclarationSnapshotError, MemoryManagerSlotError, SchemaMetadata,
    StableKey, ValidatedAllocations, validate_allocations,
};
#[cfg(test)]
use std::cell::Cell;
#[cfg(not(test))]
use std::sync::atomic::{AtomicBool, Ordering};
use std::{cell::RefCell, collections::BTreeMap};

#[cfg(not(test))]
static MEMORY_REGISTRY_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
thread_local! {
    static MEMORY_REGISTRY_INITIALIZED: Cell<bool> = const { Cell::new(false) };
}

thread_local! {
    static VALIDATED_ALLOCATIONS: RefCell<Option<ValidatedAllocations>> = const {
        RefCell::new(None)
    };
}

///
/// MemoryRegistryInitSummary
///
/// Substrate-level summary of registry state after initialization.
/// This is intended for diagnostics and testing only.
/// It is NOT a stable API contract or external view.
///

#[derive(Debug)]
pub struct MemoryRegistryInitSummary {
    /// Reserved owner ranges after initialization completes.
    pub ranges: Vec<(String, MemoryRange)>,
    /// Registered memory IDs after initialization completes.
    pub entries: Vec<(u8, MemoryRegistryEntry)>,
}

///
/// MemoryRegistryRuntime
///
/// Substrate runtime controller responsible for initializing the
/// global memory registry.
///
/// This type performs mechanical coordination only:
/// - ordering
/// - conflict detection
/// - idempotent initialization
///
/// It encodes no application semantics.
///
pub struct MemoryRegistryRuntime;

impl MemoryRegistryRuntime {
    /// Initialize the memory registry.
    ///
    /// - Optionally reserves an initial range for the caller.
    /// - Applies all deferred range reservations.
    /// - Applies all deferred ID registrations.
    ///
    /// This function is idempotent for the same initial range.
    pub fn init(
        initial_range: Option<(&str, u8, u8)>,
    ) -> Result<MemoryRegistryInitSummary, MemoryRegistryError> {
        let raw_state = manager::classify_raw_stable_memory();
        validate_raw_stable_memory_state(raw_state)?;
        ledger::validate_bootstrap_state_before_cell_init(raw_state)?;

        // Apply deferred range reservations deterministically
        let mut ranges = drain_pending_ranges();
        ranges.sort_by_key(|(_, start, _)| *start);

        // Apply deferred registrations deterministically
        let mut regs = drain_pending_registrations();
        regs.sort_by_key(|registration| registration.id);
        let has_runtime_declarations = !regs.is_empty();
        let declaration_snapshot = validate_current_registration_snapshot(&regs)?;

        MemoryRegistry::reserve_internal_layout_ledger()?;
        let validated_allocations =
            validate_pending_ledger_claims(initial_range, &ranges, &regs, declaration_snapshot)?;

        // Reserve the caller's initial range first (if provided)
        if let Some((crate_name, start, end)) = initial_range {
            MemoryRegistry::reserve_range(crate_name, start, end)?;
        }

        for (crate_name, start, end) in ranges {
            MemoryRegistry::reserve_range(&crate_name, start, end)?;
        }

        for registration in regs {
            MemoryRegistry::register_with_key_metadata(
                registration.id,
                &registration.crate_name,
                &registration.label,
                &registration.stable_key,
                registration.schema_version,
                registration.schema_fingerprint.as_deref(),
            )?;
        }

        let summary = MemoryRegistryInitSummary {
            ranges: MemoryRegistry::export_ranges(),
            entries: MemoryRegistry::export(),
        };
        if !Self::is_initialized() || has_runtime_declarations {
            set_validated_allocations(Some(validated_allocations));
        }
        set_initialized(true);

        Ok(summary)
    }

    /// Return whether the memory registry has completed initialization.
    #[must_use]
    pub fn is_initialized() -> bool {
        initialized()
    }

    /// Snapshot all registry entries.
    #[must_use]
    pub fn snapshot_entries() -> Vec<(u8, MemoryRegistryEntry)> {
        MemoryRegistry::export()
    }

    /// Return the sealed validated allocation set published by bootstrap.
    pub fn validated_allocations() -> Result<ValidatedAllocations, MemoryRegistryError> {
        if !Self::is_initialized() {
            return Err(MemoryRegistryError::RegistryNotBootstrapped);
        }

        VALIDATED_ALLOCATIONS.with_borrow(|validated| {
            validated
                .clone()
                .ok_or(MemoryRegistryError::RegistryNotBootstrapped)
        })
    }

    /// Apply any newly deferred registrations/ranges after runtime init.
    ///
    /// This is a no-op until initialization has completed. Once initialized,
    /// this drains pending range/ID registrations so lazily touched statics can
    /// become visible during the same request.
    pub fn commit_pending_if_initialized() -> Result<(), MemoryRegistryError> {
        if !Self::is_initialized() || super::is_eager_tls_initializing() {
            return Ok(());
        }

        let ranges = drain_pending_ranges();
        let regs = drain_pending_registrations();

        if ranges.is_empty() && regs.is_empty() {
            return Ok(());
        }

        Err(MemoryRegistryError::RegistrationAfterBootstrap {
            ranges: ranges.len(),
            registrations: regs.len(),
        })
    }
}

const fn validate_raw_stable_memory_state(
    raw_state: RawStableMemoryState,
) -> Result<(), MemoryRegistryError> {
    match raw_state {
        RawStableMemoryState::Empty | RawStableMemoryState::MemoryManager => Ok(()),
        RawStableMemoryState::ForeignOrCorrupt => Err(MemoryRegistryError::LedgerCorrupt {
            reason: "foreign or corrupt raw stable memory state",
        }),
    }
}

fn validate_current_registration_snapshot(
    regs: &[PendingRegistration],
) -> Result<DeclarationSnapshot, MemoryRegistryError> {
    let declarations = regs
        .iter()
        .map(allocation_declaration_from_pending)
        .collect::<Result<Vec<_>, _>>()?;

    DeclarationSnapshot::new(declarations).map_err(memory_registry_error_from_snapshot_error)
}

fn allocation_declaration_from_pending(
    registration: &PendingRegistration,
) -> Result<AllocationDeclaration, MemoryRegistryError> {
    let slot = AllocationSlotDescriptor::memory_manager_checked(registration.id)
        .map_err(memory_registry_error_from_slot_error)?;
    let schema = SchemaMetadata::new(
        registration.schema_version,
        registration.schema_fingerprint.clone(),
    )
    .map_err(|err| MemoryRegistryError::InvalidSchemaMetadata {
        stable_key: registration.stable_key.clone(),
        reason: super::super::registry::schema_metadata_reason(err),
    })?;

    AllocationDeclaration::new(
        &registration.stable_key,
        slot,
        Some(registration.label.clone()),
        schema,
    )
    .map_err(memory_registry_error_from_snapshot_error)
}

fn memory_registry_error_from_snapshot_error(err: DeclarationSnapshotError) -> MemoryRegistryError {
    match err {
        DeclarationSnapshotError::Key(err) => MemoryRegistryError::InvalidStableKey {
            stable_key: err.stable_key,
            reason: err.reason,
        },
        DeclarationSnapshotError::SchemaMetadata(err) => {
            MemoryRegistryError::InvalidSchemaMetadata {
                stable_key: "<unknown>".to_string(),
                reason: super::super::registry::schema_metadata_reason(err),
            }
        }
        DeclarationSnapshotError::DuplicateStableKey(key) => {
            MemoryRegistryError::DuplicateStableKey(key.into_string())
        }
        DeclarationSnapshotError::DuplicateSlot(slot) => match slot.memory_manager_id() {
            Ok(id) => MemoryRegistryError::DuplicateId(id),
            Err(err) => memory_registry_error_from_slot_error(err),
        },
    }
}

fn memory_registry_error_from_slot_error(err: MemoryManagerSlotError) -> MemoryRegistryError {
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

fn validate_pending_ledger_claims(
    initial_range: Option<(&str, u8, u8)>,
    ranges: &[(String, u8, u8)],
    regs: &[PendingRegistration],
    declaration_snapshot: DeclarationSnapshot,
) -> Result<ValidatedAllocations, MemoryRegistryError> {
    if let Some((owner, start, end)) = initial_range {
        ledger::validate_range(owner, MemoryRange { start, end })?;
    }

    for (owner, start, end) in ranges {
        ledger::validate_range(
            owner,
            MemoryRange {
                start: *start,
                end: *end,
            },
        )?;
    }

    for registration in regs {
        ledger::validate_entry(
            registration.id,
            &registration.crate_name,
            &registration.label,
            &registration.stable_key,
        )?;
    }

    let historical_ledger = ledger::try_allocation_ledger_snapshot()?;
    let policy = RuntimeDeclarationPolicy::from_registrations(regs);
    validate_allocations(&historical_ledger, declaration_snapshot, &policy)
        .map_err(|err| memory_registry_error_from_allocation_validation(err, regs))
}

///
/// RuntimeDeclarationPolicy
///
/// Adapter that lets generic `ic-memory` validation apply Canic's per-crate
/// namespace/range policy to a sealed multi-crate declaration snapshot.
struct RuntimeDeclarationPolicy {
    declaring_crates: BTreeMap<String, String>,
}

impl RuntimeDeclarationPolicy {
    fn from_registrations(regs: &[PendingRegistration]) -> Self {
        let declaring_crates = regs
            .iter()
            .map(|registration| {
                (
                    registration.stable_key.clone(),
                    registration.crate_name.clone(),
                )
            })
            .collect();
        Self { declaring_crates }
    }
}

impl AllocationPolicy for RuntimeDeclarationPolicy {
    type Error = MemoryRegistryError;

    fn validate_key(&self, _key: &StableKey) -> Result<(), Self::Error> {
        Ok(())
    }

    fn validate_slot(
        &self,
        key: &StableKey,
        slot: &AllocationSlotDescriptor,
    ) -> Result<(), Self::Error> {
        let declaring_crate =
            self.declaring_crates
                .get(key.as_str())
                .ok_or(MemoryRegistryError::LedgerCorrupt {
                    reason: "validated declaration is missing runtime crate ownership metadata",
                })?;
        let policy = CanicMemoryManagerPolicy::for_declaring_crate(declaring_crate);
        AllocationPolicy::validate_slot(&policy, key, slot)
    }

    fn validate_reserved_slot(
        &self,
        key: &StableKey,
        slot: &AllocationSlotDescriptor,
    ) -> Result<(), Self::Error> {
        let declaring_crate =
            self.declaring_crates
                .get(key.as_str())
                .ok_or(MemoryRegistryError::LedgerCorrupt {
                    reason: "validated declaration is missing runtime crate ownership metadata",
                })?;
        let policy = CanicMemoryManagerPolicy::for_declaring_crate(declaring_crate);
        AllocationPolicy::validate_reserved_slot(&policy, key, slot)
    }
}

fn memory_registry_error_from_allocation_validation(
    err: AllocationValidationError<MemoryRegistryError>,
    regs: &[PendingRegistration],
) -> MemoryRegistryError {
    match err {
        AllocationValidationError::Policy(err) => err,
        AllocationValidationError::StableKeySlotConflict {
            stable_key,
            historical_slot,
            declared_slot,
        } => MemoryRegistryError::HistoricalStableKeyConflict {
            stable_key: stable_key.into_string(),
            existing_id: memory_manager_id_from_allocation_slot(&historical_slot),
            new_id: memory_manager_id_from_allocation_slot(&declared_slot),
        },
        AllocationValidationError::SlotStableKeyConflict {
            slot,
            historical_key,
            declared_key,
        } => {
            let requested = regs
                .iter()
                .find(|registration| registration.stable_key == declared_key.as_str());
            MemoryRegistryError::HistoricalIdConflict {
                id: memory_manager_id_from_allocation_slot(&slot),
                existing_crate: historical_key.as_str().to_string(),
                existing_label: historical_key.as_str().to_string(),
                new_crate: requested.map_or_else(
                    || declared_key.as_str().to_string(),
                    |reg| reg.crate_name.clone(),
                ),
                new_label: requested.map_or_else(
                    || declared_key.as_str().to_string(),
                    |reg| reg.label.clone(),
                ),
                new_stable_key: declared_key.into_string(),
            }
        }
        AllocationValidationError::RetiredAllocation { .. } => MemoryRegistryError::LedgerCorrupt {
            reason: "allocation was explicitly retired and cannot be redeclared",
        },
    }
}

fn memory_manager_id_from_allocation_slot(slot: &AllocationSlotDescriptor) -> u8 {
    slot.memory_manager_id()
        .unwrap_or(ic_memory::MEMORY_MANAGER_INVALID_ID)
}

#[cfg(test)]
pub(crate) fn reset_initialized_for_tests() {
    set_initialized(false);
    set_validated_allocations(None);
}

#[cfg(not(test))]
fn initialized() -> bool {
    MEMORY_REGISTRY_INITIALIZED.load(Ordering::SeqCst)
}

#[cfg(test)]
fn initialized() -> bool {
    MEMORY_REGISTRY_INITIALIZED.with(Cell::get)
}

#[cfg(not(test))]
fn set_initialized(value: bool) {
    MEMORY_REGISTRY_INITIALIZED.store(value, Ordering::SeqCst);
}

#[cfg(test)]
fn set_initialized(value: bool) {
    MEMORY_REGISTRY_INITIALIZED.with(|initialized| initialized.set(value));
}

fn set_validated_allocations(value: Option<ValidatedAllocations>) {
    VALIDATED_ALLOCATIONS.with_borrow_mut(|validated| {
        *validated = value;
    });
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::registry::{defer_register_with_key, defer_reserve_range, reset_for_tests};

    #[test]
    fn init_applies_initial_and_pending() {
        reset_for_tests();
        defer_reserve_range("crate_b", 110, 111).expect("defer range");
        defer_register_with_key(110, "crate_b", "B110", "app.crate_b.b110.v1")
            .expect("defer register");

        let summary =
            MemoryRegistryRuntime::init(Some(("crate_a", 100, 102))).expect("init should succeed");

        assert_eq!(summary.ranges.len(), 3);
        assert_eq!(summary.entries.len(), 2);
        assert!(summary.entries.iter().any(|(id, entry)| {
            *id == 110 && entry.crate_name == "crate_b" && entry.label == "B110"
        }));
    }

    #[test]
    fn init_is_idempotent_for_same_initial_range() {
        reset_for_tests();

        MemoryRegistryRuntime::init(Some(("crate_a", 100, 102)))
            .expect("first init should succeed");
        MemoryRegistryRuntime::init(Some(("crate_a", 100, 102)))
            .expect("second init should succeed");
    }

    #[test]
    fn init_returns_error_on_conflict() {
        reset_for_tests();
        defer_reserve_range("crate_a", 100, 102).expect("defer range A");
        defer_reserve_range("crate_b", 102, 104).expect("defer range B");

        let err = MemoryRegistryRuntime::init(None).unwrap_err();
        assert!(matches!(err, MemoryRegistryError::Overlap { .. }));
    }

    #[test]
    fn init_rejects_duplicate_current_snapshot_id_before_user_ledger_mutation() {
        reset_for_tests();
        defer_reserve_range("crate_a", 100, 102).expect("defer range");
        defer_register_with_key(100, "crate_a", "slot_a", "app.crate_a.slot_a.v1")
            .expect("defer first register");
        defer_register_with_key(100, "crate_a", "slot_b", "app.crate_a.slot_b.v1")
            .expect("defer second register");

        let err = MemoryRegistryRuntime::init(None)
            .expect_err("duplicate id in one snapshot should fail");
        assert!(matches!(err, MemoryRegistryError::DuplicateId(100)));
        assert!(
            !MemoryRegistry::export_historical()
                .iter()
                .any(|(id, _)| *id == 100)
        );
    }

    #[test]
    fn init_rejects_duplicate_current_snapshot_stable_key_before_user_ledger_mutation() {
        reset_for_tests();
        defer_reserve_range("crate_a", 100, 102).expect("defer range");
        defer_register_with_key(100, "crate_a", "slot_a", "app.crate_a.slot.v1")
            .expect("defer first register");
        defer_register_with_key(101, "crate_a", "slot_b", "app.crate_a.slot.v1")
            .expect("defer second register");

        let err = MemoryRegistryRuntime::init(None)
            .expect_err("duplicate stable key in one snapshot should fail");
        assert!(
            matches!(err, MemoryRegistryError::DuplicateStableKey(key) if key == "app.crate_a.slot.v1")
        );
        assert!(
            !MemoryRegistry::export_historical()
                .iter()
                .any(|(_, entry)| entry.stable_key == "app.crate_a.slot.v1")
        );
    }

    #[test]
    fn init_rejects_exact_duplicate_current_snapshot_declaration() {
        reset_for_tests();
        defer_reserve_range("crate_a", 100, 102).expect("defer range");
        defer_register_with_key(100, "crate_a", "slot", "app.crate_a.slot.v1")
            .expect("defer first register");
        defer_register_with_key(100, "crate_a", "slot", "app.crate_a.slot.v1")
            .expect("defer second register");

        let err = MemoryRegistryRuntime::init(None)
            .expect_err("exact duplicate declaration in one snapshot should fail");
        assert!(matches!(err, MemoryRegistryError::DuplicateId(100)));
    }

    #[test]
    fn init_rejects_historical_conflict_before_user_ledger_mutation() {
        reset_for_tests();
        ledger::record_range(
            "crate_a",
            MemoryRange {
                start: 100,
                end: 102,
            },
        )
        .expect("record historical range");
        ledger::record_entry(100, "crate_a", "slot", "app.crate_a.slot.v1", None, None)
            .expect("record historical entry");
        defer_reserve_range("crate_a", 100, 102).expect("defer range");
        defer_register_with_key(101, "crate_a", "new_slot", "app.crate_a.new_slot.v1")
            .expect("defer non-conflicting register");
        defer_register_with_key(102, "crate_a", "moved_slot", "app.crate_a.slot.v1")
            .expect("defer conflicting register");

        let err = MemoryRegistryRuntime::init(None)
            .expect_err("historical stable key movement should fail before commit");
        assert!(matches!(
            err,
            MemoryRegistryError::HistoricalStableKeyConflict { .. }
        ));
        assert!(
            !MemoryRegistry::export_historical()
                .iter()
                .any(|(id, _)| *id == 101)
        );
    }

    #[test]
    fn commit_pending_after_init_rejects_late_deferred_items() {
        reset_for_tests();

        MemoryRegistryRuntime::init(Some(("core", 100, 109))).expect("init should succeed");
        defer_reserve_range("late", 110, 120).expect("defer late range");
        defer_register_with_key(112, "late", "late_slot", "app.late.late_slot.v1")
            .expect("defer late register");

        let err = MemoryRegistryRuntime::commit_pending_if_initialized()
            .expect_err("late pending commit should fail after bootstrap seal");
        assert!(matches!(
            err,
            MemoryRegistryError::RegistrationAfterBootstrap {
                ranges: 1,
                registrations: 1,
            }
        ));
        assert!(MemoryRegistry::get(112).is_none());
    }
}
