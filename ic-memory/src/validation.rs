use crate::{
    declaration::DeclarationSnapshot,
    key::StableKey,
    ledger::{AllocationLedger, AllocationRecord, AllocationState},
    policy::AllocationPolicy,
    session::ValidatedAllocations,
    slot::AllocationSlotDescriptor,
};

///
/// AllocationValidationError
///
/// Failure to validate declarations against policy and historical ledger facts.
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum AllocationValidationError<P> {
    /// Policy adapter rejected the declaration.
    #[error("allocation policy rejected a declaration")]
    Policy(P),
    /// Stable key was historically bound to a different slot.
    #[error("stable key '{stable_key}' was historically bound to a different allocation slot")]
    StableKeySlotConflict {
        /// Stable key that was redeclared.
        stable_key: StableKey,
        /// Historical slot for the stable key.
        historical_slot: Box<AllocationSlotDescriptor>,
        /// Slot claimed by the current declaration.
        declared_slot: Box<AllocationSlotDescriptor>,
    },
    /// Slot was historically bound to a different stable key.
    #[error("allocation slot '{slot:?}' was historically bound to stable key '{historical_key}'")]
    SlotStableKeyConflict {
        /// Slot claimed by the current declaration.
        slot: Box<AllocationSlotDescriptor>,
        /// Historical stable key for the slot.
        historical_key: StableKey,
        /// Stable key claimed by the current declaration.
        declared_key: StableKey,
    },
    /// Current declaration attempted to revive a retired allocation.
    #[error("stable key '{stable_key}' was explicitly retired and cannot be redeclared")]
    RetiredAllocation {
        /// Retired stable key.
        stable_key: StableKey,
        /// Retired allocation slot.
        slot: Box<AllocationSlotDescriptor>,
    },
}

/// Validate current declarations against framework policy and ledger history.
///
/// This proves allocation ABI safety only. It does not prove store-level schema
/// compatibility.
pub fn validate_allocations<P: AllocationPolicy>(
    ledger: &AllocationLedger,
    snapshot: DeclarationSnapshot,
    policy: &P,
) -> Result<ValidatedAllocations, AllocationValidationError<P::Error>> {
    for declaration in snapshot.declarations() {
        policy
            .validate_key(&declaration.stable_key)
            .map_err(AllocationValidationError::Policy)?;
        policy
            .validate_slot(&declaration.stable_key, &declaration.slot)
            .map_err(AllocationValidationError::Policy)?;

        validate_declaration_history(ledger, declaration)?;
    }

    let (declarations, runtime_fingerprint) = snapshot.into_parts();

    Ok(ValidatedAllocations::new(
        ledger.current_generation,
        declarations,
        runtime_fingerprint,
    ))
}

fn validate_declaration_history<P>(
    ledger: &AllocationLedger,
    declaration: &crate::declaration::AllocationDeclaration,
) -> Result<(), AllocationValidationError<P>> {
    if let Some(record) = find_by_key(ledger, &declaration.stable_key) {
        if record.state == AllocationState::Retired {
            return Err(AllocationValidationError::RetiredAllocation {
                stable_key: declaration.stable_key.clone(),
                slot: Box::new(record.slot.clone()),
            });
        }
        if record.slot != declaration.slot {
            return Err(AllocationValidationError::StableKeySlotConflict {
                stable_key: declaration.stable_key.clone(),
                historical_slot: Box::new(record.slot.clone()),
                declared_slot: Box::new(declaration.slot.clone()),
            });
        }
    }

    if let Some(record) = find_by_slot(ledger, &declaration.slot)
        && record.stable_key != declaration.stable_key
    {
        return Err(AllocationValidationError::SlotStableKeyConflict {
            slot: Box::new(declaration.slot.clone()),
            historical_key: record.stable_key.clone(),
            declared_key: declaration.stable_key.clone(),
        });
    }

    Ok(())
}

fn find_by_key<'ledger>(
    ledger: &'ledger AllocationLedger,
    stable_key: &StableKey,
) -> Option<&'ledger AllocationRecord> {
    ledger
        .allocation_history
        .records
        .iter()
        .find(|record| &record.stable_key == stable_key)
}

fn find_by_slot<'ledger>(
    ledger: &'ledger AllocationLedger,
    slot: &AllocationSlotDescriptor,
) -> Option<&'ledger AllocationRecord> {
    ledger
        .allocation_history
        .records
        .iter()
        .find(|record| &record.slot == slot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        declaration::AllocationDeclaration,
        ledger::{AllocationHistory, AllocationRecord},
        schema::SchemaMetadata,
        slot::AllocationSlotDescriptor,
    };

    #[derive(Debug, Eq, PartialEq)]
    struct TestPolicy;

    impl AllocationPolicy for TestPolicy {
        type Error = &'static str;

        fn validate_key(&self, key: &StableKey) -> Result<(), Self::Error> {
            if key.as_str().starts_with("bad.") {
                return Err("bad key");
            }
            Ok(())
        }

        fn validate_slot(
            &self,
            _key: &StableKey,
            slot: &AllocationSlotDescriptor,
        ) -> Result<(), Self::Error> {
            if slot == &AllocationSlotDescriptor::memory_manager(255) {
                return Err("bad slot");
            }
            Ok(())
        }

        fn validate_reserved_slot(
            &self,
            _key: &StableKey,
            _slot: &AllocationSlotDescriptor,
        ) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    fn ledger(records: Vec<AllocationRecord>) -> AllocationLedger {
        AllocationLedger {
            ledger_schema_version: 1,
            physical_format_id: 1,
            current_generation: 7,
            allocation_history: AllocationHistory {
                records,
                generations: Vec::new(),
            },
        }
    }

    fn declaration(key: &str, id: u8) -> AllocationDeclaration {
        AllocationDeclaration::new(
            key,
            AllocationSlotDescriptor::memory_manager(id),
            None,
            SchemaMetadata::default(),
        )
        .expect("declaration")
    }

    fn active_record(key: &str, id: u8) -> AllocationRecord {
        AllocationRecord::from_declaration(1, declaration(key, id), AllocationState::Active)
    }

    #[test]
    fn accepts_matching_historical_owner() {
        let snapshot =
            DeclarationSnapshot::new(vec![declaration("app.users.v1", 100)]).expect("snapshot");

        let validated = validate_allocations(
            &ledger(vec![active_record("app.users.v1", 100)]),
            snapshot,
            &TestPolicy,
        )
        .expect("validated");

        assert_eq!(validated.generation(), 7);
    }

    #[test]
    fn omitted_historical_records_do_not_fail_validation() {
        let snapshot =
            DeclarationSnapshot::new(vec![declaration("app.users.v1", 100)]).expect("snapshot");

        validate_allocations(
            &ledger(vec![
                active_record("app.users.v1", 100),
                active_record("app.orders.v1", 101),
            ]),
            snapshot,
            &TestPolicy,
        )
        .expect("omitted records are preserved, not retired");
    }

    #[test]
    fn rejects_same_key_different_slot() {
        let snapshot =
            DeclarationSnapshot::new(vec![declaration("app.users.v1", 101)]).expect("snapshot");

        let err = validate_allocations(
            &ledger(vec![active_record("app.users.v1", 100)]),
            snapshot,
            &TestPolicy,
        )
        .expect_err("conflict");

        assert!(matches!(
            err,
            AllocationValidationError::StableKeySlotConflict { .. }
        ));
    }

    #[test]
    fn rejects_same_slot_different_key() {
        let snapshot =
            DeclarationSnapshot::new(vec![declaration("app.orders.v1", 100)]).expect("snapshot");

        let err = validate_allocations(
            &ledger(vec![active_record("app.users.v1", 100)]),
            snapshot,
            &TestPolicy,
        )
        .expect_err("conflict");

        assert!(matches!(
            err,
            AllocationValidationError::SlotStableKeyConflict { .. }
        ));
    }

    #[test]
    fn rejects_retired_redeclaration() {
        let mut record = active_record("app.users.v1", 100);
        record.state = AllocationState::Retired;
        record.retired_generation = Some(3);
        let snapshot =
            DeclarationSnapshot::new(vec![declaration("app.users.v1", 100)]).expect("snapshot");

        let err = validate_allocations(&ledger(vec![record]), snapshot, &TestPolicy)
            .expect_err("retired");

        assert!(matches!(
            err,
            AllocationValidationError::RetiredAllocation { .. }
        ));
    }

    #[test]
    fn policy_rejections_fail_before_validation_succeeds() {
        let snapshot =
            DeclarationSnapshot::new(vec![declaration("bad.users.v1", 100)]).expect("snapshot");

        let err = validate_allocations(&ledger(Vec::new()), snapshot, &TestPolicy)
            .expect_err("policy failure");

        assert_eq!(err, AllocationValidationError::Policy("bad key"));
    }
}
