use super::super::manager;
#[cfg(test)]
use super::super::registry::drain_pending_registrations;
use super::super::registry::{
    MemoryRegistryError, PendingRegistration, memory_registry_error_from_declaration_error,
    static_declarations,
};
use super::super::{ledger, policy::CanicMemoryManagerPolicy};
use ic_memory::{
    AllocationPolicy, AllocationSlotDescriptor, AllocationValidationError, BootstrapError,
    DeclarationSnapshot, StableKey, ValidatedAllocations,
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
    /// - Applies all deferred ID registrations.
    ///
    /// This function is idempotent for the same declaration snapshot.
    pub fn init() -> Result<(), MemoryRegistryError> {
        let raw_state = manager::classify_raw_stable_memory();
        ledger::validate_bootstrap_state_before_cell_init(raw_state)?;

        let mut declarations = static_declarations();
        #[cfg(test)]
        declarations.extend(drain_pending_registrations());
        declarations.insert(0, PendingRegistration::internal_layout_ledger()?);
        let declaration_snapshot = DeclarationSnapshot::new(
            declarations
                .iter()
                .map(|registration| registration.declaration().clone())
                .collect(),
        )
        .map_err(|err| memory_registry_error_from_declaration_error(err, "<snapshot>"))?;
        let validated_allocations =
            validate_and_commit_ledger_claims(&declarations, declaration_snapshot)?;

        set_validated_allocations(Some(validated_allocations));
        set_initialized(true);

        Ok(())
    }

    /// Return whether the memory registry has completed initialization.
    #[must_use]
    pub fn is_initialized() -> bool {
        initialized()
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

    #[cfg(test)]
    pub fn commit_pending_if_initialized() -> Result<(), MemoryRegistryError> {
        if !Self::is_initialized() || super::is_eager_tls_initializing() {
            return Ok(());
        }

        let regs = drain_pending_registrations();

        if regs.is_empty() {
            return Ok(());
        }

        Err(MemoryRegistryError::RegistrationAfterBootstrap {
            registrations: regs.len(),
        })
    }
}

fn validate_and_commit_ledger_claims(
    regs: &[PendingRegistration],
    declaration_snapshot: DeclarationSnapshot,
) -> Result<ValidatedAllocations, MemoryRegistryError> {
    let policy = RuntimeDeclarationPolicy::from_registrations(regs);
    ledger::bootstrap_declarations(declaration_snapshot, &policy)
        .map(|commit| commit.validated)
        .map_err(memory_registry_error_from_bootstrap)
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
                    registration.declaration().stable_key().as_str().to_string(),
                    registration.crate_name().to_string(),
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
) -> MemoryRegistryError {
    match err {
        AllocationValidationError::Policy(err) => err,
        AllocationValidationError::StableKeySlotConflict { .. }
        | AllocationValidationError::SlotStableKeyConflict { .. } => {
            MemoryRegistryError::LedgerCorrupt {
                reason: "ic-memory allocation history rejected a conflicting declaration",
            }
        }
        AllocationValidationError::RetiredAllocation { .. } => MemoryRegistryError::LedgerCorrupt {
            reason: "allocation was explicitly retired and cannot be redeclared",
        },
    }
}

fn memory_registry_error_from_bootstrap<L>(
    err: BootstrapError<L, MemoryRegistryError>,
) -> MemoryRegistryError {
    match err {
        BootstrapError::Ledger(_) => MemoryRegistryError::LedgerCorrupt {
            reason: "native ic-memory ledger recovery or commit failed",
        },
        BootstrapError::Validation(err) => memory_registry_error_from_allocation_validation(err),
        BootstrapError::Staging(_) => MemoryRegistryError::LedgerCorrupt {
            reason: "native ic-memory ledger generation staging failed",
        },
    }
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
    use crate::memory::registry::{defer_register_with_key, reset_for_tests};

    #[test]
    fn init_reports_declared_slot_count() {
        reset_for_tests();
        defer_register_with_key(110, "crate_b", "B110", "app.crate_b.b110.v1")
            .expect("defer register");

        MemoryRegistryRuntime::init().expect("init should succeed");
    }

    #[test]
    fn init_is_idempotent_for_same_snapshot() {
        reset_for_tests();

        MemoryRegistryRuntime::init().expect("first init should succeed");
        MemoryRegistryRuntime::init().expect("second init should succeed");
    }

    #[test]
    fn init_rejects_historical_conflict_before_user_ledger_mutation() {
        reset_for_tests();
        seed_historical_entry(100, "crate_a", "slot", "app.crate_a.slot.v1");
        defer_register_with_key(101, "crate_a", "new_slot", "app.crate_a.new_slot.v1")
            .expect("defer non-conflicting register");
        defer_register_with_key(102, "crate_a", "moved_slot", "app.crate_a.slot.v1")
            .expect("defer conflicting register");

        let err = MemoryRegistryRuntime::init()
            .expect_err("historical stable key movement should fail before commit");
        assert!(matches!(err, MemoryRegistryError::LedgerCorrupt { .. }));
        assert!(
            !ledger::try_export_records()
                .expect("ledger records")
                .iter()
                .any(|record| record.slot().memory_manager_id() == Ok(101))
        );
        assert!(!MemoryRegistryRuntime::is_initialized());
        assert!(matches!(
            crate::memory::try_open_validated_memory("app.crate_a.new_slot.v1", 101),
            Err(MemoryRegistryError::RegistryNotBootstrapped)
        ));
    }

    #[test]
    fn commit_pending_after_init_rejects_late_deferred_items() {
        reset_for_tests();

        MemoryRegistryRuntime::init().expect("init should succeed");
        defer_register_with_key(112, "late", "late_slot", "app.late.late_slot.v1")
            .expect("defer late register");

        let err = MemoryRegistryRuntime::commit_pending_if_initialized()
            .expect_err("late pending commit should fail after bootstrap seal");
        assert!(matches!(
            err,
            MemoryRegistryError::RegistrationAfterBootstrap { registrations: 1 }
        ));
    }

    fn seed_historical_entry(id: u8, owner: &str, label: &str, stable_key: &str) {
        defer_register_with_key(id, owner, label, stable_key).expect("seed declaration");
        MemoryRegistryRuntime::init().expect("seed bootstrap");
        reset_initialized_for_tests();
    }
}
