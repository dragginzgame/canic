use crate::cdk::structures::Memory;
#[cfg(target_arch = "wasm32")]
use crate::manager;
use crate::{
    cdk::structures::{
        DefaultMemoryImpl,
        cell::Cell,
        memory::{MemoryId, VirtualMemory},
    },
    manager::{MEMORY_MANAGER, RawStableMemoryState},
    registry::{MemoryRange, MemoryRangeAuthority, MemoryRegistryEntry, MemoryRegistryError},
};
use ic_memory::{
    AllocationHistory, AllocationLedger, AllocationRecord, AllocationSlotDescriptor,
    AllocationState, GenerationRecord, SchemaMetadata, SchemaMetadataRecord, StableKey,
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
const MEMORY_LAYOUT_LEDGER_LAYOUT_EPOCH: u32 = 1;
const MEMORY_LAYOUT_LEDGER_MAGIC: u64 = 0x4341_4E49_434D_454D;
const MEMORY_LAYOUT_LEDGER_FORMAT_ID: u32 = 1;
const MEMORY_LAYOUT_LEDGER_HEADER_LEN: u32 = 64;
const MEMORY_LAYOUT_LEDGER_COMMIT_MARKER: u64 = 0x434F_4D4D_4954_4544;
const STABLE_CELL_MAGIC: &[u8; 3] = b"SCL";
const STABLE_CELL_LAYOUT_VERSION: u8 = 1;
const STABLE_CELL_HEADER_SIZE: usize = 8;
const STABLE_CELL_VALUE_OFFSET: u64 = 8;
const WASM_PAGE_SIZE: u64 = 65_536;
const CANIC_FRAMEWORK_AUTHORITY_OWNER: &str = "canic.framework";
const CANIC_FRAMEWORK_AUTHORITY_PURPOSE: &str = "Canic framework allocation authority";
const APPLICATION_AUTHORITY_OWNER: &str = "applications";
const APPLICATION_AUTHORITY_PURPOSE: &str = "downstream application allocation authority";
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
    #[serde(default = "default_layout_epoch")]
    layout_epoch: u32,
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
            layout_epoch: MEMORY_LAYOUT_LEDGER_LAYOUT_EPOCH,
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

pub fn validate_bootstrap_state_before_cell_init(
    raw_state: RawStableMemoryState,
) -> Result<(), MemoryRegistryError> {
    match raw_state {
        RawStableMemoryState::Empty => Ok(()),
        RawStableMemoryState::ForeignOrCorrupt => Err(MemoryRegistryError::LedgerCorrupt {
            reason: "foreign or corrupt raw stable memory state",
        }),
        RawStableMemoryState::MemoryManager => {
            let memory = open_memory(MEMORY_LAYOUT_LEDGER_ID);
            validate_existing_ledger_memory(&memory)
        }
    }
}

///
/// MemoryLayoutLedgerSnapshot
///
/// Diagnostic snapshot decoded from the durable ID `0` ABI ledger.

pub struct MemoryLayoutLedgerSnapshot {
    pub magic: u64,
    pub format_id: u32,
    pub schema_version: u32,
    pub layout_epoch: u32,
    pub header_len: u32,
    pub header_checksum: u64,
    pub current_generation: u64,
    pub authorities: Vec<MemoryRangeAuthority>,
    pub ranges: Vec<(String, MemoryRange)>,
    pub entries: Vec<(u8, MemoryRegistryEntry)>,
}

#[cfg(target_arch = "wasm32")]
pub fn try_diagnostic_snapshot() -> Result<MemoryLayoutLedgerSnapshot, MemoryRegistryError> {
    match manager::classify_raw_stable_memory() {
        RawStableMemoryState::Empty => snapshot_from_record(&MemoryLayoutLedgerRecord::default()),
        RawStableMemoryState::ForeignOrCorrupt => Err(MemoryRegistryError::LedgerCorrupt {
            reason: "foreign or corrupt raw stable memory state",
        }),
        RawStableMemoryState::MemoryManager => {
            let memory = open_memory(MEMORY_LAYOUT_LEDGER_ID);
            diagnostic_snapshot_from_existing_memory(&memory)
        }
    }
}

///
/// MemoryLayoutGenerationRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct MemoryLayoutGenerationRecord {
    generation: u64,
    commit_marker: u64,
    checksum: u64,
    #[serde(default)]
    authorities: Vec<MemoryLayoutAuthorityRecord>,
    ranges: Vec<MemoryLayoutRangeRecord>,
    entries: Vec<MemoryLayoutEntryRecord>,
}

impl Default for MemoryLayoutGenerationRecord {
    fn default() -> Self {
        let mut generation = Self {
            generation: 0,
            commit_marker: MEMORY_LAYOUT_LEDGER_COMMIT_MARKER,
            checksum: 0,
            authorities: canonical_authority_records(),
            ranges: Vec::new(),
            entries: Vec::new(),
        };
        generation.checksum = generation_checksum(&generation);
        generation
    }
}

///
/// MemoryLayoutAuthorityRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct MemoryLayoutAuthorityRecord {
    owner: String,
    start: u8,
    end: u8,
    purpose: String,
}

impl MemoryLayoutAuthorityRecord {
    fn from_parts(owner: &str, range: MemoryRange, purpose: &str) -> Self {
        Self {
            owner: owner.to_string(),
            start: range.start,
            end: range.end,
            purpose: purpose.to_string(),
        }
    }

    const fn range(&self) -> MemoryRange {
        MemoryRange {
            start: self.start,
            end: self.end,
        }
    }

    fn to_snapshot(&self) -> MemoryRangeAuthority {
        MemoryRangeAuthority {
            owner: self.owner.clone(),
            range: self.range(),
            purpose: self.purpose.clone(),
        }
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
    #[serde(default)]
    schema_version: Option<u32>,
    #[serde(default)]
    schema_fingerprint: Option<String>,
    #[serde(default)]
    declarations: Vec<MemoryLayoutDeclarationRecord>,
}

///
/// MemoryLayoutDeclarationRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct MemoryLayoutDeclarationRecord {
    generation: u64,
    owner: String,
    label: String,
    #[serde(default)]
    schema_version: Option<u32>,
    #[serde(default)]
    schema_fingerprint: Option<String>,
}

impl MemoryLayoutEntryRecord {
    fn from_parts(
        id: u8,
        owner: &str,
        label: &str,
        stable_key: &str,
        schema_version: Option<u32>,
        schema_fingerprint: Option<&str>,
        generation: u64,
    ) -> Self {
        Self {
            id,
            owner: owner.to_string(),
            label: label.to_string(),
            stable_key: stable_key.to_string(),
            schema_version,
            schema_fingerprint: schema_fingerprint.map(str::to_string),
            declarations: vec![MemoryLayoutDeclarationRecord::from_parts(
                generation,
                owner,
                label,
                schema_version,
                schema_fingerprint,
            )],
        }
    }

    fn update_latest(
        &mut self,
        owner: &str,
        label: &str,
        schema_version: Option<u32>,
        schema_fingerprint: Option<&str>,
        generation: u64,
    ) {
        let schema_fingerprint = schema_fingerprint.map(str::to_string);
        let changed = self.owner != owner
            || self.label != label
            || self.schema_version != schema_version
            || self.schema_fingerprint != schema_fingerprint;

        self.owner = owner.to_string();
        self.label = label.to_string();
        self.schema_version = schema_version;
        self.schema_fingerprint.clone_from(&schema_fingerprint);

        if changed || self.declarations.is_empty() {
            self.declarations.push(MemoryLayoutDeclarationRecord {
                generation,
                owner: owner.to_string(),
                label: label.to_string(),
                schema_version,
                schema_fingerprint,
            });
        }
    }
}

impl MemoryLayoutDeclarationRecord {
    fn from_parts(
        generation: u64,
        owner: &str,
        label: &str,
        schema_version: Option<u32>,
        schema_fingerprint: Option<&str>,
    ) -> Self {
        Self {
            generation,
            owner: owner.to_string(),
            label: label.to_string(),
            schema_version,
            schema_fingerprint: schema_fingerprint.map(str::to_string),
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
    schema_version: Option<u32>,
    schema_fingerprint: Option<&str>,
) -> Result<(), MemoryRegistryError> {
    MEMORY_LAYOUT_LEDGER.with_borrow_mut(|cell| {
        let mut data = cell.get().clone();
        ensure_header(&mut data)?;
        let mut generation = authoritative_generation(&data)?;

        validate_entry_against_generation(&generation, id, owner, label, stable_key)?;
        let next_generation = generation.generation.saturating_add(1);
        for existing in &mut generation.entries {
            if existing.id == id && existing.stable_key == stable_key {
                existing.update_latest(
                    owner,
                    label,
                    schema_version,
                    schema_fingerprint,
                    next_generation,
                );
                commit_generation(&mut data, generation);
                cell.set(data);
                return Ok(());
            }
        }

        generation.entries.push(MemoryLayoutEntryRecord::from_parts(
            id,
            owner,
            label,
            stable_key,
            schema_version,
            schema_fingerprint,
            next_generation,
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

pub fn export_authorities() -> Vec<MemoryRangeAuthority> {
    try_export_authorities().unwrap_or_default()
}

pub fn try_export_authorities() -> Result<Vec<MemoryRangeAuthority>, MemoryRegistryError> {
    MEMORY_LAYOUT_LEDGER.with_borrow(|cell| {
        let generation = authoritative_generation(cell.get())?;
        Ok(generation
            .authorities
            .iter()
            .map(MemoryLayoutAuthorityRecord::to_snapshot)
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
                        schema_version: entry.schema_version,
                        schema_fingerprint: entry.schema_fingerprint.clone(),
                    },
                )
            })
            .collect())
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn try_snapshot() -> Result<MemoryLayoutLedgerSnapshot, MemoryRegistryError> {
    MEMORY_LAYOUT_LEDGER.with_borrow(|cell| snapshot_from_record(cell.get()))
}

pub fn try_allocation_ledger() -> Result<AllocationLedger, MemoryRegistryError> {
    MEMORY_LAYOUT_LEDGER.with_borrow(|cell| allocation_ledger_from_record(cell.get()))
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

fn validate_existing_ledger_memory<M: Memory>(memory: &M) -> Result<(), MemoryRegistryError> {
    let data = decode_existing_ledger_memory(memory)?;
    let generation = authoritative_generation(&data)?;
    if !generation.authorities.is_empty() {
        validate_canonical_authorities(&generation)?;
    }

    Ok(())
}

#[cfg(any(target_arch = "wasm32", test))]
fn diagnostic_snapshot_from_existing_memory<M: Memory>(
    memory: &M,
) -> Result<MemoryLayoutLedgerSnapshot, MemoryRegistryError> {
    let data = decode_existing_ledger_memory(memory)?;
    snapshot_from_record(&data)
}

fn decode_existing_ledger_memory<M: Memory>(
    memory: &M,
) -> Result<MemoryLayoutLedgerRecord, MemoryRegistryError> {
    if memory.size() == 0 {
        return Err(MemoryRegistryError::LedgerCorrupt {
            reason: "MemoryManager state exists without Canic ABI ledger",
        });
    }

    let mut header = [0; STABLE_CELL_HEADER_SIZE];
    memory.read(0, &mut header);
    if &header[0..3] != STABLE_CELL_MAGIC {
        return Err(MemoryRegistryError::LedgerCorrupt {
            reason: "ledger memory is not a stable cell",
        });
    }
    if header[3] != STABLE_CELL_LAYOUT_VERSION {
        return Err(MemoryRegistryError::LedgerCorrupt {
            reason: "unsupported ledger stable cell version",
        });
    }

    let value_len = u64::from(u32::from_le_bytes([
        header[4], header[5], header[6], header[7],
    ]));
    let available_bytes = memory.size().saturating_mul(WASM_PAGE_SIZE);
    if value_len > available_bytes.saturating_sub(STABLE_CELL_VALUE_OFFSET) {
        return Err(MemoryRegistryError::LedgerCorrupt {
            reason: "ledger stable cell length is invalid",
        });
    }

    let value_len = usize::try_from(value_len).map_err(|_| MemoryRegistryError::LedgerCorrupt {
        reason: "ledger stable cell length is invalid",
    })?;

    let mut bytes = vec![0; value_len];
    memory.read(STABLE_CELL_VALUE_OFFSET, &mut bytes);
    crate::serialize::deserialize(&bytes).map_err(|_| MemoryRegistryError::LedgerCorrupt {
        reason: "ledger stable cell payload is invalid",
    })
}

fn snapshot_from_record(
    data: &MemoryLayoutLedgerRecord,
) -> Result<MemoryLayoutLedgerSnapshot, MemoryRegistryError> {
    let generation = authoritative_generation(data)?;
    if !generation.authorities.is_empty() {
        validate_canonical_authorities(&generation)?;
    }

    Ok(MemoryLayoutLedgerSnapshot {
        magic: data.magic,
        format_id: data.format_id,
        schema_version: data.schema_version,
        layout_epoch: data.layout_epoch,
        header_len: data.header_len,
        header_checksum: data.header_checksum,
        current_generation: generation.generation,
        authorities: generation
            .authorities
            .iter()
            .map(MemoryLayoutAuthorityRecord::to_snapshot)
            .collect(),
        ranges: generation
            .ranges
            .iter()
            .map(|entry| (entry.owner.clone(), entry.range()))
            .collect(),
        entries: generation
            .entries
            .iter()
            .map(|entry| {
                (
                    entry.id,
                    MemoryRegistryEntry {
                        crate_name: entry.owner.clone(),
                        label: entry.label.clone(),
                        stable_key: entry.stable_key.clone(),
                        schema_version: entry.schema_version,
                        schema_fingerprint: entry.schema_fingerprint.clone(),
                    },
                )
            })
            .collect(),
    })
}

fn allocation_ledger_from_record(
    data: &MemoryLayoutLedgerRecord,
) -> Result<AllocationLedger, MemoryRegistryError> {
    let generation = authoritative_generation(data)?;
    if !generation.authorities.is_empty() {
        validate_canonical_authorities(&generation)?;
    }

    let records = generation
        .entries
        .iter()
        .map(allocation_record_from_entry)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(AllocationLedger {
        ledger_schema_version: data.schema_version,
        physical_format_id: data.format_id,
        current_generation: generation.generation,
        allocation_history: AllocationHistory {
            records,
            generations: generation_records_from_entries(&generation),
        },
    })
}

fn allocation_record_from_entry(
    entry: &MemoryLayoutEntryRecord,
) -> Result<AllocationRecord, MemoryRegistryError> {
    let stable_key = StableKey::parse(&entry.stable_key).map_err(|err| {
        MemoryRegistryError::InvalidStableKey {
            stable_key: err.stable_key,
            reason: err.reason,
        }
    })?;
    let schema_history = schema_history_from_entry(entry)?;
    let first_generation = schema_history.first().map_or(0, |record| record.generation);
    let last_seen_generation = schema_history
        .last()
        .map_or(first_generation, |record| record.generation);

    Ok(AllocationRecord {
        stable_key,
        slot: AllocationSlotDescriptor::memory_manager(entry.id),
        state: AllocationState::Active,
        first_generation,
        last_seen_generation,
        retired_generation: None,
        schema_history,
    })
}

fn schema_history_from_entry(
    entry: &MemoryLayoutEntryRecord,
) -> Result<Vec<SchemaMetadataRecord>, MemoryRegistryError> {
    let mut history = entry
        .declarations
        .iter()
        .map(|declaration| {
            schema_metadata_record(
                &entry.stable_key,
                declaration.generation,
                declaration.schema_version,
                declaration.schema_fingerprint.clone(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    if history.is_empty() {
        history.push(schema_metadata_record(
            &entry.stable_key,
            0,
            entry.schema_version,
            entry.schema_fingerprint.clone(),
        )?);
    }

    Ok(history)
}

fn schema_metadata_record(
    stable_key: &str,
    generation: u64,
    schema_version: Option<u32>,
    schema_fingerprint: Option<String>,
) -> Result<SchemaMetadataRecord, MemoryRegistryError> {
    let schema = SchemaMetadata::new(schema_version, schema_fingerprint).map_err(|err| {
        MemoryRegistryError::InvalidSchemaMetadata {
            stable_key: stable_key.to_string(),
            reason: schema_metadata_error_reason(err),
        }
    })?;

    Ok(SchemaMetadataRecord { generation, schema })
}

fn generation_records_from_entries(
    generation: &MemoryLayoutGenerationRecord,
) -> Vec<GenerationRecord> {
    let mut generations: Vec<GenerationRecord> = Vec::new();
    for entry in &generation.entries {
        for declaration in &entry.declarations {
            if let Some(record) = generations
                .iter_mut()
                .find(|record| record.generation == declaration.generation)
            {
                record.declaration_count = record.declaration_count.saturating_add(1);
            } else {
                generations.push(GenerationRecord {
                    generation: declaration.generation,
                    parent_generation: None,
                    runtime_fingerprint: None,
                    declaration_count: 1,
                    committed_at: None,
                });
            }
        }
    }

    if !generations
        .iter()
        .any(|record| record.generation == generation.generation)
    {
        generations.push(GenerationRecord {
            generation: generation.generation,
            parent_generation: None,
            runtime_fingerprint: None,
            declaration_count: 0,
            committed_at: None,
        });
    }

    generations.sort_by_key(|record| record.generation);
    generations
}

const fn schema_metadata_error_reason(err: ic_memory::SchemaMetadataError) -> &'static str {
    match err {
        ic_memory::SchemaMetadataError::InvalidVersion => {
            "schema_version must be greater than zero when present"
        }
        ic_memory::SchemaMetadataError::EmptyFingerprint => {
            "schema_fingerprint must not be empty when present"
        }
        ic_memory::SchemaMetadataError::FingerprintTooLong => {
            "schema_fingerprint must be at most 256 bytes"
        }
        ic_memory::SchemaMetadataError::NonAsciiFingerprint => "schema_fingerprint must be ASCII",
        ic_memory::SchemaMetadataError::ControlCharacterFingerprint => {
            "schema_fingerprint must not contain ASCII control characters"
        }
    }
}

fn ensure_header(data: &mut MemoryLayoutLedgerRecord) -> Result<(), MemoryRegistryError> {
    validate_header_fields(data)?;
    validate_header_checksum(data)?;
    let mut generation = authoritative_generation(data)?;
    if generation.authorities.is_empty() {
        generation.authorities = canonical_authority_records();
        commit_generation(data, generation);
        generation = authoritative_generation(data)?;
    } else {
        validate_canonical_authorities(&generation)?;
    }
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
    validate_header_fields(data)?;
    validate_header_checksum(data)?;

    let slot0 = data.slot0.as_ref().filter(|slot| valid_generation(slot));
    let slot1 = data.slot1.as_ref().filter(|slot| valid_generation(slot));

    let generation = match (slot0, slot1) {
        (Some(left), Some(right)) if right.generation > left.generation => right,
        (Some(left), Some(_) | None) => left,
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

    Ok(generation.clone())
}

const fn validate_header_fields(
    data: &MemoryLayoutLedgerRecord,
) -> Result<(), MemoryRegistryError> {
    if data.magic != MEMORY_LAYOUT_LEDGER_MAGIC {
        return Err(MemoryRegistryError::LedgerCorrupt {
            reason: "invalid ledger magic",
        });
    }

    if data.format_id != MEMORY_LAYOUT_LEDGER_FORMAT_ID {
        return Err(MemoryRegistryError::LedgerCorrupt {
            reason: "unsupported ledger physical format",
        });
    }

    if data.schema_version != MEMORY_LAYOUT_LEDGER_SCHEMA_VERSION {
        return Err(MemoryRegistryError::LedgerCorrupt {
            reason: "unsupported ledger schema version",
        });
    }

    if data.layout_epoch != MEMORY_LAYOUT_LEDGER_LAYOUT_EPOCH {
        return Err(MemoryRegistryError::LedgerCorrupt {
            reason: "unsupported ledger layout epoch",
        });
    }

    if data.header_len != MEMORY_LAYOUT_LEDGER_HEADER_LEN {
        return Err(MemoryRegistryError::LedgerCorrupt {
            reason: "invalid ledger header length",
        });
    }

    if data.committed_slot > 1 {
        return Err(MemoryRegistryError::LedgerCorrupt {
            reason: "invalid committed ledger slot",
        });
    }

    Ok(())
}

fn validate_header_checksum(data: &MemoryLayoutLedgerRecord) -> Result<(), MemoryRegistryError> {
    if data.header_checksum == header_checksum(data) {
        return Ok(());
    }

    if data.header_checksum == legacy_header_checksum(data) {
        return Ok(());
    }

    Err(MemoryRegistryError::LedgerCorrupt {
        reason: "invalid ledger header checksum",
    })
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
    data.layout_epoch = MEMORY_LAYOUT_LEDGER_LAYOUT_EPOCH;
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

fn canonical_authority_records() -> Vec<MemoryLayoutAuthorityRecord> {
    vec![
        MemoryLayoutAuthorityRecord::from_parts(
            CANIC_FRAMEWORK_AUTHORITY_OWNER,
            MemoryRange { start: 0, end: 99 },
            CANIC_FRAMEWORK_AUTHORITY_PURPOSE,
        ),
        MemoryLayoutAuthorityRecord::from_parts(
            APPLICATION_AUTHORITY_OWNER,
            MemoryRange {
                start: 100,
                end: 254,
            },
            APPLICATION_AUTHORITY_PURPOSE,
        ),
    ]
}

fn validate_canonical_authorities(
    generation: &MemoryLayoutGenerationRecord,
) -> Result<(), MemoryRegistryError> {
    if generation.authorities == canonical_authority_records() {
        return Ok(());
    }

    Err(MemoryRegistryError::LedgerCorrupt {
        reason: "canonical range authority records are invalid",
    })
}

fn header_checksum(data: &MemoryLayoutLedgerRecord) -> u64 {
    let mut hash = FNV_OFFSET;
    hash = hash_u64(hash, data.magic);
    hash = hash_u32(hash, data.format_id);
    hash = hash_u32(hash, data.schema_version);
    hash = hash_u32(hash, data.layout_epoch);
    hash = hash_u32(hash, data.header_len);
    hash = hash_u8(hash, data.committed_slot);
    hash_u64(hash, data.superblock_generation)
}

fn legacy_header_checksum(data: &MemoryLayoutLedgerRecord) -> u64 {
    let mut hash = FNV_OFFSET;
    hash = hash_u64(hash, data.magic);
    hash = hash_u32(hash, data.format_id);
    hash = hash_u32(hash, data.schema_version);
    hash = hash_u32(hash, data.header_len);
    hash = hash_u8(hash, data.committed_slot);
    hash_u64(hash, data.superblock_generation)
}

const fn default_layout_epoch() -> u32 {
    MEMORY_LAYOUT_LEDGER_LAYOUT_EPOCH
}

fn generation_checksum(generation: &MemoryLayoutGenerationRecord) -> u64 {
    let mut hash = FNV_OFFSET;
    hash = hash_u64(hash, generation.generation);
    hash = hash_u64(hash, generation.commit_marker);
    if !generation.authorities.is_empty() {
        hash = hash_usize(hash, generation.authorities.len());
        for authority in &generation.authorities {
            hash = hash_str(hash, &authority.owner);
            hash = hash_u8(hash, authority.start);
            hash = hash_u8(hash, authority.end);
            hash = hash_str(hash, &authority.purpose);
        }
    }
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
        hash = hash_option_u32(hash, entry.schema_version);
        hash = hash_option_str(hash, entry.schema_fingerprint.as_deref());
        hash = hash_usize(hash, entry.declarations.len());
        for declaration in &entry.declarations {
            hash = hash_u64(hash, declaration.generation);
            hash = hash_str(hash, &declaration.owner);
            hash = hash_str(hash, &declaration.label);
            hash = hash_option_u32(hash, declaration.schema_version);
            hash = hash_option_str(hash, declaration.schema_fingerprint.as_deref());
        }
    }
    hash
}

fn hash_usize(hash: u64, value: usize) -> u64 {
    hash_u64(hash, value as u64)
}

const fn hash_u8(hash: u64, value: u8) -> u64 {
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

fn hash_option_u32(hash: u64, value: Option<u32>) -> u64 {
    match value {
        Some(value) => hash_u32(hash_u8(hash, 1), value),
        None => hash_u8(hash, 0),
    }
}

fn hash_option_str(hash: u64, value: Option<&str>) -> u64 {
    match value {
        Some(value) => hash_str(hash_u8(hash, 1), value),
        None => hash_u8(hash, 0),
    }
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
        assert_eq!(authoritative.authorities, canonical_authority_records());
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
        assert_eq!(authoritative.authorities, canonical_authority_records());
        assert!(authoritative.ranges.is_empty());
    }

    #[test]
    fn default_generation_contains_canonical_authority_records() {
        let generation = MemoryLayoutGenerationRecord::default();

        assert_eq!(generation.authorities, canonical_authority_records());
    }

    #[test]
    fn ensure_header_rejects_corrupt_authority_records() {
        let mut record = MemoryLayoutLedgerRecord::default();
        let mut generation = authoritative_generation(&record).expect("generation");
        generation.authorities[0].end = 98;
        generation.checksum = generation_checksum(&generation);
        record.slot0 = Some(generation);
        record.header_checksum = header_checksum(&record);

        let err = ensure_header(&mut record).expect_err("corrupt authority must fail");

        assert!(matches!(err, MemoryRegistryError::LedgerCorrupt { .. }));
    }

    #[test]
    fn existing_ledger_memory_validation_distinguishes_genesis_and_valid_ledger() {
        let memory = DefaultMemoryImpl::default();
        let err = validate_existing_ledger_memory(&memory).expect_err("missing ledger should fail");
        assert!(matches!(err, MemoryRegistryError::LedgerCorrupt { .. }));

        let _cell =
            crate::cdk::structures::Cell::new(memory.clone(), MemoryLayoutLedgerRecord::default());

        validate_existing_ledger_memory(&memory).expect("valid ledger should pass");
    }

    #[test]
    fn diagnostic_snapshot_reads_existing_ledger_memory_without_runtime_registry() {
        let memory = DefaultMemoryImpl::default();
        let mut record = MemoryLayoutLedgerRecord::default();
        let mut generation = authoritative_generation(&record).expect("generation");
        generation.ranges.push(MemoryLayoutRangeRecord::from_parts(
            "crate_a",
            MemoryRange {
                start: 100,
                end: 102,
            },
        ));
        generation.entries.push(MemoryLayoutEntryRecord::from_parts(
            101,
            "crate_a",
            "slot",
            "app.crate_a.slot.v1",
            Some(1),
            Some("sha256:abc123"),
            1,
        ));
        commit_generation(&mut record, generation);
        let _cell = crate::cdk::structures::Cell::new(memory.clone(), record);

        let snapshot =
            diagnostic_snapshot_from_existing_memory(&memory).expect("diagnostic snapshot");

        assert_eq!(snapshot.magic, MEMORY_LAYOUT_LEDGER_MAGIC);
        assert_eq!(snapshot.format_id, MEMORY_LAYOUT_LEDGER_FORMAT_ID);
        assert_eq!(snapshot.schema_version, MEMORY_LAYOUT_LEDGER_SCHEMA_VERSION);
        assert_eq!(snapshot.layout_epoch, MEMORY_LAYOUT_LEDGER_LAYOUT_EPOCH);
        assert_eq!(snapshot.header_len, MEMORY_LAYOUT_LEDGER_HEADER_LEN);
        assert_eq!(snapshot.current_generation, 1);
        assert!(snapshot.authorities.iter().any(|authority| {
            authority.owner == "canic.framework"
                && authority.range == MemoryRange { start: 0, end: 99 }
        }));
        assert_eq!(
            snapshot.ranges,
            vec![(
                "crate_a".to_string(),
                MemoryRange {
                    start: 100,
                    end: 102,
                },
            )]
        );
        let (_, entry) = snapshot
            .entries
            .into_iter()
            .find(|(id, _)| *id == 101)
            .expect("entry");
        assert_eq!(entry.stable_key, "app.crate_a.slot.v1");
        assert_eq!(entry.schema_version, Some(1));
        assert_eq!(entry.schema_fingerprint.as_deref(), Some("sha256:abc123"));
    }

    #[test]
    fn existing_ledger_memory_validation_rejects_non_ledger_cells() {
        let memory = DefaultMemoryImpl::default();
        memory.grow(1);
        memory.write(0, b"BAD");

        let err = validate_existing_ledger_memory(&memory).expect_err("foreign cell should fail");
        assert!(matches!(err, MemoryRegistryError::LedgerCorrupt { .. }));

        let memory = DefaultMemoryImpl::default();
        memory.grow(1);
        memory.write(0, STABLE_CELL_MAGIC);
        memory.write(3, &[STABLE_CELL_LAYOUT_VERSION]);
        memory.write(4, &3_u32.to_le_bytes());
        memory.write(STABLE_CELL_VALUE_OFFSET, &[1, 2, 3]);

        let err =
            validate_existing_ledger_memory(&memory).expect_err("invalid payload should fail");
        assert!(matches!(err, MemoryRegistryError::LedgerCorrupt { .. }));
    }

    #[test]
    fn authoritative_generation_rejects_invalid_header_fields() {
        let record = MemoryLayoutLedgerRecord {
            magic: 0,
            ..MemoryLayoutLedgerRecord::default()
        };
        let err = authoritative_generation(&record).expect_err("invalid magic must fail");
        assert!(matches!(err, MemoryRegistryError::LedgerCorrupt { .. }));

        let record = MemoryLayoutLedgerRecord {
            format_id: 2,
            ..MemoryLayoutLedgerRecord::default()
        };
        let err = authoritative_generation(&record).expect_err("invalid format must fail");
        assert!(matches!(err, MemoryRegistryError::LedgerCorrupt { .. }));

        let record = MemoryLayoutLedgerRecord {
            schema_version: 2,
            ..MemoryLayoutLedgerRecord::default()
        };
        let err = authoritative_generation(&record).expect_err("invalid schema must fail");
        assert!(matches!(err, MemoryRegistryError::LedgerCorrupt { .. }));

        let record = MemoryLayoutLedgerRecord {
            layout_epoch: 2,
            ..MemoryLayoutLedgerRecord::default()
        };
        let err = authoritative_generation(&record).expect_err("invalid epoch must fail");
        assert!(matches!(err, MemoryRegistryError::LedgerCorrupt { .. }));

        let record = MemoryLayoutLedgerRecord {
            header_len: 0,
            ..MemoryLayoutLedgerRecord::default()
        };
        let err = authoritative_generation(&record).expect_err("invalid header len must fail");
        assert!(matches!(err, MemoryRegistryError::LedgerCorrupt { .. }));
    }

    #[test]
    fn authoritative_generation_accepts_legacy_header_checksum() {
        let mut record = MemoryLayoutLedgerRecord::default();
        record.header_checksum = legacy_header_checksum(&record);

        let generation = authoritative_generation(&record).expect("legacy checksum should pass");

        assert_eq!(generation.generation, 0);
    }

    #[test]
    fn authoritative_generation_rejects_invalid_header_checksum() {
        let mut record = MemoryLayoutLedgerRecord::default();
        record.header_checksum = record.header_checksum.wrapping_add(1);

        let err = authoritative_generation(&record).expect_err("bad checksum must fail");

        assert!(matches!(err, MemoryRegistryError::LedgerCorrupt { .. }));
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

    #[test]
    fn schema_metadata_changes_append_declaration_history() {
        reset_for_tests();

        record_entry(
            100,
            "crate_a",
            "slot",
            "app.crate_a.slot.v1",
            Some(1),
            Some("sha256:aaa"),
        )
        .expect("record first declaration");
        record_entry(
            100,
            "crate_a",
            "slot",
            "app.crate_a.slot.v1",
            Some(2),
            Some("sha256:bbb"),
        )
        .expect("record second declaration");

        MEMORY_LAYOUT_LEDGER.with_borrow(|cell| {
            let generation = authoritative_generation(cell.get()).expect("generation");
            let entry = generation
                .entries
                .iter()
                .find(|entry| entry.id == 100)
                .expect("entry");

            assert_eq!(entry.schema_version, Some(2));
            assert_eq!(entry.schema_fingerprint.as_deref(), Some("sha256:bbb"));
            assert_eq!(entry.declarations.len(), 2);
            assert_eq!(entry.declarations[0].schema_version, Some(1));
            assert_eq!(entry.declarations[1].schema_version, Some(2));
        });
    }

    #[test]
    fn allocation_ledger_projection_preserves_schema_history() {
        reset_for_tests();

        record_entry(
            100,
            "crate_a",
            "slot",
            "app.crate_a.slot.v1",
            Some(1),
            Some("sha256:aaa"),
        )
        .expect("record first declaration");
        record_entry(
            100,
            "crate_a",
            "slot",
            "app.crate_a.slot.v1",
            Some(2),
            Some("sha256:bbb"),
        )
        .expect("record second declaration");

        let ledger = try_allocation_ledger().expect("allocation ledger projection");

        assert_eq!(ledger.current_generation, 2);
        assert_eq!(ledger.allocation_history.records.len(), 1);
        assert_eq!(ledger.allocation_history.generations.len(), 2);

        let record = &ledger.allocation_history.records[0];
        assert_eq!(record.stable_key.as_str(), "app.crate_a.slot.v1");
        assert_eq!(record.slot, AllocationSlotDescriptor::memory_manager(100));
        assert_eq!(record.state, AllocationState::Active);
        assert_eq!(record.first_generation, 1);
        assert_eq!(record.last_seen_generation, 2);
        assert_eq!(record.schema_history.len(), 2);
        assert_eq!(record.schema_history[0].generation, 1);
        assert_eq!(record.schema_history[0].schema.schema_version, Some(1));
        assert_eq!(
            record.schema_history[0]
                .schema
                .schema_fingerprint
                .as_deref(),
            Some("sha256:aaa")
        );
        assert_eq!(record.schema_history[1].generation, 2);
        assert_eq!(record.schema_history[1].schema.schema_version, Some(2));
        assert_eq!(
            record.schema_history[1]
                .schema
                .schema_fingerprint
                .as_deref(),
            Some("sha256:bbb")
        );
    }
}
