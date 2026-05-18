use serde::{Deserialize, Serialize};

const COMMIT_MARKER: u64 = 0x4943_4D45_4D43_4F4D;
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

///
/// ProtectedGenerationSlot
///
/// One physical generation slot that can participate in protected recovery.
pub trait ProtectedGenerationSlot {
    /// Generation encoded by this slot.
    fn generation(&self) -> u64;

    /// Return whether the slot passed its marker/checksum validation.
    fn validates(&self) -> bool;
}

///
/// CommitSlotIndex
///
/// Physical dual-slot index selected by protected recovery.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum CommitSlotIndex {
    /// First physical commit slot.
    Slot0,
    /// Second physical commit slot.
    Slot1,
}

impl CommitSlotIndex {
    /// Return the opposite physical slot.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Slot0 => Self::Slot1,
            Self::Slot1 => Self::Slot0,
        }
    }
}

///
/// AuthoritativeSlot
///
/// Highest-generation valid slot selected by protected recovery.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthoritativeSlot<'slot, T> {
    /// Physical slot index.
    pub index: CommitSlotIndex,
    /// Valid committed generation in that slot.
    pub record: &'slot T,
}

/// Select the highest-generation valid physical slot.
pub fn select_authoritative_slot<'slot, T: ProtectedGenerationSlot>(
    slot0: Option<&'slot T>,
    slot1: Option<&'slot T>,
) -> Result<AuthoritativeSlot<'slot, T>, CommitRecoveryError> {
    let slot0 = slot0
        .filter(|slot| slot.validates())
        .map(|record| AuthoritativeSlot {
            index: CommitSlotIndex::Slot0,
            record,
        });
    let slot1 = slot1
        .filter(|slot| slot.validates())
        .map(|record| AuthoritativeSlot {
            index: CommitSlotIndex::Slot1,
            record,
        });

    match (slot0, slot1) {
        (Some(left), Some(right)) if right.record.generation() > left.record.generation() => {
            Ok(right)
        }
        (Some(left), Some(_) | None) => Ok(left),
        (None, Some(right)) => Ok(right),
        (None, None) => Err(CommitRecoveryError::NoValidGeneration),
    }
}

///
/// CommittedGenerationBytes
///
/// Physically committed ledger generation payload protected by a checksum.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CommittedGenerationBytes {
    /// Generation number represented by this payload.
    pub generation: u64,
    /// Physical commit marker. Readers reject records with an invalid marker.
    pub commit_marker: u64,
    /// Checksum over the generation, marker, and payload bytes.
    pub checksum: u64,
    /// Encoded ledger generation payload.
    pub payload: Vec<u8>,
}

impl CommittedGenerationBytes {
    /// Build a committed generation record.
    #[must_use]
    pub fn new(generation: u64, payload: Vec<u8>) -> Self {
        let mut record = Self {
            generation,
            commit_marker: COMMIT_MARKER,
            checksum: 0,
            payload,
        };
        record.checksum = generation_checksum(&record);
        record
    }

    /// Return whether the marker and checksum validate.
    #[must_use]
    pub fn validates(&self) -> bool {
        self.commit_marker == COMMIT_MARKER && self.checksum == generation_checksum(self)
    }
}

impl ProtectedGenerationSlot for CommittedGenerationBytes {
    fn generation(&self) -> u64 {
        self.generation
    }

    fn validates(&self) -> bool {
        self.validates()
    }
}

///
/// DualCommitStore
///
/// Dual-slot protected commit protocol for encoded ledger generations.
///
/// Writers stage a complete generation record into the inactive slot. Readers
/// recover by selecting the highest-generation valid slot. A torn or partial
/// write cannot become authoritative unless its marker and checksum validate.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct DualCommitStore {
    /// First physical commit slot.
    pub slot0: Option<CommittedGenerationBytes>,
    /// Second physical commit slot.
    pub slot1: Option<CommittedGenerationBytes>,
}

impl DualCommitStore {
    /// Return true when no commit slot has ever been written.
    #[must_use]
    pub const fn is_uninitialized(&self) -> bool {
        self.slot0.is_none() && self.slot1.is_none()
    }

    /// Return the highest-generation valid committed record.
    pub fn authoritative(&self) -> Result<&CommittedGenerationBytes, CommitRecoveryError> {
        select_authoritative_slot(self.slot0.as_ref(), self.slot1.as_ref())
            .map(|authoritative| authoritative.record)
    }

    /// Build a read-only recovery diagnostic for the protected commit slots.
    #[must_use]
    pub fn diagnostic(&self) -> CommitStoreDiagnostic {
        let authoritative = self.authoritative();
        CommitStoreDiagnostic {
            slot0: CommitSlotDiagnostic::from_slot(self.slot0.as_ref()),
            slot1: CommitSlotDiagnostic::from_slot(self.slot1.as_ref()),
            authoritative_generation: authoritative.ok().map(|record| record.generation),
            recovery_error: authoritative.err(),
        }
    }

    /// Commit a new payload to the inactive slot.
    ///
    /// The returned store models the post-write physical state. If a real
    /// substrate traps before the inactive slot is fully written, the prior
    /// valid slot remains authoritative under `authoritative`.
    pub fn commit_payload(
        &mut self,
        payload: Vec<u8>,
    ) -> Result<&CommittedGenerationBytes, CommitRecoveryError> {
        let next_generation = self
            .authoritative()
            .map_or(0, |record| record.generation.saturating_add(1));
        let next = CommittedGenerationBytes::new(next_generation, payload);

        if self.inactive_slot_index() == 0 {
            self.slot0 = Some(next);
        } else {
            self.slot1 = Some(next);
        }

        self.authoritative()
    }

    /// Simulate a torn write into the inactive slot.
    ///
    /// This helper is intentionally part of the model because recovery behavior
    /// is an ABI requirement, not an implementation detail.
    pub fn write_corrupt_inactive_slot(&mut self, generation: u64, payload: Vec<u8>) {
        let mut corrupt = CommittedGenerationBytes::new(generation, payload);
        corrupt.checksum = corrupt.checksum.wrapping_add(1);

        if self.inactive_slot_index() == 0 {
            self.slot0 = Some(corrupt);
        } else {
            self.slot1 = Some(corrupt);
        }
    }

    fn inactive_slot_index(&self) -> u8 {
        match select_authoritative_slot(self.slot0.as_ref(), self.slot1.as_ref()) {
            Ok(authoritative) if authoritative.index == CommitSlotIndex::Slot0 => 1,
            Ok(_) | Err(_) => 0,
        }
    }
}

///
/// CommitStoreDiagnostic
///
/// Read-only diagnostic summary of protected commit recovery state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CommitStoreDiagnostic {
    /// First physical commit slot diagnostic.
    pub slot0: CommitSlotDiagnostic,
    /// Second physical commit slot diagnostic.
    pub slot1: CommitSlotDiagnostic,
    /// Highest valid generation selected by recovery.
    pub authoritative_generation: Option<u64>,
    /// Recovery error when no authoritative generation can be selected.
    pub recovery_error: Option<CommitRecoveryError>,
}

///
/// CommitSlotDiagnostic
///
/// Read-only diagnostic summary for one protected commit slot.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CommitSlotDiagnostic {
    /// Whether a physical slot record is present.
    pub present: bool,
    /// Generation encoded by the slot, if present.
    pub generation: Option<u64>,
    /// Whether marker and checksum validation succeeded.
    pub valid: bool,
}

impl CommitSlotDiagnostic {
    fn from_slot(slot: Option<&CommittedGenerationBytes>) -> Self {
        match slot {
            Some(record) => Self {
                present: true,
                generation: Some(record.generation),
                valid: record.validates(),
            },
            None => Self {
                present: false,
                generation: None,
                valid: false,
            },
        }
    }
}

///
/// CommitRecoveryError
///
/// Protected commit recovery failure.
#[derive(Clone, Copy, Debug, Deserialize, Eq, thiserror::Error, PartialEq, Serialize)]
pub enum CommitRecoveryError {
    /// No committed slot passed marker and checksum validation.
    #[error("no valid committed ledger generation")]
    NoValidGeneration,
}

fn generation_checksum(generation: &CommittedGenerationBytes) -> u64 {
    let mut hash = FNV_OFFSET;
    hash = hash_u64(hash, generation.generation);
    hash = hash_u64(hash, generation.commit_marker);
    hash = hash_usize(hash, generation.payload.len());
    for byte in &generation.payload {
        hash = hash_byte(hash, *byte);
    }
    hash
}

fn hash_usize(hash: u64, value: usize) -> u64 {
    hash_u64(hash, value as u64)
}

fn hash_u64(mut hash: u64, value: u64) -> u64 {
    for byte in value.to_le_bytes() {
        hash = hash_byte(hash, byte);
    }
    hash
}

const fn hash_byte(hash: u64, byte: u8) -> u64 {
    (hash ^ byte as u64).wrapping_mul(FNV_PRIME)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(value: u8) -> Vec<u8> {
        vec![value; 4]
    }

    #[test]
    fn committed_generation_validates_marker_and_checksum() {
        let mut generation = CommittedGenerationBytes::new(7, payload(1));
        assert!(generation.validates());

        generation.checksum = generation.checksum.wrapping_add(1);
        assert!(!generation.validates());
    }

    #[test]
    fn authoritative_selects_highest_valid_generation() {
        let mut store = DualCommitStore::default();
        store.commit_payload(payload(1)).expect("first commit");
        store.commit_payload(payload(2)).expect("second commit");

        let authoritative = store.authoritative().expect("authoritative");
        let authoritative_slot =
            select_authoritative_slot(store.slot0.as_ref(), store.slot1.as_ref())
                .expect("authoritative slot");

        assert_eq!(authoritative.generation, 1);
        assert_eq!(authoritative.payload, payload(2));
        assert_eq!(authoritative_slot.index, CommitSlotIndex::Slot1);
        assert_eq!(authoritative_slot.record.payload, payload(2));
    }

    #[test]
    fn corrupt_newer_slot_leaves_prior_generation_authoritative() {
        let mut store = DualCommitStore::default();
        store.commit_payload(payload(1)).expect("first commit");
        store.write_corrupt_inactive_slot(1, payload(2));

        let authoritative = store.authoritative().expect("authoritative");

        assert_eq!(authoritative.generation, 0);
        assert_eq!(authoritative.payload, payload(1));
    }

    #[test]
    fn no_valid_generation_fails_closed() {
        let mut store = DualCommitStore::default();
        store.write_corrupt_inactive_slot(0, payload(1));
        store.write_corrupt_inactive_slot(1, payload(2));

        let err = store.authoritative().expect_err("no valid slot");

        assert_eq!(err, CommitRecoveryError::NoValidGeneration);
    }

    #[test]
    fn diagnostic_reports_authoritative_generation_and_corrupt_slots() {
        let mut store = DualCommitStore::default();
        store.commit_payload(payload(1)).expect("first commit");
        store.write_corrupt_inactive_slot(1, payload(2));

        let diagnostic = store.diagnostic();

        assert_eq!(diagnostic.authoritative_generation, Some(0));
        assert_eq!(diagnostic.recovery_error, None);
        assert_eq!(diagnostic.slot0.generation, Some(0));
        assert!(diagnostic.slot0.valid);
        assert_eq!(diagnostic.slot1.generation, Some(1));
        assert!(!diagnostic.slot1.valid);
    }

    #[test]
    fn diagnostic_reports_no_valid_generation_for_empty_store() {
        let diagnostic = DualCommitStore::default().diagnostic();

        assert_eq!(diagnostic.authoritative_generation, None);
        assert_eq!(
            diagnostic.recovery_error,
            Some(CommitRecoveryError::NoValidGeneration)
        );
        assert!(!diagnostic.slot0.present);
        assert!(!diagnostic.slot1.present);
    }

    #[test]
    fn uninitialized_distinguishes_empty_from_corrupt() {
        let mut store = DualCommitStore::default();
        assert!(store.is_uninitialized());

        store.write_corrupt_inactive_slot(0, payload(1));

        assert!(!store.is_uninitialized());
    }

    #[test]
    fn commit_after_corrupt_slot_advances_from_prior_valid_generation() {
        let mut store = DualCommitStore::default();
        store.commit_payload(payload(1)).expect("first commit");
        store.write_corrupt_inactive_slot(1, payload(2));
        store.commit_payload(payload(3)).expect("third commit");

        let authoritative = store.authoritative().expect("authoritative");

        assert_eq!(authoritative.generation, 1);
        assert_eq!(authoritative.payload, payload(3));
    }
}
