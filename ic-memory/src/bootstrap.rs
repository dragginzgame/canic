use crate::{
    declaration::DeclarationSnapshot,
    ledger::{AllocationLedger, LedgerCodec, LedgerCommitError, LedgerCommitStore},
    policy::AllocationPolicy,
    session::ValidatedAllocations,
    validation::{AllocationValidationError, validate_allocations},
};

///
/// AllocationBootstrap
///
/// Generic generation bootstrap pipeline.
///
/// This type owns allocation-governance sequencing only. Frameworks own when
/// the pipeline runs, how the ledger store is backed by stable memory, and when
/// endpoint dispatch is allowed.
#[derive(Debug)]
pub struct AllocationBootstrap<'store> {
    store: &'store mut LedgerCommitStore,
}

impl<'store> AllocationBootstrap<'store> {
    /// Build a bootstrap pipeline over a protected ledger commit store.
    pub const fn new(store: &'store mut LedgerCommitStore) -> Self {
        Self { store }
    }

    /// Recover, validate, stage, commit, and publish one allocation generation.
    pub fn validate_and_commit<C, P>(
        &mut self,
        codec: &C,
        snapshot: DeclarationSnapshot,
        policy: &P,
        committed_at: Option<u64>,
    ) -> Result<BootstrapCommit, BootstrapError<C::Error, P::Error>>
    where
        C: LedgerCodec,
        P: AllocationPolicy,
    {
        let prior = self.store.recover(codec).map_err(BootstrapError::Ledger)?;
        let validated =
            validate_allocations(&prior, snapshot, policy).map_err(BootstrapError::Validation)?;
        let staged = prior.stage_validated_generation(&validated, committed_at);
        let committed = self
            .store
            .commit(&staged, codec)
            .map_err(BootstrapError::Ledger)?;

        Ok(BootstrapCommit {
            validated: validated.with_generation(committed.current_generation),
            ledger: committed,
        })
    }
}

///
/// BootstrapCommit
///
/// Result of a successful generic allocation bootstrap commit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapCommit {
    /// Ledger recovered after the protected generation commit.
    pub ledger: AllocationLedger,
    /// Validated allocation declarations tied to the committed generation.
    pub validated: ValidatedAllocations,
}

///
/// BootstrapError
///
/// Failure to recover, validate, or commit an allocation generation.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum BootstrapError<C, P> {
    /// Ledger recovery or protected commit failed.
    #[error(transparent)]
    Ledger(LedgerCommitError<C>),
    /// Policy or historical allocation validation failed.
    #[error(transparent)]
    Validation(AllocationValidationError<P>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        declaration::AllocationDeclaration,
        ledger::{AllocationHistory, AllocationLedger},
        schema::SchemaMetadata,
        slot::AllocationSlotDescriptor,
    };
    use std::cell::RefCell;

    #[derive(Debug, Default)]
    struct TestCodec {
        encoded: RefCell<Option<AllocationLedger>>,
    }

    impl LedgerCodec for TestCodec {
        type Error = &'static str;

        fn encode(&self, ledger: &AllocationLedger) -> Result<Vec<u8>, Self::Error> {
            *self.encoded.borrow_mut() = Some(ledger.clone());
            Ok(ledger.current_generation.to_le_bytes().to_vec())
        }

        fn decode(&self, _bytes: &[u8]) -> Result<AllocationLedger, Self::Error> {
            self.encoded
                .borrow()
                .clone()
                .ok_or("ledger was not encoded")
        }
    }

    #[derive(Debug, Eq, PartialEq)]
    struct TestPolicy;

    impl AllocationPolicy for TestPolicy {
        type Error = &'static str;

        fn validate_key(&self, _key: &crate::StableKey) -> Result<(), Self::Error> {
            Ok(())
        }

        fn validate_slot(
            &self,
            _key: &crate::StableKey,
            _slot: &AllocationSlotDescriptor,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn validate_reserved_slot(
            &self,
            _key: &crate::StableKey,
            _slot: &AllocationSlotDescriptor,
        ) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    fn ledger() -> AllocationLedger {
        AllocationLedger {
            ledger_schema_version: 1,
            physical_format_id: 1,
            current_generation: 0,
            allocation_history: AllocationHistory::default(),
        }
    }

    fn declaration() -> AllocationDeclaration {
        AllocationDeclaration::new(
            "app.users.v1",
            AllocationSlotDescriptor::memory_manager(100),
            None,
            SchemaMetadata::default(),
        )
        .expect("declaration")
    }

    #[test]
    fn validate_and_commit_publishes_committed_generation() {
        let codec = TestCodec::default();
        let mut store = LedgerCommitStore::default();
        store.commit(&ledger(), &codec).expect("initial ledger");
        let snapshot = DeclarationSnapshot::new(vec![declaration()]).expect("snapshot");

        let commit = AllocationBootstrap::new(&mut store)
            .validate_and_commit(&codec, snapshot, &TestPolicy, Some(42))
            .expect("bootstrap commit");

        assert_eq!(commit.ledger.current_generation, 1);
        assert_eq!(commit.validated.generation(), 1);
        assert_eq!(commit.ledger.allocation_history.records.len(), 1);
        assert_eq!(commit.ledger.allocation_history.generations.len(), 1);
    }
}
