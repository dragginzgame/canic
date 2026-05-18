use ic_memory::{
    AllocationDeclaration, AllocationHistory, AllocationLedger, AllocationPolicy, AllocationSlot,
    AllocationSlotDescriptor, DeclarationSnapshot, MEMORY_MANAGER_INVALID_ID, MemoryManagerIdRange,
    MemoryManagerSlotError, RangeAuthority, SchemaMetadata, StableKey, validate_allocations,
};

fn ic_memory_internal_range() -> MemoryManagerIdRange {
    MemoryManagerIdRange::new(0, 9).expect("ic-memory internal range")
}

fn canic_framework_range() -> MemoryManagerIdRange {
    MemoryManagerIdRange::new(10, 99).expect("Canic framework range")
}

fn application_range() -> MemoryManagerIdRange {
    MemoryManagerIdRange::new(100, 254).expect("application range")
}

///
/// CanicMemoryManagerPolicy
///
/// Test adapter proving Canic's `ic-memory` policy mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CanicMemoryManagerPolicy {
    declaring_crate: &'static str,
    declaring_crates: &'static [(&'static str, &'static str)],
}

impl CanicMemoryManagerPolicy {
    const fn for_declaring_crate(declaring_crate: &'static str) -> Self {
        Self {
            declaring_crate,
            declaring_crates: &[],
        }
    }

    const fn for_declarations(declaring_crates: &'static [(&'static str, &'static str)]) -> Self {
        Self {
            declaring_crate: "",
            declaring_crates,
        }
    }

    fn declaring_crate_for_key(&self, key: &StableKey) -> &str {
        self.declaring_crates
            .iter()
            .find_map(|(stable_key, declaring_crate)| {
                (*stable_key == key.as_str()).then_some(*declaring_crate)
            })
            .unwrap_or(self.declaring_crate)
    }
}

impl AllocationPolicy for CanicMemoryManagerPolicy {
    type Error = CanicMemoryPolicyError;

    fn validate_key(&self, _key: &StableKey) -> Result<(), Self::Error> {
        Ok(())
    }

    fn validate_slot(
        &self,
        key: &StableKey,
        slot: &AllocationSlotDescriptor,
    ) -> Result<(), Self::Error> {
        validate_key_slot_claim(self.declaring_crate_for_key(key), key, slot, false)
    }

    fn validate_reserved_slot(
        &self,
        key: &StableKey,
        slot: &AllocationSlotDescriptor,
    ) -> Result<(), Self::Error> {
        validate_key_slot_claim(self.declaring_crate_for_key(key), key, slot, true)
    }
}

impl RangeAuthority for CanicMemoryManagerPolicy {
    type Error = CanicMemoryPolicyError;

    fn validate_slot(&self, slot: &AllocationSlotDescriptor) -> Result<(), Self::Error> {
        slot.memory_manager_id()
            .map_err(CanicMemoryPolicyError::MemoryManagerSlot)?;
        Ok(())
    }
}

///
/// CanicMemoryPolicyError
///
/// Canic allocation policy rejection.
#[derive(Clone, Debug, Eq, PartialEq)]
enum CanicMemoryPolicyError {
    /// Slot is not a usable `MemoryManager` descriptor.
    MemoryManagerSlot(MemoryManagerSlotError),
    /// Stable-key namespace does not own the claimed MemoryManager ID.
    RangeAuthorityViolation,
    /// The declaring crate does not own the stable-key namespace.
    NamespaceOwnerViolation,
    /// Application reservations are not part of the Canic framework plan.
    ApplicationReservation,
}

fn validate_key_slot_claim(
    declaring_crate: &str,
    key: &StableKey,
    slot: &AllocationSlotDescriptor,
    reservation: bool,
) -> Result<(), CanicMemoryPolicyError> {
    let id = slot
        .memory_manager_id()
        .map_err(CanicMemoryPolicyError::MemoryManagerSlot)?;

    let key = key.as_str();
    if key.starts_with("ic_memory.") {
        if declaring_crate != "ic-memory" {
            return Err(CanicMemoryPolicyError::NamespaceOwnerViolation);
        }
        return in_range(id, ic_memory_internal_range());
    }
    if key.starts_with("canic.") {
        if !declaring_crate.starts_with("canic") {
            return Err(CanicMemoryPolicyError::NamespaceOwnerViolation);
        }
        return in_range(id, canic_framework_range());
    }
    if reservation {
        return Err(CanicMemoryPolicyError::ApplicationReservation);
    }
    in_range(id, application_range())
}

const fn in_range(id: u8, range: MemoryManagerIdRange) -> Result<(), CanicMemoryPolicyError> {
    if range.contains(id) {
        return Ok(());
    }
    Err(CanicMemoryPolicyError::RangeAuthorityViolation)
}

fn key(value: &str) -> StableKey {
    StableKey::parse(value).expect("stable key")
}

fn memory(id: u8) -> AllocationSlotDescriptor {
    AllocationSlotDescriptor::memory_manager(id)
}

fn ledger() -> AllocationLedger {
    AllocationLedger {
        ledger_schema_version: 1,
        physical_format_id: 1,
        current_generation: 0,
        allocation_history: AllocationHistory::default(),
    }
}

fn declaration(stable_key: &str, id: u8) -> AllocationDeclaration {
    AllocationDeclaration::new(stable_key, memory(id), None, SchemaMetadata::default())
        .expect("declaration")
}

fn validate_claim(
    policy: CanicMemoryManagerPolicy,
    stable_key: &str,
    id: u8,
) -> Result<(), CanicMemoryPolicyError> {
    AllocationPolicy::validate_slot(&policy, &key(stable_key), &memory(id))
}

fn validate_reservation(
    policy: CanicMemoryManagerPolicy,
    stable_key: &str,
    id: u8,
) -> Result<(), CanicMemoryPolicyError> {
    policy.validate_reserved_slot(&key(stable_key), &memory(id))
}

#[test]
fn canic_policy_accepts_ic_memory_internal_governance_slots() {
    let policy = CanicMemoryManagerPolicy::for_declaring_crate("ic-memory");

    validate_claim(policy, "ic_memory.ledger.v1", 0).expect("ledger slot");
    validate_reservation(policy, "ic_memory.generation_log.v1", 1).expect("reserved internal slot");
    validate_claim(policy, "ic_memory.maintenance_journal.v1", 9).expect("internal reserve slot");
}

#[test]
fn canic_policy_rejects_ic_memory_key_outside_internal_range() {
    let policy = CanicMemoryManagerPolicy::for_declaring_crate("ic-memory");
    let err = validate_claim(policy, "ic_memory.ledger.v1", 10).expect_err("range violation");

    assert_eq!(err, CanicMemoryPolicyError::RangeAuthorityViolation);
}

#[test]
fn canic_policy_rejects_ic_memory_key_from_non_ic_memory_crate() {
    let policy = CanicMemoryManagerPolicy::for_declaring_crate("canic-core");
    let err = validate_claim(policy, "ic_memory.ledger.v1", 0).expect_err("namespace owner");

    assert_eq!(err, CanicMemoryPolicyError::NamespaceOwnerViolation);
}

#[test]
fn canic_policy_accepts_canic_framework_slots() {
    let policy = CanicMemoryManagerPolicy::for_declaring_crate("canic-core");

    validate_claim(policy, "canic.core.app_state.v1", 10).expect("first framework slot");
    validate_claim(policy, "canic.core.subnet_state.v1", 99).expect("last framework slot");
}

#[test]
fn canic_policy_rejects_canic_key_in_application_range() {
    let policy = CanicMemoryManagerPolicy::for_declaring_crate("canic-core");
    let err = validate_claim(policy, "canic.core.app_state.v1", 100).expect_err("range violation");

    assert_eq!(err, CanicMemoryPolicyError::RangeAuthorityViolation);
}

#[test]
fn canic_policy_accepts_application_slots() {
    let policy = CanicMemoryManagerPolicy::for_declaring_crate("app-crate");

    validate_claim(policy, "app.users.v1", 100).expect("first app slot");
    validate_claim(policy, "app.users.v1", 254).expect("last app slot");
}

#[test]
fn canic_policy_rejects_application_key_below_application_range() {
    let policy = CanicMemoryManagerPolicy::for_declaring_crate("app-crate");
    let err = validate_claim(policy, "app.users.v1", 99).expect_err("range violation");

    assert_eq!(err, CanicMemoryPolicyError::RangeAuthorityViolation);
}

#[test]
fn canic_policy_rejects_id_255_for_all_namespaces() {
    for (declaring_crate, stable_key) in [
        ("ic-memory", "ic_memory.ledger.v1"),
        ("canic-core", "canic.core.app_state.v1"),
        ("app-crate", "app.users.v1"),
    ] {
        let policy = CanicMemoryManagerPolicy::for_declaring_crate(declaring_crate);
        let err =
            validate_claim(policy, stable_key, MEMORY_MANAGER_INVALID_ID).expect_err("sentinel");

        assert_eq!(
            err,
            CanicMemoryPolicyError::MemoryManagerSlot(
                MemoryManagerSlotError::InvalidMemoryManagerId {
                    id: MEMORY_MANAGER_INVALID_ID
                }
            )
        );
    }
}

#[test]
fn canic_policy_rejects_application_reservations() {
    let policy = CanicMemoryManagerPolicy::for_declaring_crate("app-crate");
    let err = validate_reservation(policy, "app.users.v1", 100).expect_err("app reservation");

    assert_eq!(err, CanicMemoryPolicyError::ApplicationReservation);
}

#[test]
fn canic_policy_rejects_non_memory_manager_slots() {
    let slot = AllocationSlotDescriptor {
        slot: AllocationSlot::NamedPartition("ledger".to_string()),
        substrate: "named".to_string(),
        descriptor_version: 1,
    };

    let policy = CanicMemoryManagerPolicy::for_declaring_crate("app-crate");
    let err = AllocationPolicy::validate_slot(&policy, &key("app.users.v1"), &slot)
        .expect_err("unsupported slot");

    assert!(matches!(err, CanicMemoryPolicyError::MemoryManagerSlot(_)));
}

#[test]
fn canic_policy_integrates_with_ic_memory_allocation_validation() {
    let snapshot = DeclarationSnapshot::new(vec![
        declaration("ic_memory.ledger.v1", 0),
        declaration("canic.core.app_state.v1", 10),
        declaration("app.users.v1", 100),
    ])
    .expect("snapshot");

    let policy = CanicMemoryManagerPolicy::for_declarations(&[
        ("ic_memory.ledger.v1", "ic-memory"),
        ("canic.core.app_state.v1", "canic-core"),
        ("app.users.v1", "app-crate"),
    ]);
    let validated =
        validate_allocations(&ledger(), snapshot, &policy).expect("validated allocations");

    assert_eq!(validated.declarations().len(), 3);
}

#[test]
fn canic_policy_rejection_flows_through_ic_memory_allocation_validation() {
    let snapshot = DeclarationSnapshot::new(vec![declaration("canic.core.app_state.v1", 100)])
        .expect("snapshot");

    let policy =
        CanicMemoryManagerPolicy::for_declarations(&[("canic.core.app_state.v1", "canic-core")]);
    let err = validate_allocations(&ledger(), snapshot, &policy).expect_err("range violation");

    assert!(matches!(
        err,
        ic_memory::AllocationValidationError::Policy(
            CanicMemoryPolicyError::RangeAuthorityViolation
        )
    ));
}
