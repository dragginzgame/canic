use crate::{
    cdk::structures::{
        DefaultMemoryImpl,
        cell::Cell,
        memory::{MemoryId, VirtualMemory},
    },
    manager::MEMORY_MANAGER,
    registry::{MemoryRange, MemoryRegistryEntry, MemoryRegistryError},
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

pub const MEMORY_LAYOUT_LEDGER_ID: u8 = 0;
pub const MEMORY_LAYOUT_LEDGER_OWNER: &str = "canic-memory";
pub const MEMORY_LAYOUT_LEDGER_LABEL: &str = "MemoryLayoutLedger";
pub const MEMORY_LAYOUT_LEDGER_STABLE_KEY: &str = "canic.memory.abi_ledger.v1";
pub const MEMORY_LAYOUT_RESERVED_MIN: u8 = 0;
pub const MEMORY_LAYOUT_RESERVED_MAX: u8 = 0;

const MEMORY_LAYOUT_LEDGER_SCHEMA_VERSION: u32 = 1;
const MEMORY_LAYOUT_LEDGER_MAGIC: u64 = 0x4341_4E49_434D_454D;
const MEMORY_LAYOUT_LEDGER_FORMAT_ID: u32 = 1;
const MEMORY_LAYOUT_LEDGER_HEADER_LEN: u32 = 64;
const MEMORY_LAYOUT_LEDGER_COMMIT_MARKER: u64 = 0x434F_4D4D_4954_4544;
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

thread_local! {
    static MEMORY_LAYOUT_LEDGER: RefCell<
        Cell<MemoryLayoutLedgerRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(Cell::init(
        open_memory(MEMORY_LAYOUT_LEDGER_ID),
        MemoryLayoutLedgerRecord::default(),
    ));
}

///
/// MemoryLayoutLedgerRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct MemoryLayoutLedgerRecord {
    magic: u64,
    format_id: u32,
    schema_version: u32,
    header_len: u32,
    header_checksum: u64,
    committed_slot: u8,
    superblock_generation: u64,
    slot0: Option<MemoryLayoutGenerationRecord>,
    slot1: Option<MemoryLayoutGenerationRecord>,
}

impl Default for MemoryLayoutLedgerRecord {
    fn default() -> Self {
        let generation = MemoryLayoutGenerationRecord::default();
        let mut record = Self {
            magic: MEMORY_LAYOUT_LEDGER_MAGIC,
            format_id: MEMORY_LAYOUT_LEDGER_FORMAT_ID,
            schema_version: MEMORY_LAYOUT_LEDGER_SCHEMA_VERSION,
            header_len: MEMORY_LAYOUT_LEDGER_HEADER_LEN,
            header_checksum: 0,
            committed_slot: 0,
            superblock_generation: generation.generation,
            slot0: Some(generation),
            slot1: None,
        };
        record.header_checksum = header_checksum(&record);
        record
    }
}

crate::impl_storable_unbounded!(MemoryLayoutLedgerRecord);

///
/// MemoryLayoutGenerationRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct MemoryLayoutGenerationRecord {
    generation: u64,
    commit_marker: u64,
    checksum: u64,
    ranges: Vec<MemoryLayoutRangeRecord>,
    entries: Vec<MemoryLayoutEntryRecord>,
}

impl Default for MemoryLayoutGenerationRecord {
    fn default() -> Self {
        let mut generation = Self {
            generation: 0,
            commit_marker: MEMORY_LAYOUT_LEDGER_COMMIT_MARKER,
            checksum: 0,
            ranges: Vec::new(),
            entries: Vec::new(),
        };
        generation.checksum = generation_checksum(&generation);
        generation
    }
}

///
/// MemoryLayoutRangeRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct MemoryLayoutRangeRecord {
    owner: String,
    start: u8,
    end: u8,
}

impl MemoryLayoutRangeRecord {
    fn from_parts(owner: &str, range: MemoryRange) -> Self {
        Self {
            owner: owner.to_string(),
            start: range.start,
            end: range.end,
        }
    }

    const fn range(&self) -> MemoryRange {
        MemoryRange {
            start: self.start,
            end: self.end,
        }
    }
}

///
/// MemoryLayoutEntryRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct MemoryLayoutEntryRecord {
    id: u8,
    owner: String,
    label: String,
    stable_key: String,
}

impl MemoryLayoutEntryRecord {
    fn from_parts(id: u8, owner: &str, label: &str, stable_key: &str) -> Self {
        Self {
            id,
            owner: owner.to_string(),
            label: label.to_string(),
            stable_key: stable_key.to_string(),
        }
    }
}

pub fn record_range(owner: &str, range: MemoryRange) -> Result<(), MemoryRegistryError> {
    MEMORY_LAYOUT_LEDGER.with_borrow_mut(|cell| {
        let mut data = cell.get().clone();
        ensure_header(&mut data)?;
        let mut generation = authoritative_generation(&data)?;

        validate_range_against_generation(&generation, owner, range)?;
        for existing in &mut generation.ranges {
            let existing_range = existing.range();
            if existing_range.start == range.start && existing_range.end == range.end {
                existing.owner = owner.to_string();
                commit_generation(&mut data, generation);
                cell.set(data);
                return Ok(());
            }
        }

        generation
            .ranges
            .push(MemoryLayoutRangeRecord::from_parts(owner, range));
        generation.ranges.sort_by_key(|entry| entry.start);
        commit_generation(&mut data, generation);
        cell.set(data);

        Ok(())
    })
}

pub fn validate_range(owner: &str, range: MemoryRange) -> Result<(), MemoryRegistryError> {
    MEMORY_LAYOUT_LEDGER.with_borrow(|cell| {
        let generation = authoritative_generation(cell.get())?;
        validate_range_against_generation(&generation, owner, range)
    })
}

pub fn record_entry(
    id: u8,
    owner: &str,
    label: &str,
    stable_key: &str,
) -> Result<(), MemoryRegistryError> {
    MEMORY_LAYOUT_LEDGER.with_borrow_mut(|cell| {
        let mut data = cell.get().clone();
        ensure_header(&mut data)?;
        let mut generation = authoritative_generation(&data)?;

        validate_entry_against_generation(&generation, id, owner, label, stable_key)?;
        for existing in &mut generation.entries {
            if existing.id == id && existing.stable_key == stable_key {
                existing.owner = owner.to_string();
                existing.label = label.to_string();
                commit_generation(&mut data, generation);
                cell.set(data);
                return Ok(());
            }
        }

        generation.entries.push(MemoryLayoutEntryRecord::from_parts(
            id, owner, label, stable_key,
        ));
        generation.entries.sort_by_key(|entry| entry.id);
        commit_generation(&mut data, generation);
        cell.set(data);

        Ok(())
    })
}

pub fn validate_entry(
    id: u8,
    owner: &str,
    label: &str,
    stable_key: &str,
) -> Result<(), MemoryRegistryError> {
    MEMORY_LAYOUT_LEDGER.with_borrow(|cell| {
        let generation = authoritative_generation(cell.get())?;
        validate_entry_against_generation(&generation, id, owner, label, stable_key)
    })
}

pub fn export_ranges() -> Vec<(String, MemoryRange)> {
    try_export_ranges().unwrap_or_default()
}

pub fn try_export_ranges() -> Result<Vec<(String, MemoryRange)>, MemoryRegistryError> {
    MEMORY_LAYOUT_LEDGER.with_borrow(|cell| {
        let generation = authoritative_generation(cell.get())?;
        Ok(generation
            .ranges
            .iter()
            .map(|entry| (entry.owner.clone(), entry.range()))
            .collect())
    })
}

pub fn export_entries() -> Vec<(u8, MemoryRegistryEntry)> {
    try_export_entries().unwrap_or_default()
}

pub fn try_export_entries() -> Result<Vec<(u8, MemoryRegistryEntry)>, MemoryRegistryError> {
    MEMORY_LAYOUT_LEDGER.with_borrow(|cell| {
        let generation = authoritative_generation(cell.get())?;
        Ok(generation
            .entries
            .iter()
            .map(|entry| {
                (
                    entry.id,
                    MemoryRegistryEntry {
                        crate_name: entry.owner.clone(),
                        label: entry.label.clone(),
                        stable_key: entry.stable_key.clone(),
                    },
                )
            })
            .collect())
    })
}

#[cfg(test)]
pub fn reset_for_tests() {
    MEMORY_LAYOUT_LEDGER.with_borrow_mut(|cell| {
        cell.set(MemoryLayoutLedgerRecord::default());
    });
}

fn open_memory(id: u8) -> VirtualMemory<DefaultMemoryImpl> {
    MEMORY_MANAGER.with_borrow_mut(|mgr| mgr.get(MemoryId::new(id)))
}

fn ensure_header(data: &mut MemoryLayoutLedgerRecord) -> Result<(), MemoryRegistryError> {
    if data.magic != MEMORY_LAYOUT_LEDGER_MAGIC {
        data.magic = MEMORY_LAYOUT_LEDGER_MAGIC;
    }
    if data.format_id == 0 {
        data.format_id = MEMORY_LAYOUT_LEDGER_FORMAT_ID;
    }
    if data.schema_version == 0 {
        data.schema_version = MEMORY_LAYOUT_LEDGER_SCHEMA_VERSION;
    }
    if data.header_len == 0 {
        data.header_len = MEMORY_LAYOUT_LEDGER_HEADER_LEN;
    }
    let generation = authoritative_generation(data)?;
    if data.slot0.is_none() && data.slot1.is_none() {
        data.slot0 = Some(generation);
        data.committed_slot = 0;
    }
    data.superblock_generation = authoritative_generation(data)?.generation;
    data.header_checksum = header_checksum(data);
    Ok(())
}

fn authoritative_generation(
    data: &MemoryLayoutLedgerRecord,
) -> Result<MemoryLayoutGenerationRecord, MemoryRegistryError> {
    let slot0 = data.slot0.as_ref().filter(|slot| valid_generation(slot));
    let slot1 = data.slot1.as_ref().filter(|slot| valid_generation(slot));

    let generation = match (slot0, slot1) {
        (Some(left), Some(right)) if right.generation > left.generation => right,
        (Some(left), Some(_)) | (Some(left), None) => left,
        (None, Some(right)) => right,
        (None, None) if data.slot0.is_none() && data.slot1.is_none() => {
            return Ok(MemoryLayoutGenerationRecord::default());
        }
        (None, None) => {
            return Err(MemoryRegistryError::LedgerCorrupt {
                reason: "no valid committed generation",
            });
        }
    };

    if data.magic != MEMORY_LAYOUT_LEDGER_MAGIC {
        return Err(MemoryRegistryError::LedgerCorrupt {
            reason: "invalid ledger magic",
        });
    }

    Ok(generation.clone())
}

fn valid_generation(generation: &MemoryLayoutGenerationRecord) -> bool {
    generation.commit_marker == MEMORY_LAYOUT_LEDGER_COMMIT_MARKER
        && generation.checksum == generation_checksum(generation)
}

fn commit_generation(
    data: &mut MemoryLayoutLedgerRecord,
    mut generation: MemoryLayoutGenerationRecord,
) {
    generation.generation = generation.generation.saturating_add(1);
    generation.commit_marker = MEMORY_LAYOUT_LEDGER_COMMIT_MARKER;
    generation.checksum = generation_checksum(&generation);

    if data.committed_slot == 0 {
        data.slot1 = Some(generation.clone());
        data.committed_slot = 1;
    } else {
        data.slot0 = Some(generation.clone());
        data.committed_slot = 0;
    }

    data.magic = MEMORY_LAYOUT_LEDGER_MAGIC;
    data.format_id = MEMORY_LAYOUT_LEDGER_FORMAT_ID;
    data.schema_version = MEMORY_LAYOUT_LEDGER_SCHEMA_VERSION;
    data.header_len = MEMORY_LAYOUT_LEDGER_HEADER_LEN;
    data.superblock_generation = generation.generation;
    data.header_checksum = header_checksum(data);
}

fn validate_range_against_generation(
    generation: &MemoryLayoutGenerationRecord,
    owner: &str,
    range: MemoryRange,
) -> Result<(), MemoryRegistryError> {
    for existing in &generation.ranges {
        let existing_range = existing.range();
        if ranges_overlap(existing_range, range) {
            if existing_range.start == range.start && existing_range.end == range.end {
                return Ok(());
            }

            return Err(MemoryRegistryError::HistoricalRangeConflict {
                existing_crate: existing.owner.clone(),
                existing_start: existing_range.start,
                existing_end: existing_range.end,
                new_crate: owner.to_string(),
                new_start: range.start,
                new_end: range.end,
            });
        }
    }

    Ok(())
}

fn validate_entry_against_generation(
    generation: &MemoryLayoutGenerationRecord,
    id: u8,
    owner: &str,
    label: &str,
    stable_key: &str,
) -> Result<(), MemoryRegistryError> {
    for existing in &generation.entries {
        if existing.id == id {
            if existing.stable_key == stable_key {
                return Ok(());
            }

            return Err(MemoryRegistryError::HistoricalIdConflict {
                id,
                existing_crate: existing.owner.clone(),
                existing_label: existing.label.clone(),
                new_crate: owner.to_string(),
                new_label: label.to_string(),
                new_stable_key: stable_key.to_string(),
            });
        }

        if existing.stable_key == stable_key {
            return Err(MemoryRegistryError::HistoricalStableKeyConflict {
                stable_key: stable_key.to_string(),
                existing_id: existing.id,
                new_id: id,
            });
        }
    }

    Ok(())
}

fn header_checksum(data: &MemoryLayoutLedgerRecord) -> u64 {
    let mut hash = FNV_OFFSET;
    hash = hash_u64(hash, data.magic);
    hash = hash_u32(hash, data.format_id);
    hash = hash_u32(hash, data.schema_version);
    hash = hash_u32(hash, data.header_len);
    hash = hash_u8(hash, data.committed_slot);
    hash = hash_u64(hash, data.superblock_generation);
    hash = hash_generation_option(hash, data.slot0.as_ref());
    hash_generation_option(hash, data.slot1.as_ref())
}

fn generation_checksum(generation: &MemoryLayoutGenerationRecord) -> u64 {
    let mut hash = FNV_OFFSET;
    hash = hash_u64(hash, generation.generation);
    hash = hash_u64(hash, generation.commit_marker);
    hash = hash_usize(hash, generation.ranges.len());
    for range in &generation.ranges {
        hash = hash_str(hash, &range.owner);
        hash = hash_u8(hash, range.start);
        hash = hash_u8(hash, range.end);
    }
    hash = hash_usize(hash, generation.entries.len());
    for entry in &generation.entries {
        hash = hash_u8(hash, entry.id);
        hash = hash_str(hash, &entry.owner);
        hash = hash_str(hash, &entry.label);
        hash = hash_str(hash, &entry.stable_key);
    }
    hash
}

fn hash_generation_option(hash: u64, generation: Option<&MemoryLayoutGenerationRecord>) -> u64 {
    match generation {
        Some(generation) => {
            let hash = hash_u8(hash, 1);
            let hash = hash_u64(hash, generation.generation);
            hash_u64(hash, generation.checksum)
        }
        None => hash_u8(hash, 0),
    }
}

fn hash_usize(hash: u64, value: usize) -> u64 {
    hash_u64(hash, value as u64)
}

fn hash_u8(hash: u64, value: u8) -> u64 {
    hash_byte(hash, value)
}

fn hash_u32(mut hash: u64, value: u32) -> u64 {
    for byte in value.to_le_bytes() {
        hash = hash_byte(hash, byte);
    }
    hash
}

fn hash_u64(mut hash: u64, value: u64) -> u64 {
    for byte in value.to_le_bytes() {
        hash = hash_byte(hash, byte);
    }
    hash
}

fn hash_str(mut hash: u64, value: &str) -> u64 {
    hash = hash_usize(hash, value.len());
    for byte in value.as_bytes() {
        hash = hash_byte(hash, *byte);
    }
    hash
}

const fn hash_byte(hash: u64, byte: u8) -> u64 {
    (hash ^ byte as u64).wrapping_mul(FNV_PRIME)
}

const fn ranges_overlap(a: MemoryRange, b: MemoryRange) -> bool {
    a.start <= b.end && b.start <= a.end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authoritative_generation_chooses_highest_valid_slot() {
        let mut record = MemoryLayoutLedgerRecord::default();
        let mut generation = authoritative_generation(&record).expect("authoritative generation");
        generation.ranges.push(MemoryLayoutRangeRecord::from_parts(
            "crate_a",
            MemoryRange {
                start: 100,
                end: 102,
            },
        ));
        commit_generation(&mut record, generation);

        let authoritative = authoritative_generation(&record).expect("authoritative generation");

        assert_eq!(authoritative.generation, 1);
        assert_eq!(authoritative.ranges.len(), 1);
    }

    #[test]
    fn authoritative_generation_ignores_corrupt_newer_slot() {
        let mut record = MemoryLayoutLedgerRecord::default();
        let mut generation = authoritative_generation(&record).expect("authoritative generation");
        generation.ranges.push(MemoryLayoutRangeRecord::from_parts(
            "crate_a",
            MemoryRange {
                start: 100,
                end: 102,
            },
        ));
        commit_generation(&mut record, generation);

        if let Some(slot) = &mut record.slot1 {
            slot.checksum = slot.checksum.wrapping_add(1);
        }

        let authoritative = authoritative_generation(&record).expect("authoritative generation");

        assert_eq!(authoritative.generation, 0);
        assert!(authoritative.ranges.is_empty());
    }

    #[test]
    fn authoritative_generation_fails_when_no_slot_validates() {
        let mut record = MemoryLayoutLedgerRecord::default();
        if let Some(slot) = &mut record.slot0 {
            slot.checksum = slot.checksum.wrapping_add(1);
        }

        let err = authoritative_generation(&record).expect_err("corrupt ledger should fail");

        assert!(matches!(err, MemoryRegistryError::LedgerCorrupt { .. }));
    }
}
