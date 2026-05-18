use ic_memory::{
    AllocationDeclaration, AllocationHistory, AllocationLedger, AllocationPolicy, AllocationSlot,
    AllocationSlotDescriptor, DeclarationSnapshot, RangeAuthority, SchemaMetadata, StableKey,
    validate_allocations,
};

const IC_MEMORY_INTERNAL_MIN_ID: u8 = 0;
const IC_MEMORY_INTERNAL_MAX_ID: u8 = 9;
const CANIC_FRAMEWORK_MIN_ID: u8 = 10;
const CANIC_FRAMEWORK_MAX_ID: u8 = 99;
const APPLICATION_MIN_ID: u8 = 100;
const INVALID_MEMORY_MANAGER_ID: u8 = u8::MAX;

///
/// CanicMemoryManagerPolicy
///
/// Test adapter proving Canic's `ic-memory` policy mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CanicMemoryManagerPolicy;

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
        validate_key_slot_claim(key, slot, false)
    }

    fn validate_reserved_slot(
        &self,
        key: &StableKey,
        slot: &AllocationSlotDescriptor,
    ) -> Result<(), Self::Error> {
        validate_key_slot_claim(key, slot, true)
    }
}

impl RangeAuthority for CanicMemoryManagerPolicy {
    type Error = CanicMemoryPolicyError;

    fn validate_slot(&self, slot: &AllocationSlotDescriptor) -> Result<(), Self::Error> {
        let id = memory_manager_id(slot)?;
        if id == INVALID_MEMORY_MANAGER_ID {
            return Err(CanicMemoryPolicyError::InvalidMemoryManagerId { id });
        }
        Ok(())
    }
}

///
/// CanicMemoryPolicyError
///
/// Canic allocation policy rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CanicMemoryPolicyError {
    /// Slot is not a `MemoryManagerId` descriptor.
    UnsupportedSlot,
    /// MemoryManager ID 255 is permanently invalid.
    InvalidMemoryManagerId {
        /// Invalid ID.
        id: u8,
    },
    /// Stable-key namespace does not own the claimed MemoryManager ID.
    RangeAuthorityViolation,
    /// Application reservations are not part of the Canic framework plan.
    ApplicationReservation,
}

fn validate_key_slot_claim(
    key: &StableKey,
    slot: &AllocationSlotDescriptor,
    reservation: bool,
) -> Result<(), CanicMemoryPolicyError> {
    let id = memory_manager_id(slot)?;
    if id == INVALID_MEMORY_MANAGER_ID {
        return Err(CanicMemoryPolicyError::InvalidMemoryManagerId { id });
    }

    let key = key.as_str();
    if key.starts_with("ic_memory.") {
        return in_range(id, IC_MEMORY_INTERNAL_MIN_ID, IC_MEMORY_INTERNAL_MAX_ID);
    }
    if key.starts_with("canic.") {
        return in_range(id, CANIC_FRAMEWORK_MIN_ID, CANIC_FRAMEWORK_MAX_ID);
    }
    if reservation {
        return Err(CanicMemoryPolicyError::ApplicationReservation);
    }
    in_range(id, APPLICATION_MIN_ID, INVALID_MEMORY_MANAGER_ID - 1)
}

const fn memory_manager_id(slot: &AllocationSlotDescriptor) -> Result<u8, CanicMemoryPolicyError> {
    match &slot.slot {
        AllocationSlot::MemoryManagerId(id) => Ok(*id),
        AllocationSlot::NamedPartition(_) | AllocationSlot::Custom { .. } => {
            Err(CanicMemoryPolicyError::UnsupportedSlot)
        }
    }
}

fn in_range(id: u8, min: u8, max: u8) -> Result<(), CanicMemoryPolicyError> {
    if (min..=max).contains(&id) {
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
    let policy = CanicMemoryManagerPolicy;

    validate_claim(policy, "ic_memory.ledger.v1", 0).expect("ledger slot");
    validate_reservation(policy, "ic_memory.generation_log.v1", 1).expect("reserved internal slot");
    validate_claim(policy, "ic_memory.maintenance_journal.v1", 9).expect("internal reserve slot");
}

#[test]
fn canic_policy_rejects_ic_memory_key_outside_internal_range() {
    let err = validate_claim(CanicMemoryManagerPolicy, "ic_memory.ledger.v1", 10)
        .expect_err("range violation");

    assert_eq!(err, CanicMemoryPolicyError::RangeAuthorityViolation);
}

#[test]
fn canic_policy_accepts_canic_framework_slots() {
    let policy = CanicMemoryManagerPolicy;

    validate_claim(policy, "canic.core.app_state.v1", 10).expect("first framework slot");
    validate_claim(policy, "canic.core.subnet_state.v1", 99).expect("last framework slot");
}

#[test]
fn canic_policy_rejects_canic_key_in_application_range() {
    let err = validate_claim(CanicMemoryManagerPolicy, "canic.core.app_state.v1", 100)
        .expect_err("range violation");

    assert_eq!(err, CanicMemoryPolicyError::RangeAuthorityViolation);
}

#[test]
fn canic_policy_accepts_application_slots() {
    let policy = CanicMemoryManagerPolicy;

    validate_claim(policy, "app.users.v1", 100).expect("first app slot");
    validate_claim(policy, "app.users.v1", 254).expect("last app slot");
}

#[test]
fn canic_policy_rejects_application_key_below_application_range() {
    let err =
        validate_claim(CanicMemoryManagerPolicy, "app.users.v1", 99).expect_err("range violation");

    assert_eq!(err, CanicMemoryPolicyError::RangeAuthorityViolation);
}

#[test]
fn canic_policy_rejects_id_255_for_all_namespaces() {
    let policy = CanicMemoryManagerPolicy;

    for stable_key in [
        "ic_memory.ledger.v1",
        "canic.core.app_state.v1",
        "app.users.v1",
    ] {
        let err = validate_claim(policy, stable_key, 255).expect_err("invalid sentinel");

        assert_eq!(
            err,
            CanicMemoryPolicyError::InvalidMemoryManagerId { id: 255 }
        );
    }
}

#[test]
fn canic_policy_rejects_application_reservations() {
    let err = validate_reservation(CanicMemoryManagerPolicy, "app.users.v1", 100)
        .expect_err("app reservation");

    assert_eq!(err, CanicMemoryPolicyError::ApplicationReservation);
}

#[test]
fn canic_policy_rejects_non_memory_manager_slots() {
    let slot = AllocationSlotDescriptor {
        slot: AllocationSlot::NamedPartition("ledger".to_string()),
        substrate: "named".to_string(),
        descriptor_version: 1,
    };

    let err =
        AllocationPolicy::validate_slot(&CanicMemoryManagerPolicy, &key("app.users.v1"), &slot)
            .expect_err("unsupported slot");

    assert_eq!(err, CanicMemoryPolicyError::UnsupportedSlot);
}

#[test]
fn canic_policy_integrates_with_ic_memory_allocation_validation() {
    let snapshot = DeclarationSnapshot::new(vec![
        declaration("ic_memory.ledger.v1", 0),
        declaration("canic.core.app_state.v1", 10),
        declaration("app.users.v1", 100),
    ])
    .expect("snapshot");

    let validated = validate_allocations(&ledger(), snapshot, &CanicMemoryManagerPolicy)
        .expect("validated allocations");

    assert_eq!(validated.declarations().len(), 3);
}

#[test]
fn canic_policy_rejection_flows_through_ic_memory_allocation_validation() {
    let snapshot = DeclarationSnapshot::new(vec![declaration("canic.core.app_state.v1", 100)])
        .expect("snapshot");

    let err = validate_allocations(&ledger(), snapshot, &CanicMemoryManagerPolicy)
        .expect_err("range violation");

    assert!(matches!(
        err,
        ic_memory::AllocationValidationError::Policy(
            CanicMemoryPolicyError::RangeAuthorityViolation
        )
    ));
}
