use crate::{
    declaration::AllocationDeclaration,
    declaration::DeclarationSnapshot,
    ledger::{
        AllocationLedger, AllocationReservationError, AllocationRetirement,
        AllocationRetirementError, LedgerCodec, LedgerCommitError, LedgerCommitStore,
    },
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
        self.validate_against(codec, prior, snapshot, policy, committed_at)
    }

    /// Initialize an empty ledger store explicitly, then validate and commit.
    ///
    /// This is the generic genesis path. The supplied `genesis` ledger is a
    /// framework decision; the generic crate only guarantees that it is used
    /// when the protected physical store is empty, never when recovery sees
    /// corrupt or partially written state.
    pub fn initialize_validate_and_commit<C, P>(
        &mut self,
        codec: &C,
        genesis: &AllocationLedger,
        snapshot: DeclarationSnapshot,
        policy: &P,
        committed_at: Option<u64>,
    ) -> Result<BootstrapCommit, BootstrapError<C::Error, P::Error>>
    where
        C: LedgerCodec,
        P: AllocationPolicy,
    {
        let prior = self
            .store
            .recover_or_initialize(codec, genesis)
            .map_err(BootstrapError::Ledger)?;
        self.validate_against(codec, prior, snapshot, policy, committed_at)
    }

    /// Recover, policy-check, reserve, and commit one reservation generation.
    pub fn reserve_and_commit<C, P>(
        &mut self,
        codec: &C,
        reservations: &[AllocationDeclaration],
        policy: &P,
        committed_at: Option<u64>,
    ) -> Result<AllocationLedger, BootstrapReservationError<C::Error, P::Error>>
    where
        C: LedgerCodec,
        P: AllocationPolicy,
    {
        let prior = self
            .store
            .recover(codec)
            .map_err(BootstrapReservationError::Ledger)?;
        self.reserve_against(codec, prior, reservations, policy, committed_at)
    }

    /// Initialize an empty ledger store, then reserve and commit.
    pub fn initialize_reserve_and_commit<C, P>(
        &mut self,
        codec: &C,
        genesis: &AllocationLedger,
        reservations: &[AllocationDeclaration],
        policy: &P,
        committed_at: Option<u64>,
    ) -> Result<AllocationLedger, BootstrapReservationError<C::Error, P::Error>>
    where
        C: LedgerCodec,
        P: AllocationPolicy,
    {
        let prior = self
            .store
            .recover_or_initialize(codec, genesis)
            .map_err(BootstrapReservationError::Ledger)?;
        self.reserve_against(codec, prior, reservations, policy, committed_at)
    }

    /// Recover, retire, and commit one explicit retirement generation.
    pub fn retire_and_commit<C>(
        &mut self,
        codec: &C,
        retirement: &AllocationRetirement,
        committed_at: Option<u64>,
    ) -> Result<AllocationLedger, BootstrapRetirementError<C::Error>>
    where
        C: LedgerCodec,
    {
        let prior = self
            .store
            .recover(codec)
            .map_err(BootstrapRetirementError::Ledger)?;
        self.retire_against(codec, prior, retirement, committed_at)
    }

    fn reserve_against<C, P>(
        &mut self,
        codec: &C,
        prior: AllocationLedger,
        reservations: &[AllocationDeclaration],
        policy: &P,
        committed_at: Option<u64>,
    ) -> Result<AllocationLedger, BootstrapReservationError<C::Error, P::Error>>
    where
        C: LedgerCodec,
        P: AllocationPolicy,
    {
        for reservation in reservations {
            policy
                .validate_key(&reservation.stable_key)
                .map_err(BootstrapReservationError::Policy)?;
            policy
                .validate_reserved_slot(&reservation.stable_key, &reservation.slot)
                .map_err(BootstrapReservationError::Policy)?;
        }

        let staged = prior
            .stage_reservation_generation(reservations, committed_at)
            .map_err(BootstrapReservationError::Reservation)?;
        self.store
            .commit(&staged, codec)
            .map_err(BootstrapReservationError::Ledger)
    }

    fn retire_against<C>(
        &mut self,
        codec: &C,
        prior: AllocationLedger,
        retirement: &AllocationRetirement,
        committed_at: Option<u64>,
    ) -> Result<AllocationLedger, BootstrapRetirementError<C::Error>>
    where
        C: LedgerCodec,
    {
        let staged = prior
            .stage_retirement_generation(retirement, committed_at)
            .map_err(BootstrapRetirementError::Retirement)?;
        self.store
            .commit(&staged, codec)
            .map_err(BootstrapRetirementError::Ledger)
    }

    fn validate_against<C, P>(
        &mut self,
        codec: &C,
        prior: AllocationLedger,
        snapshot: DeclarationSnapshot,
        policy: &P,
        committed_at: Option<u64>,
    ) -> Result<BootstrapCommit, BootstrapError<C::Error, P::Error>>
    where
        C: LedgerCodec,
        P: AllocationPolicy,
    {
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

///
/// BootstrapReservationError
///
/// Failure to policy-check, stage, or commit an allocation reservation.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum BootstrapReservationError<C, P> {
    /// Ledger recovery or protected commit failed.
    #[error(transparent)]
    Ledger(LedgerCommitError<C>),
    /// Policy adapter rejected a reservation declaration.
    #[error("allocation policy rejected a reservation")]
    Policy(P),
    /// Reservation conflicted with historical allocation facts.
    #[error(transparent)]
    Reservation(AllocationReservationError),
}

///
/// BootstrapRetirementError
///
/// Failure to stage or commit an explicit allocation retirement.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum BootstrapRetirementError<C> {
    /// Ledger recovery or protected commit failed.
    #[error(transparent)]
    Ledger(LedgerCommitError<C>),
    /// Retirement conflicted with historical allocation facts.
    #[error(transparent)]
    Retirement(AllocationRetirementError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        declaration::AllocationDeclaration,
        ledger::{AllocationHistory, AllocationLedger, AllocationState},
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

    #[derive(Debug, Eq, PartialEq)]
    struct RejectReservedPolicy;

    impl AllocationPolicy for RejectReservedPolicy {
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
            Err("reserved slot rejected")
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

    #[test]
    fn initialize_validate_and_commit_seeds_empty_ledger_store() {
        let codec = TestCodec::default();
        let mut store = LedgerCommitStore::default();
        let snapshot = DeclarationSnapshot::new(vec![declaration()]).expect("snapshot");

        let commit = AllocationBootstrap::new(&mut store)
            .initialize_validate_and_commit(&codec, &ledger(), snapshot, &TestPolicy, Some(42))
            .expect("bootstrap commit");

        assert_eq!(commit.ledger.current_generation, 1);
        assert_eq!(commit.validated.generation(), 1);
        assert_eq!(commit.ledger.allocation_history.records.len(), 1);
    }

    #[test]
    fn initialize_validate_and_commit_fails_closed_on_corrupt_store() {
        let codec = TestCodec::default();
        let mut store = LedgerCommitStore::default();
        store
            .write_corrupt_inactive_ledger(&ledger(), &codec)
            .expect("corrupt ledger");
        let snapshot = DeclarationSnapshot::new(vec![declaration()]).expect("snapshot");

        let err = AllocationBootstrap::new(&mut store)
            .initialize_validate_and_commit(&codec, &ledger(), snapshot, &TestPolicy, Some(42))
            .expect_err("corrupt state");

        assert!(matches!(err, BootstrapError::Ledger(_)));
    }

    #[test]
    fn reserve_and_commit_policy_checks_and_commits_reservation() {
        let codec = TestCodec::default();
        let mut store = LedgerCommitStore::default();
        store.commit(&ledger(), &codec).expect("initial ledger");
        let reservation = declaration();

        let committed = AllocationBootstrap::new(&mut store)
            .reserve_and_commit(&codec, &[reservation], &TestPolicy, Some(42))
            .expect("reservation commit");

        assert_eq!(committed.current_generation, 1);
        assert_eq!(committed.allocation_history.records.len(), 1);
        assert_eq!(
            committed.allocation_history.records[0].state,
            AllocationState::Reserved
        );
    }

    #[test]
    fn initialize_reserve_and_commit_seeds_empty_store() {
        let codec = TestCodec::default();
        let mut store = LedgerCommitStore::default();
        let reservation = declaration();

        let committed = AllocationBootstrap::new(&mut store)
            .initialize_reserve_and_commit(&codec, &ledger(), &[reservation], &TestPolicy, Some(42))
            .expect("reservation commit");

        assert_eq!(committed.current_generation, 1);
        assert_eq!(
            committed.allocation_history.records[0].state,
            AllocationState::Reserved
        );
    }

    #[test]
    fn reserve_and_commit_rejects_policy_failure_before_commit() {
        let codec = TestCodec::default();
        let mut store = LedgerCommitStore::default();
        store.commit(&ledger(), &codec).expect("initial ledger");
        let reservation = declaration();

        let err = AllocationBootstrap::new(&mut store)
            .reserve_and_commit(&codec, &[reservation], &RejectReservedPolicy, Some(42))
            .expect_err("policy failure");
        let recovered = store.recover(&codec).expect("recovered");

        assert!(matches!(err, BootstrapReservationError::Policy(_)));
        assert_eq!(recovered.current_generation, 0);
        assert!(recovered.allocation_history.records.is_empty());
    }

    #[test]
    fn retire_and_commit_tombstones_through_protected_commit() {
        let codec = TestCodec::default();
        let mut store = LedgerCommitStore::default();
        store.commit(&ledger(), &codec).expect("initial ledger");
        let snapshot = DeclarationSnapshot::new(vec![declaration()]).expect("snapshot");
        AllocationBootstrap::new(&mut store)
            .validate_and_commit(&codec, snapshot, &TestPolicy, Some(42))
            .expect("active commit");
        let retirement = AllocationRetirement::new(
            "app.users.v1",
            AllocationSlotDescriptor::memory_manager(100),
        )
        .expect("retirement");

        let committed = AllocationBootstrap::new(&mut store)
            .retire_and_commit(&codec, &retirement, Some(43))
            .expect("retirement commit");

        assert_eq!(committed.current_generation, 2);
        assert_eq!(
            committed.allocation_history.records[0].state,
            AllocationState::Retired
        );
        assert_eq!(
            committed.allocation_history.records[0].retired_generation,
            Some(2)
        );
    }

    #[test]
    fn retire_and_commit_rejects_unknown_key_before_commit() {
        let codec = TestCodec::default();
        let mut store = LedgerCommitStore::default();
        store.commit(&ledger(), &codec).expect("initial ledger");
        let retirement = AllocationRetirement::new(
            "app.users.v1",
            AllocationSlotDescriptor::memory_manager(100),
        )
        .expect("retirement");

        let err = AllocationBootstrap::new(&mut store)
            .retire_and_commit(&codec, &retirement, Some(43))
            .expect_err("unknown key");
        let recovered = store.recover(&codec).expect("recovered");

        assert!(matches!(err, BootstrapRetirementError::Retirement(_)));
        assert_eq!(recovered.current_generation, 0);
        assert!(recovered.allocation_history.records.is_empty());
    }
}
