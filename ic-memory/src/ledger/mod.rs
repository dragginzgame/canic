use crate::{
    declaration::AllocationDeclaration,
    key::{StableKey, StableKeyError},
    physical::{CommitRecoveryError, DualCommitStore},
    schema::SchemaMetadata,
    session::ValidatedAllocations,
    slot::AllocationSlotDescriptor,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Current allocation ledger schema version.
pub const CURRENT_LEDGER_SCHEMA_VERSION: u32 = 1;

/// Current protected physical ledger format identifier.
pub const CURRENT_PHYSICAL_FORMAT_ID: u32 = 1;

///
/// AllocationLedger
///
/// Durable root of allocation history.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AllocationLedger {
    /// Ledger schema version.
    pub ledger_schema_version: u32,
    /// Physical encoding format identifier.
    pub physical_format_id: u32,
    /// Current committed generation selected by recovery.
    pub current_generation: u64,
    /// Historical allocation facts.
    pub allocation_history: AllocationHistory,
}

///
/// AllocationHistory
///
/// Durable allocation records and generation history.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AllocationHistory {
    /// Stable-key allocation records.
    pub records: Vec<AllocationRecord>,
    /// Committed generation records.
    pub generations: Vec<GenerationRecord>,
}

///
/// AllocationRecord
///
/// Durable ownership record for one stable key.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AllocationRecord {
    /// Stable key that owns the slot.
    pub stable_key: StableKey,
    /// Durable allocation slot owned by the key.
    pub slot: AllocationSlotDescriptor,
    /// Current allocation lifecycle state.
    pub state: AllocationState,
    /// First committed generation that recorded this allocation.
    pub first_generation: u64,
    /// Latest committed generation that observed this allocation declaration.
    pub last_seen_generation: u64,
    /// Generation that explicitly retired this allocation.
    pub retired_generation: Option<u64>,
    /// Per-generation schema metadata history.
    pub schema_history: Vec<SchemaMetadataRecord>,
}

///
/// AllocationRetirement
///
/// Explicit request to tombstone one historical allocation identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AllocationRetirement {
    /// Stable key being retired.
    pub stable_key: StableKey,
    /// Allocation slot historically owned by the stable key.
    pub slot: AllocationSlotDescriptor,
}

impl AllocationRetirement {
    /// Build an explicit retirement request from raw parts.
    pub fn new(
        stable_key: impl AsRef<str>,
        slot: AllocationSlotDescriptor,
    ) -> Result<Self, AllocationRetirementError> {
        Ok(Self {
            stable_key: StableKey::parse(stable_key).map_err(AllocationRetirementError::Key)?,
            slot,
        })
    }
}

///
/// AllocationState
///
/// Allocation lifecycle state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AllocationState {
    /// Slot is reserved for a future allocation identity.
    Reserved,
    /// Slot is active and may be opened after validation.
    Active,
    /// Slot was explicitly retired and remains tombstoned.
    Retired,
}

///
/// SchemaMetadataRecord
///
/// Schema metadata observed in one committed generation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchemaMetadataRecord {
    /// Generation that declared this schema metadata.
    pub generation: u64,
    /// Schema metadata declared by that generation.
    pub schema: SchemaMetadata,
}

///
/// GenerationRecord
///
/// Diagnostic metadata for one committed ledger generation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GenerationRecord {
    /// Committed generation number.
    pub generation: u64,
    /// Parent generation, if recorded.
    pub parent_generation: Option<u64>,
    /// Optional binary/runtime fingerprint.
    pub runtime_fingerprint: Option<String>,
    /// Number of declarations in the generation.
    pub declaration_count: u32,
    /// Optional commit timestamp supplied by the integration layer.
    pub committed_at: Option<u64>,
}

///
/// LedgerCompatibility
///
/// Supported logical and physical ledger format versions.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LedgerCompatibility {
    /// Minimum supported ledger schema version.
    pub min_ledger_schema_version: u32,
    /// Maximum supported ledger schema version.
    pub max_ledger_schema_version: u32,
    /// Required physical encoding format identifier.
    pub physical_format_id: u32,
}

impl LedgerCompatibility {
    /// Return the compatibility supported by this crate version.
    #[must_use]
    pub const fn current() -> Self {
        Self {
            min_ledger_schema_version: CURRENT_LEDGER_SCHEMA_VERSION,
            max_ledger_schema_version: CURRENT_LEDGER_SCHEMA_VERSION,
            physical_format_id: CURRENT_PHYSICAL_FORMAT_ID,
        }
    }

    /// Validate a decoded ledger before it is used as authoritative state.
    pub const fn validate(
        &self,
        ledger: &AllocationLedger,
    ) -> Result<(), LedgerCompatibilityError> {
        if ledger.ledger_schema_version < self.min_ledger_schema_version {
            return Err(LedgerCompatibilityError::UnsupportedLedgerSchemaVersion {
                found: ledger.ledger_schema_version,
                min_supported: self.min_ledger_schema_version,
                max_supported: self.max_ledger_schema_version,
            });
        }
        if ledger.ledger_schema_version > self.max_ledger_schema_version {
            return Err(LedgerCompatibilityError::UnsupportedLedgerSchemaVersion {
                found: ledger.ledger_schema_version,
                min_supported: self.min_ledger_schema_version,
                max_supported: self.max_ledger_schema_version,
            });
        }
        if ledger.physical_format_id != self.physical_format_id {
            return Err(LedgerCompatibilityError::UnsupportedPhysicalFormat {
                found: ledger.physical_format_id,
                supported: self.physical_format_id,
            });
        }
        Ok(())
    }
}

impl Default for LedgerCompatibility {
    fn default() -> Self {
        Self::current()
    }
}

///
/// LedgerCompatibilityError
///
/// Decoded ledger format is unsupported by this reader.
#[derive(Clone, Copy, Debug, Eq, thiserror::Error, PartialEq)]
pub enum LedgerCompatibilityError {
    /// Ledger schema version is outside the supported range.
    #[error(
        "ledger_schema_version {found} is unsupported; supported range is {min_supported}-{max_supported}"
    )]
    UnsupportedLedgerSchemaVersion {
        /// Version found in the ledger.
        found: u32,
        /// Minimum supported version.
        min_supported: u32,
        /// Maximum supported version.
        max_supported: u32,
    },
    /// Physical format ID is not supported by this reader.
    #[error("physical_format_id {found} is unsupported; supported format is {supported}")]
    UnsupportedPhysicalFormat {
        /// Format found in the ledger.
        found: u32,
        /// Supported format ID.
        supported: u32,
    },
}

///
/// LedgerIntegrityError
///
/// Decoded ledger violates structural allocation-history invariants.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum LedgerIntegrityError {
    /// Stable key appears in more than one allocation record.
    #[error("stable key '{stable_key}' appears in more than one allocation record")]
    DuplicateStableKey {
        /// Duplicate stable key.
        stable_key: StableKey,
    },
    /// Allocation slot appears in more than one allocation record.
    #[error("allocation slot '{slot:?}' appears in more than one allocation record")]
    DuplicateSlot {
        /// Duplicate allocation slot.
        slot: Box<AllocationSlotDescriptor>,
    },
    /// Allocation record generation ordering is invalid.
    #[error("stable key '{stable_key}' has first_generation after last_seen_generation")]
    InvalidRecordGenerationOrder {
        /// Stable key whose record is invalid.
        stable_key: StableKey,
        /// First generation in the record.
        first_generation: u64,
        /// Last seen generation in the record.
        last_seen_generation: u64,
    },
    /// Allocation record points past the current generation.
    #[error(
        "stable key '{stable_key}' references generation {generation} after current generation {current_generation}"
    )]
    FutureRecordGeneration {
        /// Stable key whose record is invalid.
        stable_key: StableKey,
        /// Generation referenced by the record.
        generation: u64,
        /// Current ledger generation.
        current_generation: u64,
    },
    /// Non-retired allocation carries retired metadata.
    #[error("stable key '{stable_key}' is not retired but has retired_generation metadata")]
    UnexpectedRetiredGeneration {
        /// Stable key whose record is invalid.
        stable_key: StableKey,
    },
    /// Retired allocation is missing retired metadata.
    #[error("stable key '{stable_key}' is retired but retired_generation is missing")]
    MissingRetiredGeneration {
        /// Stable key whose record is invalid.
        stable_key: StableKey,
    },
    /// Retired generation predates the allocation record.
    #[error("stable key '{stable_key}' has retired_generation before first_generation")]
    RetiredBeforeFirstGeneration {
        /// Stable key whose record is invalid.
        stable_key: StableKey,
        /// First generation in the record.
        first_generation: u64,
        /// Retired generation in the record.
        retired_generation: u64,
    },
    /// Allocation record has no schema metadata history.
    #[error("stable key '{stable_key}' has empty schema metadata history")]
    EmptySchemaHistory {
        /// Stable key whose record is invalid.
        stable_key: StableKey,
    },
    /// Schema metadata generation history is not strictly increasing.
    #[error("stable key '{stable_key}' has non-increasing schema metadata generation history")]
    NonIncreasingSchemaHistory {
        /// Stable key whose record is invalid.
        stable_key: StableKey,
    },
    /// Schema metadata generation is outside the allocation record lifetime.
    #[error("stable key '{stable_key}' has schema metadata generation outside the ledger bounds")]
    SchemaHistoryOutOfBounds {
        /// Stable key whose record is invalid.
        stable_key: StableKey,
        /// Schema metadata generation.
        generation: u64,
    },
    /// Generation record appears more than once.
    #[error("generation {generation} appears more than once")]
    DuplicateGeneration {
        /// Duplicate generation.
        generation: u64,
    },
    /// Generation record points past the current generation.
    #[error("generation {generation} is after current generation {current_generation}")]
    FutureGeneration {
        /// Generation record value.
        generation: u64,
        /// Current ledger generation.
        current_generation: u64,
    },
    /// Generation parent does not precede the child generation.
    #[error("generation {generation} has invalid parent generation {parent_generation:?}")]
    InvalidParentGeneration {
        /// Generation record value.
        generation: u64,
        /// Invalid parent generation.
        parent_generation: Option<u64>,
    },
}

///
/// LedgerCodec
///
/// Integration-supplied encoding for persisted allocation ledgers.
pub trait LedgerCodec {
    /// Encoding or decoding error type.
    type Error;

    /// Encode a logical allocation ledger into durable bytes.
    fn encode(&self, ledger: &AllocationLedger) -> Result<Vec<u8>, Self::Error>;

    /// Decode durable bytes into a logical allocation ledger.
    fn decode(&self, bytes: &[u8]) -> Result<AllocationLedger, Self::Error>;
}

///
/// LedgerCommitStore
///
/// Generation-scoped allocation ledger commit store.
///
/// This type owns the generic commit lifecycle. It deliberately does not own
/// serialization or stable-memory IO; those remain substrate/integration
/// responsibilities.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct LedgerCommitStore {
    /// Protected physical commit slots.
    pub physical: DualCommitStore,
}

impl LedgerCommitStore {
    /// Recover the authoritative allocation ledger using `codec`.
    pub fn recover<C: LedgerCodec>(
        &self,
        codec: &C,
    ) -> Result<AllocationLedger, LedgerCommitError<C::Error>> {
        self.recover_with_compatibility(codec, LedgerCompatibility::current())
    }

    /// Recover the authoritative allocation ledger using explicit compatibility rules.
    pub fn recover_with_compatibility<C: LedgerCodec>(
        &self,
        codec: &C,
        compatibility: LedgerCompatibility,
    ) -> Result<AllocationLedger, LedgerCommitError<C::Error>> {
        let committed = self
            .physical
            .authoritative()
            .map_err(LedgerCommitError::Recovery)?;
        let ledger = codec
            .decode(&committed.payload)
            .map_err(LedgerCommitError::Codec)?;
        compatibility
            .validate(&ledger)
            .map_err(LedgerCommitError::Compatibility)?;
        ledger
            .validate_integrity()
            .map_err(LedgerCommitError::Integrity)?;
        Ok(ledger)
    }

    /// Recover the authoritative ledger, or explicitly initialize an empty store.
    ///
    /// Initialization is allowed only when no physical commit slot has ever
    /// been written. Corrupt or partially written stores fail closed even when
    /// a genesis ledger is supplied.
    pub fn recover_or_initialize<C: LedgerCodec>(
        &mut self,
        codec: &C,
        genesis: &AllocationLedger,
    ) -> Result<AllocationLedger, LedgerCommitError<C::Error>> {
        self.recover_or_initialize_with_compatibility(
            codec,
            genesis,
            LedgerCompatibility::current(),
        )
    }

    /// Recover the authoritative ledger, or initialize an empty store with explicit compatibility.
    pub fn recover_or_initialize_with_compatibility<C: LedgerCodec>(
        &mut self,
        codec: &C,
        genesis: &AllocationLedger,
        compatibility: LedgerCompatibility,
    ) -> Result<AllocationLedger, LedgerCommitError<C::Error>> {
        match self.recover_with_compatibility(codec, compatibility) {
            Ok(ledger) => Ok(ledger),
            Err(LedgerCommitError::Recovery(CommitRecoveryError::NoValidGeneration))
                if self.physical.is_uninitialized() =>
            {
                self.commit_with_compatibility(genesis, codec, compatibility)
            }
            Err(err) => Err(err),
        }
    }

    /// Commit one logical allocation ledger generation through `codec`.
    pub fn commit<C: LedgerCodec>(
        &mut self,
        ledger: &AllocationLedger,
        codec: &C,
    ) -> Result<AllocationLedger, LedgerCommitError<C::Error>> {
        self.commit_with_compatibility(ledger, codec, LedgerCompatibility::current())
    }

    /// Commit one logical allocation ledger generation through explicit compatibility.
    pub fn commit_with_compatibility<C: LedgerCodec>(
        &mut self,
        ledger: &AllocationLedger,
        codec: &C,
        compatibility: LedgerCompatibility,
    ) -> Result<AllocationLedger, LedgerCommitError<C::Error>> {
        compatibility
            .validate(ledger)
            .map_err(LedgerCommitError::Compatibility)?;
        ledger
            .validate_integrity()
            .map_err(LedgerCommitError::Integrity)?;
        let payload = codec.encode(ledger).map_err(LedgerCommitError::Codec)?;
        self.physical
            .commit_payload(payload)
            .map_err(LedgerCommitError::Recovery)?;
        self.recover_with_compatibility(codec, compatibility)
    }

    /// Simulate a torn write of a logical ledger payload into the inactive slot.
    pub fn write_corrupt_inactive_ledger<C: LedgerCodec>(
        &mut self,
        ledger: &AllocationLedger,
        codec: &C,
    ) -> Result<(), LedgerCommitError<C::Error>> {
        let payload = codec.encode(ledger).map_err(LedgerCommitError::Codec)?;
        self.physical
            .write_corrupt_inactive_slot(ledger.current_generation, payload);
        Ok(())
    }
}

///
/// LedgerCommitError
///
/// Failure to recover or commit a logical allocation ledger.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum LedgerCommitError<E> {
    /// Protected physical commit recovery failed.
    #[error(transparent)]
    Recovery(CommitRecoveryError),
    /// Integration-supplied codec failed.
    #[error("allocation ledger codec failed")]
    Codec(E),
    /// Decoded ledger format is not compatible with this reader.
    #[error(transparent)]
    Compatibility(LedgerCompatibilityError),
    /// Decoded ledger violates structural allocation-history invariants.
    #[error(transparent)]
    Integrity(LedgerIntegrityError),
}

///
/// AllocationReservationError
///
/// Failure to stage a reservation generation.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum AllocationReservationError {
    /// Stable key was historically bound to a different slot.
    #[error("stable key '{stable_key}' was historically bound to a different allocation slot")]
    StableKeySlotConflict {
        /// Stable key being reserved.
        stable_key: StableKey,
        /// Historical slot for the stable key.
        historical_slot: Box<AllocationSlotDescriptor>,
        /// Slot claimed by the reservation.
        reserved_slot: Box<AllocationSlotDescriptor>,
    },
    /// Slot was historically bound to a different stable key.
    #[error("allocation slot '{slot:?}' was historically bound to stable key '{historical_key}'")]
    SlotStableKeyConflict {
        /// Slot being reserved.
        slot: Box<AllocationSlotDescriptor>,
        /// Historical stable key for the slot.
        historical_key: StableKey,
        /// Stable key claimed by the reservation.
        reserved_key: StableKey,
    },
    /// Allocation already exists as an active record.
    #[error("stable key '{stable_key}' is already active and cannot be reserved")]
    ActiveAllocation {
        /// Active stable key.
        stable_key: StableKey,
        /// Active allocation slot.
        slot: Box<AllocationSlotDescriptor>,
    },
    /// Allocation was already retired and cannot be reserved.
    #[error("stable key '{stable_key}' was explicitly retired and cannot be reserved")]
    RetiredAllocation {
        /// Retired stable key.
        stable_key: StableKey,
        /// Retired allocation slot.
        slot: Box<AllocationSlotDescriptor>,
    },
}

///
/// AllocationRetirementError
///
/// Failure to stage an explicit retirement generation.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum AllocationRetirementError {
    /// Stable-key grammar failure.
    #[error(transparent)]
    Key(StableKeyError),
    /// Stable key has no historical allocation record.
    #[error("stable key '{0}' has no allocation record to retire")]
    UnknownStableKey(StableKey),
    /// Stable key was historically bound to a different slot.
    #[error("stable key '{stable_key}' cannot be retired for a different allocation slot")]
    SlotMismatch {
        /// Stable key being retired.
        stable_key: StableKey,
        /// Historical slot for the stable key.
        historical_slot: Box<AllocationSlotDescriptor>,
        /// Slot named by the retirement request.
        retired_slot: Box<AllocationSlotDescriptor>,
    },
    /// Allocation was already retired.
    #[error("stable key '{stable_key}' was already retired")]
    AlreadyRetired {
        /// Retired stable key.
        stable_key: StableKey,
        /// Retired allocation slot.
        slot: Box<AllocationSlotDescriptor>,
    },
}

impl AllocationRecord {
    /// Create a new allocation record from a declaration.
    #[must_use]
    pub fn from_declaration(
        generation: u64,
        declaration: AllocationDeclaration,
        state: AllocationState,
    ) -> Self {
        Self {
            stable_key: declaration.stable_key,
            slot: declaration.slot,
            state,
            first_generation: generation,
            last_seen_generation: generation,
            retired_generation: None,
            schema_history: vec![SchemaMetadataRecord {
                generation,
                schema: declaration.schema,
            }],
        }
    }

    /// Create a new reserved allocation record from a declaration.
    #[must_use]
    pub fn reserved(generation: u64, declaration: AllocationDeclaration) -> Self {
        Self::from_declaration(generation, declaration, AllocationState::Reserved)
    }

    fn observe_declaration(&mut self, generation: u64, declaration: &AllocationDeclaration) {
        self.last_seen_generation = generation;
        if self.state == AllocationState::Reserved {
            self.state = AllocationState::Active;
        }

        let latest_schema = self.schema_history.last().map(|record| &record.schema);
        if latest_schema != Some(&declaration.schema) {
            self.schema_history.push(SchemaMetadataRecord {
                generation,
                schema: declaration.schema.clone(),
            });
        }
    }

    fn observe_reservation(&mut self, generation: u64, reservation: &AllocationDeclaration) {
        self.last_seen_generation = generation;

        let latest_schema = self.schema_history.last().map(|record| &record.schema);
        if latest_schema != Some(&reservation.schema) {
            self.schema_history.push(SchemaMetadataRecord {
                generation,
                schema: reservation.schema.clone(),
            });
        }
    }
}

impl AllocationLedger {
    /// Validate structural ledger invariants before recovery or commit.
    pub fn validate_integrity(&self) -> Result<(), LedgerIntegrityError> {
        let mut stable_keys = BTreeSet::new();
        let mut slots = BTreeSet::new();

        for record in &self.allocation_history.records {
            if !stable_keys.insert(record.stable_key.clone()) {
                return Err(LedgerIntegrityError::DuplicateStableKey {
                    stable_key: record.stable_key.clone(),
                });
            }
            if !slots.insert(record.slot.clone()) {
                return Err(LedgerIntegrityError::DuplicateSlot {
                    slot: Box::new(record.slot.clone()),
                });
            }
            validate_record_integrity(self.current_generation, record)?;
        }

        let mut generations = BTreeSet::new();
        for generation in &self.allocation_history.generations {
            if !generations.insert(generation.generation) {
                return Err(LedgerIntegrityError::DuplicateGeneration {
                    generation: generation.generation,
                });
            }
            if generation.generation > self.current_generation {
                return Err(LedgerIntegrityError::FutureGeneration {
                    generation: generation.generation,
                    current_generation: self.current_generation,
                });
            }
            if generation
                .parent_generation
                .is_some_and(|parent| parent >= generation.generation)
            {
                return Err(LedgerIntegrityError::InvalidParentGeneration {
                    generation: generation.generation,
                    parent_generation: generation.parent_generation,
                });
            }
        }

        Ok(())
    }

    /// Return a copy of the ledger with `validated` recorded as the next generation.
    ///
    /// This is a pure logical update. Physical atomicity is the responsibility of
    /// the substrate commit protocol.
    #[must_use]
    pub fn stage_validated_generation(
        &self,
        validated: &ValidatedAllocations,
        committed_at: Option<u64>,
    ) -> Self {
        let next_generation = self.current_generation.saturating_add(1);
        let mut next = self.clone();
        next.current_generation = next_generation;
        let declaration_count = u32::try_from(validated.declarations().len()).unwrap_or(u32::MAX);

        for declaration in validated.declarations() {
            record_declaration(&mut next, next_generation, declaration);
        }

        next.allocation_history.generations.push(GenerationRecord {
            generation: next_generation,
            parent_generation: Some(self.current_generation),
            runtime_fingerprint: validated.runtime_fingerprint().map(str::to_string),
            declaration_count,
            committed_at,
        });

        next
    }

    /// Return a copy of the ledger with `reservations` recorded as the next generation.
    ///
    /// This is a pure logical update. The caller is responsible for applying
    /// framework policy before staging reservations.
    pub fn stage_reservation_generation(
        &self,
        reservations: &[AllocationDeclaration],
        committed_at: Option<u64>,
    ) -> Result<Self, AllocationReservationError> {
        let next_generation = self.current_generation.saturating_add(1);
        let mut next = self.clone();
        next.current_generation = next_generation;

        for reservation in reservations {
            record_reservation(&mut next, next_generation, reservation)?;
        }

        next.allocation_history.generations.push(GenerationRecord {
            generation: next_generation,
            parent_generation: Some(self.current_generation),
            runtime_fingerprint: None,
            declaration_count: u32::try_from(reservations.len()).unwrap_or(u32::MAX),
            committed_at,
        });

        Ok(next)
    }

    /// Return a copy of the ledger with one explicit retirement committed.
    pub fn stage_retirement_generation(
        &self,
        retirement: &AllocationRetirement,
        committed_at: Option<u64>,
    ) -> Result<Self, AllocationRetirementError> {
        let next_generation = self.current_generation.saturating_add(1);
        let mut next = self.clone();
        let record = next
            .allocation_history
            .records
            .iter_mut()
            .find(|record| record.stable_key == retirement.stable_key)
            .ok_or_else(|| {
                AllocationRetirementError::UnknownStableKey(retirement.stable_key.clone())
            })?;

        if record.slot != retirement.slot {
            return Err(AllocationRetirementError::SlotMismatch {
                stable_key: retirement.stable_key.clone(),
                historical_slot: Box::new(record.slot.clone()),
                retired_slot: Box::new(retirement.slot.clone()),
            });
        }
        if record.state == AllocationState::Retired {
            return Err(AllocationRetirementError::AlreadyRetired {
                stable_key: retirement.stable_key.clone(),
                slot: Box::new(record.slot.clone()),
            });
        }

        record.state = AllocationState::Retired;
        record.retired_generation = Some(next_generation);
        next.current_generation = next_generation;
        next.allocation_history.generations.push(GenerationRecord {
            generation: next_generation,
            parent_generation: Some(self.current_generation),
            runtime_fingerprint: None,
            declaration_count: 0,
            committed_at,
        });

        Ok(next)
    }
}

fn record_declaration(
    ledger: &mut AllocationLedger,
    generation: u64,
    declaration: &AllocationDeclaration,
) {
    if let Some(record) = ledger
        .allocation_history
        .records
        .iter_mut()
        .find(|record| record.stable_key == declaration.stable_key)
    {
        record.observe_declaration(generation, declaration);
        return;
    }

    ledger
        .allocation_history
        .records
        .push(AllocationRecord::from_declaration(
            generation,
            declaration.clone(),
            AllocationState::Active,
        ));
}

fn record_reservation(
    ledger: &mut AllocationLedger,
    generation: u64,
    reservation: &AllocationDeclaration,
) -> Result<(), AllocationReservationError> {
    if let Some(record) = ledger
        .allocation_history
        .records
        .iter_mut()
        .find(|record| record.stable_key == reservation.stable_key)
    {
        if record.slot != reservation.slot {
            return Err(AllocationReservationError::StableKeySlotConflict {
                stable_key: reservation.stable_key.clone(),
                historical_slot: Box::new(record.slot.clone()),
                reserved_slot: Box::new(reservation.slot.clone()),
            });
        }

        return match record.state {
            AllocationState::Reserved => {
                record.observe_reservation(generation, reservation);
                Ok(())
            }
            AllocationState::Active => Err(AllocationReservationError::ActiveAllocation {
                stable_key: reservation.stable_key.clone(),
                slot: Box::new(record.slot.clone()),
            }),
            AllocationState::Retired => Err(AllocationReservationError::RetiredAllocation {
                stable_key: reservation.stable_key.clone(),
                slot: Box::new(record.slot.clone()),
            }),
        };
    }

    if let Some(record) = ledger
        .allocation_history
        .records
        .iter()
        .find(|record| record.slot == reservation.slot)
    {
        return Err(AllocationReservationError::SlotStableKeyConflict {
            slot: Box::new(reservation.slot.clone()),
            historical_key: record.stable_key.clone(),
            reserved_key: reservation.stable_key.clone(),
        });
    }

    ledger
        .allocation_history
        .records
        .push(AllocationRecord::reserved(generation, reservation.clone()));
    Ok(())
}

fn validate_record_integrity(
    current_generation: u64,
    record: &AllocationRecord,
) -> Result<(), LedgerIntegrityError> {
    if record.first_generation > record.last_seen_generation {
        return Err(LedgerIntegrityError::InvalidRecordGenerationOrder {
            stable_key: record.stable_key.clone(),
            first_generation: record.first_generation,
            last_seen_generation: record.last_seen_generation,
        });
    }
    if record.last_seen_generation > current_generation {
        return Err(LedgerIntegrityError::FutureRecordGeneration {
            stable_key: record.stable_key.clone(),
            generation: record.last_seen_generation,
            current_generation,
        });
    }

    match (record.state, record.retired_generation) {
        (AllocationState::Retired, Some(retired_generation)) => {
            if retired_generation < record.first_generation {
                return Err(LedgerIntegrityError::RetiredBeforeFirstGeneration {
                    stable_key: record.stable_key.clone(),
                    first_generation: record.first_generation,
                    retired_generation,
                });
            }
            if retired_generation > current_generation {
                return Err(LedgerIntegrityError::FutureRecordGeneration {
                    stable_key: record.stable_key.clone(),
                    generation: retired_generation,
                    current_generation,
                });
            }
        }
        (AllocationState::Retired, None) => {
            return Err(LedgerIntegrityError::MissingRetiredGeneration {
                stable_key: record.stable_key.clone(),
            });
        }
        (AllocationState::Reserved | AllocationState::Active, Some(_)) => {
            return Err(LedgerIntegrityError::UnexpectedRetiredGeneration {
                stable_key: record.stable_key.clone(),
            });
        }
        (AllocationState::Reserved | AllocationState::Active, None) => {}
    }

    validate_schema_history_integrity(current_generation, record)
}

fn validate_schema_history_integrity(
    current_generation: u64,
    record: &AllocationRecord,
) -> Result<(), LedgerIntegrityError> {
    if record.schema_history.is_empty() {
        return Err(LedgerIntegrityError::EmptySchemaHistory {
            stable_key: record.stable_key.clone(),
        });
    }

    let mut previous = None;
    for schema in &record.schema_history {
        if previous.is_some_and(|generation| schema.generation <= generation) {
            return Err(LedgerIntegrityError::NonIncreasingSchemaHistory {
                stable_key: record.stable_key.clone(),
            });
        }
        if schema.generation < record.first_generation || schema.generation > current_generation {
            return Err(LedgerIntegrityError::SchemaHistoryOutOfBounds {
                stable_key: record.stable_key.clone(),
                generation: schema.generation,
            });
        }
        previous = Some(schema.generation);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{declaration::DeclarationSnapshot, schema::SchemaMetadata};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct TestCodec;

    impl LedgerCodec for TestCodec {
        type Error = &'static str;

        fn encode(&self, ledger: &AllocationLedger) -> Result<Vec<u8>, Self::Error> {
            let mut bytes = Vec::with_capacity(16);
            bytes.extend_from_slice(&ledger.ledger_schema_version.to_le_bytes());
            bytes.extend_from_slice(&ledger.physical_format_id.to_le_bytes());
            bytes.extend_from_slice(&ledger.current_generation.to_le_bytes());
            Ok(bytes)
        }

        fn decode(&self, bytes: &[u8]) -> Result<AllocationLedger, Self::Error> {
            let bytes = <[u8; 16]>::try_from(bytes).map_err(|_| "invalid ledger")?;
            let ledger_schema_version =
                u32::from_le_bytes(bytes[0..4].try_into().map_err(|_| "invalid schema")?);
            let physical_format_id =
                u32::from_le_bytes(bytes[4..8].try_into().map_err(|_| "invalid format")?);
            let current_generation =
                u64::from_le_bytes(bytes[8..16].try_into().map_err(|_| "invalid generation")?);
            Ok(AllocationLedger {
                ledger_schema_version,
                physical_format_id,
                current_generation,
                ..ledger()
            })
        }
    }

    fn declaration(key: &str, id: u8, schema_version: Option<u32>) -> AllocationDeclaration {
        AllocationDeclaration::new(
            key,
            AllocationSlotDescriptor::memory_manager(id),
            None,
            SchemaMetadata {
                schema_version,
                schema_fingerprint: None,
            },
        )
        .expect("declaration")
    }

    fn ledger() -> AllocationLedger {
        AllocationLedger {
            ledger_schema_version: CURRENT_LEDGER_SCHEMA_VERSION,
            physical_format_id: CURRENT_PHYSICAL_FORMAT_ID,
            current_generation: 3,
            allocation_history: AllocationHistory::default(),
        }
    }

    fn active_record(key: &str, id: u8) -> AllocationRecord {
        AllocationRecord::from_declaration(1, declaration(key, id, None), AllocationState::Active)
    }

    fn validated(
        generation: u64,
        declarations: Vec<AllocationDeclaration>,
    ) -> crate::session::ValidatedAllocations {
        crate::session::ValidatedAllocations::new(generation, declarations, None)
    }

    #[test]
    fn stage_validated_generation_records_new_allocations() {
        let declarations = vec![declaration("app.users.v1", 100, Some(1))];
        let validated = validated(3, declarations);

        let staged = ledger().stage_validated_generation(&validated, Some(42));

        assert_eq!(staged.current_generation, 4);
        assert_eq!(staged.allocation_history.records.len(), 1);
        assert_eq!(staged.allocation_history.records[0].first_generation, 4);
        assert_eq!(staged.allocation_history.generations[0].generation, 4);
        assert_eq!(
            staged.allocation_history.generations[0].committed_at,
            Some(42)
        );
    }

    #[test]
    fn stage_validated_generation_preserves_omitted_records() {
        let first = validated(
            3,
            vec![
                declaration("app.users.v1", 100, Some(1)),
                declaration("app.orders.v1", 101, Some(1)),
            ],
        );
        let second = validated(4, vec![declaration("app.users.v1", 100, Some(1))]);

        let staged = ledger().stage_validated_generation(&first, None);
        let staged = staged.stage_validated_generation(&second, None);

        assert_eq!(staged.current_generation, 5);
        assert_eq!(staged.allocation_history.records.len(), 2);
        let omitted = staged
            .allocation_history
            .records
            .iter()
            .find(|record| record.stable_key.as_str() == "app.orders.v1")
            .expect("omitted record");
        assert_eq!(omitted.state, AllocationState::Active);
        assert_eq!(omitted.last_seen_generation, 4);
    }

    #[test]
    fn stage_validated_generation_records_schema_metadata_history() {
        let first = validated(3, vec![declaration("app.users.v1", 100, Some(1))]);
        let second = validated(4, vec![declaration("app.users.v1", 100, Some(2))]);

        let staged = ledger().stage_validated_generation(&first, None);
        let staged = staged.stage_validated_generation(&second, None);
        let record = &staged.allocation_history.records[0];

        assert_eq!(record.schema_history.len(), 2);
        assert_eq!(record.schema_history[0].generation, 4);
        assert_eq!(record.schema_history[1].generation, 5);
    }

    #[test]
    fn stage_reservation_generation_records_reserved_allocations() {
        let reservations = vec![declaration("ic_memory.generation_log.v1", 1, None)];

        let staged = ledger()
            .stage_reservation_generation(&reservations, Some(42))
            .expect("reserved generation");

        assert_eq!(staged.current_generation, 4);
        assert_eq!(staged.allocation_history.records.len(), 1);
        assert_eq!(
            staged.allocation_history.records[0].state,
            AllocationState::Reserved
        );
        assert_eq!(
            staged.allocation_history.generations[0].declaration_count,
            1
        );
    }

    #[test]
    fn stage_reservation_generation_rejects_active_allocation() {
        let active = validated(3, vec![declaration("app.users.v1", 100, None)]);
        let staged = ledger().stage_validated_generation(&active, None);
        let reservations = vec![declaration("app.users.v1", 100, None)];

        let err = staged
            .stage_reservation_generation(&reservations, None)
            .expect_err("active cannot become reserved");

        assert!(matches!(
            err,
            AllocationReservationError::ActiveAllocation { .. }
        ));
    }

    #[test]
    fn stage_validated_generation_activates_reserved_record() {
        let reservations = vec![declaration("app.future_store.v1", 100, Some(1))];
        let staged = ledger()
            .stage_reservation_generation(&reservations, None)
            .expect("reserved generation");
        let active = validated(4, vec![declaration("app.future_store.v1", 100, Some(2))]);

        let staged = staged.stage_validated_generation(&active, None);
        let record = &staged.allocation_history.records[0];

        assert_eq!(record.state, AllocationState::Active);
        assert_eq!(record.first_generation, 4);
        assert_eq!(record.last_seen_generation, 5);
        assert_eq!(record.schema_history.len(), 2);
    }

    #[test]
    fn stage_retirement_generation_tombstones_named_allocation() {
        let active = validated(3, vec![declaration("app.users.v1", 100, None)]);
        let staged = ledger().stage_validated_generation(&active, None);
        let retirement = AllocationRetirement::new(
            "app.users.v1",
            AllocationSlotDescriptor::memory_manager(100),
        )
        .expect("retirement");

        let staged = staged
            .stage_retirement_generation(&retirement, Some(42))
            .expect("retired generation");
        let record = &staged.allocation_history.records[0];

        assert_eq!(staged.current_generation, 5);
        assert_eq!(record.state, AllocationState::Retired);
        assert_eq!(record.retired_generation, Some(5));
        assert_eq!(
            staged.allocation_history.generations[1].declaration_count,
            0
        );
    }

    #[test]
    fn stage_retirement_generation_requires_matching_slot() {
        let active = validated(3, vec![declaration("app.users.v1", 100, None)]);
        let staged = ledger().stage_validated_generation(&active, None);
        let retirement = AllocationRetirement::new(
            "app.users.v1",
            AllocationSlotDescriptor::memory_manager(101),
        )
        .expect("retirement");

        let err = staged
            .stage_retirement_generation(&retirement, None)
            .expect_err("slot mismatch");

        assert!(matches!(
            err,
            AllocationRetirementError::SlotMismatch { .. }
        ));
    }

    #[test]
    fn snapshot_can_feed_validated_generation() {
        let snapshot = DeclarationSnapshot::new(vec![declaration("app.users.v1", 100, None)])
            .expect("snapshot");
        let (declarations, runtime_fingerprint) = snapshot.into_parts();
        let validated =
            crate::session::ValidatedAllocations::new(3, declarations, runtime_fingerprint);

        let staged = ledger().stage_validated_generation(&validated, None);

        assert_eq!(staged.allocation_history.records.len(), 1);
    }

    #[test]
    fn stage_validated_generation_records_runtime_fingerprint() {
        let validated = crate::session::ValidatedAllocations::new(
            3,
            vec![declaration("app.users.v1", 100, None)],
            Some("wasm:abc123".to_string()),
        );

        let staged = ledger().stage_validated_generation(&validated, None);

        assert_eq!(
            staged.allocation_history.generations[0].runtime_fingerprint,
            Some("wasm:abc123".to_string())
        );
    }

    #[test]
    fn validate_integrity_rejects_duplicate_stable_keys() {
        let mut ledger = ledger();
        ledger.allocation_history.records = vec![
            active_record("app.users.v1", 100),
            active_record("app.users.v1", 101),
        ];

        let err = ledger.validate_integrity().expect_err("duplicate key");

        assert!(matches!(
            err,
            LedgerIntegrityError::DuplicateStableKey { .. }
        ));
    }

    #[test]
    fn validate_integrity_rejects_duplicate_slots() {
        let mut ledger = ledger();
        ledger.allocation_history.records = vec![
            active_record("app.users.v1", 100),
            active_record("app.orders.v1", 100),
        ];

        let err = ledger.validate_integrity().expect_err("duplicate slot");

        assert!(matches!(err, LedgerIntegrityError::DuplicateSlot { .. }));
    }

    #[test]
    fn validate_integrity_rejects_retired_record_without_retired_generation() {
        let mut ledger = ledger();
        let mut record = active_record("app.users.v1", 100);
        record.state = AllocationState::Retired;
        ledger.allocation_history.records = vec![record];

        let err = ledger
            .validate_integrity()
            .expect_err("missing retired generation");

        assert!(matches!(
            err,
            LedgerIntegrityError::MissingRetiredGeneration { .. }
        ));
    }

    #[test]
    fn validate_integrity_rejects_non_retired_record_with_retired_generation() {
        let mut ledger = ledger();
        let mut record = active_record("app.users.v1", 100);
        record.retired_generation = Some(2);
        ledger.allocation_history.records = vec![record];

        let err = ledger
            .validate_integrity()
            .expect_err("unexpected retired generation");

        assert!(matches!(
            err,
            LedgerIntegrityError::UnexpectedRetiredGeneration { .. }
        ));
    }

    #[test]
    fn validate_integrity_rejects_non_increasing_schema_history() {
        let mut ledger = ledger();
        let mut record = active_record("app.users.v1", 100);
        record.schema_history.push(SchemaMetadataRecord {
            generation: 1,
            schema: SchemaMetadata::default(),
        });
        ledger.allocation_history.records = vec![record];

        let err = ledger
            .validate_integrity()
            .expect_err("non-increasing schema history");

        assert!(matches!(
            err,
            LedgerIntegrityError::NonIncreasingSchemaHistory { .. }
        ));
    }

    #[test]
    fn ledger_commit_store_rejects_invalid_ledger_before_write() {
        let mut store = LedgerCommitStore::default();
        let codec = TestCodec;
        let mut invalid = ledger();
        invalid.allocation_history.records = vec![
            active_record("app.users.v1", 100),
            active_record("app.orders.v1", 100),
        ];

        let err = store.commit(&invalid, &codec).expect_err("invalid ledger");

        assert!(matches!(
            err,
            LedgerCommitError::Integrity(LedgerIntegrityError::DuplicateSlot { .. })
        ));
        assert!(store.physical.is_uninitialized());
    }

    #[test]
    fn ledger_commit_store_recovers_latest_committed_ledger() {
        let mut store = LedgerCommitStore::default();
        let codec = TestCodec;
        let first = AllocationLedger {
            current_generation: 1,
            ..ledger()
        };
        let second = AllocationLedger {
            current_generation: 2,
            ..ledger()
        };

        store.commit(&first, &codec).expect("first commit");
        store.commit(&second, &codec).expect("second commit");
        let recovered = store.recover(&codec).expect("recovered ledger");

        assert_eq!(recovered.current_generation, 2);
    }

    #[test]
    fn ledger_commit_store_ignores_corrupt_inactive_ledger() {
        let mut store = LedgerCommitStore::default();
        let codec = TestCodec;
        let first = AllocationLedger {
            current_generation: 1,
            ..ledger()
        };
        let second = AllocationLedger {
            current_generation: 2,
            ..ledger()
        };

        store.commit(&first, &codec).expect("first commit");
        store
            .write_corrupt_inactive_ledger(&second, &codec)
            .expect("corrupt write");
        let recovered = store.recover(&codec).expect("recovered ledger");

        assert_eq!(recovered.current_generation, 1);
    }

    #[test]
    fn ledger_commit_store_initializes_empty_store_explicitly() {
        let mut store = LedgerCommitStore::default();
        let codec = TestCodec;
        let genesis = ledger();

        let recovered = store
            .recover_or_initialize(&codec, &genesis)
            .expect("initialized ledger");

        assert_eq!(recovered.current_generation, 3);
        assert!(!store.physical.is_uninitialized());
    }

    #[test]
    fn ledger_commit_store_rejects_corrupt_store_even_with_genesis() {
        let mut store = LedgerCommitStore::default();
        let codec = TestCodec;
        store
            .write_corrupt_inactive_ledger(&ledger(), &codec)
            .expect("corrupt write");

        let err = store
            .recover_or_initialize(&codec, &ledger())
            .expect_err("corrupt state");

        assert!(matches!(
            err,
            LedgerCommitError::Recovery(CommitRecoveryError::NoValidGeneration)
        ));
    }

    #[test]
    fn ledger_commit_store_rejects_incompatible_schema_before_write() {
        let mut store = LedgerCommitStore::default();
        let codec = TestCodec;
        let incompatible = AllocationLedger {
            ledger_schema_version: CURRENT_LEDGER_SCHEMA_VERSION + 1,
            ..ledger()
        };

        let err = store
            .commit(&incompatible, &codec)
            .expect_err("incompatible schema");

        assert!(matches!(
            err,
            LedgerCommitError::Compatibility(
                LedgerCompatibilityError::UnsupportedLedgerSchemaVersion { .. }
            )
        ));
        assert!(store.physical.is_uninitialized());
    }

    #[test]
    fn ledger_commit_store_rejects_incompatible_schema_on_recovery() {
        let mut store = LedgerCommitStore::default();
        let codec = TestCodec;
        let incompatible = AllocationLedger {
            ledger_schema_version: CURRENT_LEDGER_SCHEMA_VERSION + 1,
            ..ledger()
        };
        let payload = codec.encode(&incompatible).expect("payload");
        store
            .physical
            .commit_payload(payload)
            .expect("physical commit");

        let err = store.recover(&codec).expect_err("incompatible schema");

        assert!(matches!(
            err,
            LedgerCommitError::Compatibility(
                LedgerCompatibilityError::UnsupportedLedgerSchemaVersion { .. }
            )
        ));
    }

    #[test]
    fn ledger_commit_store_rejects_incompatible_physical_format() {
        let mut store = LedgerCommitStore::default();
        let codec = TestCodec;
        let incompatible = AllocationLedger {
            physical_format_id: CURRENT_PHYSICAL_FORMAT_ID + 1,
            ..ledger()
        };

        let err = store
            .recover_or_initialize(&codec, &incompatible)
            .expect_err("incompatible format");

        assert!(matches!(
            err,
            LedgerCommitError::Compatibility(
                LedgerCompatibilityError::UnsupportedPhysicalFormat { .. }
            )
        ));
        assert!(store.physical.is_uninitialized());
    }
}
