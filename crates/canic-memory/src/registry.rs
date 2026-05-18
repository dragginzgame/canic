use crate::{ThisError, ledger};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::BTreeMap};

///
/// MemoryRange
///
/// Inclusive stable-memory ID range reserved by one owner crate.

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MemoryRange {
    /// First stable-memory ID in the range.
    pub start: u8,
    /// Last stable-memory ID in the range.
    pub end: u8,
}

impl MemoryRange {
    /// Return whether `id` is inside this inclusive range.
    #[must_use]
    pub const fn contains(&self, id: u8) -> bool {
        id >= self.start && id <= self.end
    }
}

///
/// MemoryRegistryEntry
///
/// Registered stable-memory slot metadata.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MemoryRegistryEntry {
    /// Crate name that registered the stable-memory slot.
    pub crate_name: String,
    /// Human-readable label for the registered stable-memory slot.
    pub label: String,
    /// Explicit ABI-stable key that owns this memory ID permanently.
    pub stable_key: String,
    /// Optional in-place schema version metadata for diagnostics.
    pub schema_version: Option<u32>,
    /// Optional opaque schema fingerprint metadata for diagnostics.
    pub schema_fingerprint: Option<String>,
}

///
/// MemoryRangeEntry
///
/// Reserved stable-memory range with explicit owner context.

#[derive(Clone, Debug)]
pub struct MemoryRangeEntry {
    /// Crate name that reserved the range.
    pub owner: String,
    /// Inclusive stable-memory ID range reserved by `owner`.
    pub range: MemoryRange,
}

///
/// MemoryRangeSnapshot
///
/// Registered stable-memory slots grouped under one reserved range.

#[derive(Clone, Debug)]
pub struct MemoryRangeSnapshot {
    /// Crate name that reserved the range.
    pub owner: String,
    /// Inclusive stable-memory ID range reserved by `owner`.
    pub range: MemoryRange,
    /// Registered entries whose IDs fall inside `range`.
    pub entries: Vec<(u8, MemoryRegistryEntry)>,
}

///
/// MemoryRangeAuthority
///
/// Durable allocation-authority range recorded by the ABI ledger.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MemoryRangeAuthority {
    /// Authority label for the range.
    pub owner: String,
    /// Inclusive stable-memory ID range controlled by this authority.
    pub range: MemoryRange,
    /// Stable diagnostic purpose for the authority record.
    pub purpose: String,
}

///
/// PendingRegistration
///
/// One stable-memory declaration collected before bootstrap validation.

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PendingRegistration {
    pub id: u8,
    pub crate_name: String,
    pub label: String,
    pub stable_key: String,
    pub schema_version: Option<u32>,
    pub schema_fingerprint: Option<String>,
}

///
/// MemoryRegistryError
///
/// Errors returned when a memory range or ID registration is invalid.

#[derive(Debug, ThisError)]
pub enum MemoryRegistryError {
    /// A requested owner range overlaps an already reserved range.
    #[error(
        "memory range overlap: crate '{existing_crate}' [{existing_start}-{existing_end}]
conflicts with crate '{new_crate}' [{new_start}-{new_end}]"
    )]
    Overlap {
        /// Crate that already owns the conflicting range.
        existing_crate: String,
        /// First ID in the existing range.
        existing_start: u8,
        /// Last ID in the existing range.
        existing_end: u8,
        /// Crate requesting the new range.
        new_crate: String,
        /// First ID in the requested range.
        new_start: u8,
        /// Last ID in the requested range.
        new_end: u8,
    },

    /// The requested range has `start > end`.
    #[error("memory range is invalid: start={start} end={end}")]
    InvalidRange {
        /// First ID in the requested range.
        start: u8,
        /// Last ID in the requested range.
        end: u8,
    },

    /// The memory ID is already registered.
    #[error("memory id {0} is already registered; each memory id must be globally unique")]
    DuplicateId(u8),

    /// The stable key is declared more than once in one runtime snapshot.
    #[error(
        "memory stable key '{0}' is declared more than once; each stable key must be globally unique"
    )]
    DuplicateStableKey(String),

    /// The crate attempted to register an ID before reserving any range.
    #[error("memory id {id} has no reserved range for crate '{crate_name}'")]
    NoReservedRange {
        /// Crate attempting to register the ID.
        crate_name: String,
        /// Stable-memory ID being registered.
        id: u8,
    },

    /// The ID falls inside a range reserved by another crate.
    #[error(
        "memory id {id} reserved to crate '{owner}' [{owner_start}-{owner_end}], not '{crate_name}'"
    )]
    IdOwnedByOther {
        /// Crate attempting to register the ID.
        crate_name: String,
        /// Stable-memory ID being registered.
        id: u8,
        /// Crate that owns the range containing `id`.
        owner: String,
        /// First ID in the owning range.
        owner_start: u8,
        /// Last ID in the owning range.
        owner_end: u8,
    },

    /// The crate has reserved ranges, but none contain the requested ID.
    #[error("memory id {id} is outside reserved ranges for crate '{crate_name}'")]
    IdOutOfRange {
        /// Crate attempting to register the ID.
        crate_name: String,
        /// Stable-memory ID being registered.
        id: u8,
    },

    /// The ID is the unallocated-bucket sentinel and is not usable.
    #[error(
        "memory id {id} is the unallocated-bucket sentinel and is not a usable virtual memory id"
    )]
    ReservedInternalId {
        /// Invalid sentinel stable-memory ID.
        id: u8,
    },

    /// A requested range conflicts with a range recorded in the stable layout ledger.
    #[error(
        "memory range historical conflict: crate '{existing_crate}' [{existing_start}-{existing_end}]
conflicts with crate '{new_crate}' [{new_start}-{new_end}]"
    )]
    HistoricalRangeConflict {
        /// Crate that historically owns the conflicting range.
        existing_crate: String,
        /// First ID in the historical range.
        existing_start: u8,
        /// Last ID in the historical range.
        existing_end: u8,
        /// Crate requesting the new range.
        new_crate: String,
        /// First ID in the requested range.
        new_start: u8,
        /// Last ID in the requested range.
        new_end: u8,
    },

    /// A requested ID conflicts with an ID recorded in the stable layout ledger.
    #[error(
        "memory id {id} was historically registered to crate '{existing_crate}' label '{existing_label}', not crate '{new_crate}' label '{new_label}'"
    )]
    HistoricalIdConflict {
        /// Stable-memory ID being registered.
        id: u8,
        /// Crate that historically registered the ID.
        existing_crate: String,
        /// Historical label for the ID.
        existing_label: String,
        /// Crate requesting the ID now.
        new_crate: String,
        /// Requested label for the ID now.
        new_label: String,
        /// Requested stable key for the ID now.
        new_stable_key: String,
    },

    /// A requested stable key conflicts with a different historical ID.
    #[error(
        "memory stable key '{stable_key}' was historically registered to id {existing_id}, not id {new_id}"
    )]
    HistoricalStableKeyConflict {
        /// Stable key being registered.
        stable_key: String,
        /// Historical ID for the stable key.
        existing_id: u8,
        /// Requested ID for the stable key now.
        new_id: u8,
    },

    /// The stable key is not canonical.
    #[error("memory stable key '{stable_key}' is invalid: {reason}")]
    InvalidStableKey {
        /// Rejected stable key.
        stable_key: String,
        /// Human-readable reason for the rejection.
        reason: &'static str,
    },

    /// The schema metadata is not canonical.
    #[error("memory schema metadata is invalid for stable key '{stable_key}': {reason}")]
    InvalidSchemaMetadata {
        /// Stable key whose schema metadata was rejected.
        stable_key: String,
        /// Human-readable reason for the rejection.
        reason: &'static str,
    },

    /// The stable key namespace and memory ID range do not match.
    #[error(
        "memory stable key '{stable_key}' with id {id} violates namespace/range authority: {reason}"
    )]
    RangeAuthorityViolation {
        /// Stable key being registered.
        stable_key: String,
        /// Stable-memory ID being registered.
        id: u8,
        /// Human-readable reason for the rejection.
        reason: &'static str,
    },

    /// Registration was attempted after the bootstrap declaration snapshot was sealed.
    #[error(
        "memory registration after bootstrap is sealed is not allowed: {ranges} range(s), {registrations} registration(s)"
    )]
    RegistrationAfterBootstrap {
        /// Number of late range declarations.
        ranges: usize,
        /// Number of late memory ID declarations.
        registrations: usize,
    },

    /// A memory handle was requested before bootstrap validated the declaration snapshot.
    #[error("memory registry has not completed bootstrap validation")]
    RegistryNotBootstrapped,

    /// The persisted ABI ledger cannot be validated.
    #[error("memory layout ledger is corrupt: {reason}")]
    LedgerCorrupt {
        /// Human-readable corruption reason.
        reason: &'static str,
    },
}

//
// Internal global state (substrate-level, single-threaded)
//

thread_local! {
    static RESERVED_RANGES: RefCell<Vec<(String, MemoryRange)>> = const { RefCell::new(Vec::new()) };
    static REGISTRY: RefCell<BTreeMap<u8, MemoryRegistryEntry>> = const { RefCell::new(BTreeMap::new()) };

    // Deferred registrations (used before init)
    static PENDING_RANGES: RefCell<Vec<(String, u8, u8)>> = const { RefCell::new(Vec::new()) };
    static PENDING_REGISTRATIONS: RefCell<Vec<PendingRegistration>> = const { RefCell::new(Vec::new()) };
}

///
/// MemoryRegistry
///
/// Canonical substrate registry for stable memory IDs.
///
pub struct MemoryRegistry;

impl MemoryRegistry {
    /// Reserve the internal persisted layout ledger range and slot.
    pub(crate) fn reserve_internal_layout_ledger() -> Result<(), MemoryRegistryError> {
        Self::reserve_range(
            ledger::MEMORY_LAYOUT_LEDGER_OWNER,
            ledger::MEMORY_LAYOUT_RESERVED_MIN,
            ledger::MEMORY_LAYOUT_RESERVED_MAX,
        )?;

        if let Some(entry) = Self::get(ledger::MEMORY_LAYOUT_LEDGER_ID)
            && entry.crate_name == ledger::MEMORY_LAYOUT_LEDGER_OWNER
            && entry.label == ledger::MEMORY_LAYOUT_LEDGER_LABEL
            && entry.stable_key == ledger::MEMORY_LAYOUT_LEDGER_STABLE_KEY
        {
            return Ok(());
        }

        Self::register_with_key(
            ledger::MEMORY_LAYOUT_LEDGER_ID,
            ledger::MEMORY_LAYOUT_LEDGER_OWNER,
            ledger::MEMORY_LAYOUT_LEDGER_LABEL,
            ledger::MEMORY_LAYOUT_LEDGER_STABLE_KEY,
        )
    }

    /// Reserve an inclusive memory ID range for one crate.
    ///
    /// Exact duplicate reservations by the same crate are accepted so init and
    /// post-upgrade can share the same bootstrap path.
    pub fn reserve_range(crate_name: &str, start: u8, end: u8) -> Result<(), MemoryRegistryError> {
        if start > end {
            return Err(MemoryRegistryError::InvalidRange { start, end });
        }
        validate_range_excludes_reserved_internal_id(start, end)?;
        validate_range_excludes_layout_metadata(crate_name, start, end)?;

        let range = MemoryRange { start, end };
        let mut already_reserved = false;

        RESERVED_RANGES.with_borrow(|ranges| {
            for (existing_crate, existing_range) in ranges {
                if ranges_overlap(*existing_range, range) {
                    if existing_crate == crate_name
                        && existing_range.start == start
                        && existing_range.end == end
                    {
                        // Allow exact duplicate reservations for idempotent init.
                        already_reserved = true;
                        return Ok(());
                    }
                    return Err(MemoryRegistryError::Overlap {
                        existing_crate: existing_crate.clone(),
                        existing_start: existing_range.start,
                        existing_end: existing_range.end,
                        new_crate: crate_name.to_string(),
                        new_start: start,
                        new_end: end,
                    });
                }
            }

            Ok(())
        })?;

        ledger::record_range(crate_name, range)?;

        if already_reserved {
            return Ok(());
        }

        RESERVED_RANGES.with_borrow_mut(|ranges| {
            ranges.push((crate_name.to_string(), range));
        });

        Ok(())
    }

    /// Register one memory ID under an existing owner range.
    pub fn register(id: u8, crate_name: &str, label: &str) -> Result<(), MemoryRegistryError> {
        Self::register_with_key(
            id,
            crate_name,
            label,
            &fallback_stable_key(crate_name, label),
        )
    }

    /// Register one memory ID under an existing owner range using an explicit ABI key.
    pub fn register_with_key(
        id: u8,
        crate_name: &str,
        label: &str,
        stable_key: &str,
    ) -> Result<(), MemoryRegistryError> {
        Self::register_with_key_metadata(id, crate_name, label, stable_key, None, None)
    }

    /// Register one memory ID with explicit ABI key and optional schema metadata.
    pub fn register_with_key_metadata(
        id: u8,
        crate_name: &str,
        label: &str,
        stable_key: &str,
        schema_version: Option<u32>,
        schema_fingerprint: Option<&str>,
    ) -> Result<(), MemoryRegistryError> {
        validate_non_internal_id(id)?;
        validate_id_excludes_layout_metadata(crate_name, id)?;
        validate_registration_range(crate_name, id)?;
        validate_stable_key(stable_key)?;
        validate_schema_metadata(stable_key, schema_version, schema_fingerprint)?;
        validate_id_authority(id, crate_name, stable_key)?;

        REGISTRY.with_borrow(|reg| {
            if reg.contains_key(&id) {
                return Err(MemoryRegistryError::DuplicateId(id));
            }
            Ok(())
        })?;

        ledger::record_entry(
            id,
            crate_name,
            label,
            stable_key,
            schema_version,
            schema_fingerprint,
        )?;

        REGISTRY.with_borrow_mut(|reg| {
            reg.insert(
                id,
                MemoryRegistryEntry {
                    crate_name: crate_name.to_string(),
                    label: label.to_string(),
                    stable_key: stable_key.to_string(),
                    schema_version,
                    schema_fingerprint: schema_fingerprint.map(str::to_string),
                },
            );
        });

        Ok(())
    }

    /// Export all registered entries (canonical snapshot).
    #[must_use]
    pub fn export() -> Vec<(u8, MemoryRegistryEntry)> {
        REGISTRY.with_borrow(|reg| reg.iter().map(|(k, v)| (*k, v.clone())).collect())
    }

    /// Export all reserved ranges.
    #[must_use]
    pub fn export_ranges() -> Vec<(String, MemoryRange)> {
        RESERVED_RANGES.with_borrow(std::clone::Clone::clone)
    }

    /// Export all reserved ranges with explicit owners.
    #[must_use]
    pub fn export_range_entries() -> Vec<MemoryRangeEntry> {
        RESERVED_RANGES.with_borrow(|ranges| {
            ranges
                .iter()
                .map(|(owner, range)| MemoryRangeEntry {
                    owner: owner.clone(),
                    range: *range,
                })
                .collect()
        })
    }

    /// Export registry entries grouped by reserved range.
    #[must_use]
    pub fn export_ids_by_range() -> Vec<MemoryRangeSnapshot> {
        let mut ranges = RESERVED_RANGES.with_borrow(std::clone::Clone::clone);
        let entries = REGISTRY.with_borrow(std::clone::Clone::clone);

        ranges.sort_by_key(|(_, range)| range.start);

        ranges
            .into_iter()
            .map(|(owner, range)| {
                let entries = entries
                    .iter()
                    .filter(|(id, _)| range.contains(**id))
                    .map(|(id, entry)| (*id, entry.clone()))
                    .collect();

                MemoryRangeSnapshot {
                    owner,
                    range,
                    entries,
                }
            })
            .collect()
    }

    /// Retrieve a single registry entry.
    #[must_use]
    pub fn get(id: u8) -> Option<MemoryRegistryEntry> {
        REGISTRY.with_borrow(|reg| reg.get(&id).cloned())
    }

    /// Export all ranges ever recorded in the stable memory layout ledger.
    #[must_use]
    pub fn export_historical_ranges() -> Vec<(String, MemoryRange)> {
        ledger::export_ranges()
    }

    /// Export canonical allocation authorities recorded in the stable memory layout ledger.
    #[must_use]
    pub fn export_historical_authorities() -> Vec<MemoryRangeAuthority> {
        ledger::export_authorities()
    }

    /// Fallibly export all ranges ever recorded in the stable memory layout ledger.
    pub fn try_export_historical_ranges() -> Result<Vec<(String, MemoryRange)>, MemoryRegistryError>
    {
        ledger::try_export_ranges()
    }

    /// Fallibly export canonical allocation authorities recorded in the stable memory layout ledger.
    pub fn try_export_historical_authorities()
    -> Result<Vec<MemoryRangeAuthority>, MemoryRegistryError> {
        ledger::try_export_authorities()
    }

    /// Export all memory IDs ever recorded in the stable memory layout ledger.
    #[must_use]
    pub fn export_historical() -> Vec<(u8, MemoryRegistryEntry)> {
        ledger::export_entries()
    }

    /// Fallibly export all memory IDs ever recorded in the stable memory layout ledger.
    pub fn try_export_historical() -> Result<Vec<(u8, MemoryRegistryEntry)>, MemoryRegistryError> {
        ledger::try_export_entries()
    }
}

//
// Deferred registration helpers (used before runtime init)
//

/// Queue a range reservation for deterministic application during bootstrap.
#[doc(hidden)]
pub fn defer_reserve_range(
    crate_name: &str,
    start: u8,
    end: u8,
) -> Result<(), MemoryRegistryError> {
    if start > end {
        return Err(MemoryRegistryError::InvalidRange { start, end });
    }
    validate_range_excludes_reserved_internal_id(start, end)?;
    validate_range_excludes_layout_metadata(crate_name, start, end)?;

    // Queue range reservations for runtime init to apply deterministically.
    PENDING_RANGES.with_borrow_mut(|ranges| {
        ranges.push((crate_name.to_string(), start, end));
    });

    Ok(())
}

/// Queue an ID registration for deterministic application during bootstrap.
#[doc(hidden)]
pub fn defer_register(id: u8, crate_name: &str, label: &str) -> Result<(), MemoryRegistryError> {
    defer_register_with_key(
        id,
        crate_name,
        label,
        &fallback_stable_key(crate_name, label),
    )
}

/// Queue an ID registration with an explicit ABI-stable key.
#[doc(hidden)]
pub fn defer_register_with_key(
    id: u8,
    crate_name: &str,
    label: &str,
    stable_key: &str,
) -> Result<(), MemoryRegistryError> {
    defer_register_with_key_metadata(id, crate_name, label, stable_key, None, None)
}

/// Queue an explicit-key ID registration with optional schema metadata.
#[doc(hidden)]
pub fn defer_register_with_key_metadata(
    id: u8,
    crate_name: &str,
    label: &str,
    stable_key: &str,
    schema_version: Option<u32>,
    schema_fingerprint: Option<&str>,
) -> Result<(), MemoryRegistryError> {
    validate_non_internal_id(id)?;
    validate_id_excludes_layout_metadata(crate_name, id)?;
    validate_stable_key(stable_key)?;
    validate_schema_metadata(stable_key, schema_version, schema_fingerprint)?;
    validate_id_authority(id, crate_name, stable_key)?;

    // Queue ID registrations for runtime init to apply after ranges are reserved.
    PENDING_REGISTRATIONS.with_borrow_mut(|regs| {
        regs.push(PendingRegistration {
            id,
            crate_name: crate_name.to_string(),
            label: label.to_string(),
            stable_key: stable_key.to_string(),
            schema_version,
            schema_fingerprint: schema_fingerprint.map(str::to_string),
        });
    });

    Ok(())
}

/// Drain all queued range reservations in insertion order.
#[must_use]
pub(crate) fn drain_pending_ranges() -> Vec<(String, u8, u8)> {
    PENDING_RANGES.with_borrow_mut(std::mem::take)
}

/// Drain all queued ID registrations in insertion order.
#[must_use]
pub(crate) fn drain_pending_registrations() -> Vec<PendingRegistration> {
    PENDING_REGISTRATIONS.with_borrow_mut(std::mem::take)
}

//
// Test-only helpers
//

#[cfg(test)]
/// Clear registry and pending queues for isolated unit tests.
pub fn reset_for_tests() {
    reset_runtime_for_tests();
    ledger::reset_for_tests();
    crate::runtime::registry::reset_initialized_for_tests();
}

#[cfg(test)]
fn reset_runtime_for_tests() {
    RESERVED_RANGES.with_borrow_mut(Vec::clear);
    REGISTRY.with_borrow_mut(BTreeMap::clear);
    PENDING_RANGES.with_borrow_mut(Vec::clear);
    PENDING_REGISTRATIONS.with_borrow_mut(Vec::clear);
}

//
// Internal helpers
//

const fn ranges_overlap(a: MemoryRange, b: MemoryRange) -> bool {
    a.start <= b.end && b.start <= a.end
}

const INTERNAL_RESERVED_MEMORY_ID: u8 = u8::MAX;
const CANIC_FRAMEWORK_MAX_ID: u8 = 99;
const APPLICATION_MIN_ID: u8 = 100;

const MEMORY_LAYOUT_RESERVED_RANGE: MemoryRange = MemoryRange {
    start: ledger::MEMORY_LAYOUT_RESERVED_MIN,
    end: ledger::MEMORY_LAYOUT_RESERVED_MAX,
};

const fn validate_non_internal_id(id: u8) -> Result<(), MemoryRegistryError> {
    if id == INTERNAL_RESERVED_MEMORY_ID {
        return Err(MemoryRegistryError::ReservedInternalId { id });
    }
    Ok(())
}

fn fallback_stable_key(crate_name: &str, label: &str) -> String {
    format!(
        "legacy.{}.{}.v1",
        canonical_segment(crate_name),
        canonical_segment(label)
    )
}

fn validate_stable_key(stable_key: &str) -> Result<(), MemoryRegistryError> {
    if stable_key.is_empty() {
        return invalid_stable_key(stable_key, "must not be empty");
    }
    if stable_key.len() > 128 {
        return invalid_stable_key(stable_key, "must be at most 128 bytes");
    }
    if !stable_key.is_ascii() {
        return invalid_stable_key(stable_key, "must be ASCII");
    }
    if stable_key.bytes().any(|b| b.is_ascii_uppercase()) {
        return invalid_stable_key(stable_key, "must be lowercase");
    }
    if stable_key.contains(char::is_whitespace) {
        return invalid_stable_key(stable_key, "must not contain whitespace");
    }
    if stable_key.contains('/') || stable_key.contains('-') {
        return invalid_stable_key(stable_key, "must not contain slashes or hyphens");
    }
    if stable_key.starts_with('.') || stable_key.ends_with('.') {
        return invalid_stable_key(stable_key, "must not start or end with a dot");
    }

    let Some(version_index) = stable_key.rfind(".v") else {
        return invalid_stable_key(stable_key, "must end with .vN");
    };
    let version = &stable_key[version_index + 2..];
    if version.is_empty()
        || version.starts_with('0')
        || !version.bytes().all(|b| b.is_ascii_digit())
    {
        return invalid_stable_key(stable_key, "version suffix must be nonzero .vN");
    }

    let prefix = &stable_key[..version_index];
    if prefix.is_empty() {
        return invalid_stable_key(
            stable_key,
            "must contain at least one segment before version",
        );
    }

    for segment in prefix.split('.') {
        validate_stable_key_segment(stable_key, segment)?;
    }

    Ok(())
}

fn validate_stable_key_segment(stable_key: &str, segment: &str) -> Result<(), MemoryRegistryError> {
    if segment.is_empty() {
        return invalid_stable_key(stable_key, "must not contain empty segments");
    }
    let mut bytes = segment.bytes();
    let Some(first) = bytes.next() else {
        return invalid_stable_key(stable_key, "must not contain empty segments");
    };
    if !first.is_ascii_lowercase() {
        return invalid_stable_key(stable_key, "segments must start with a lowercase letter");
    }
    if !bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') {
        return invalid_stable_key(
            stable_key,
            "segments may contain only lowercase letters, digits, and underscores",
        );
    }
    Ok(())
}

fn validate_schema_metadata(
    stable_key: &str,
    schema_version: Option<u32>,
    schema_fingerprint: Option<&str>,
) -> Result<(), MemoryRegistryError> {
    if schema_version == Some(0) {
        return Err(MemoryRegistryError::InvalidSchemaMetadata {
            stable_key: stable_key.to_string(),
            reason: "schema_version must be greater than zero when present",
        });
    }

    let Some(fingerprint) = schema_fingerprint else {
        return Ok(());
    };

    if fingerprint.is_empty() {
        return Err(MemoryRegistryError::InvalidSchemaMetadata {
            stable_key: stable_key.to_string(),
            reason: "schema_fingerprint must not be empty when present",
        });
    }
    if fingerprint.len() > 256 {
        return Err(MemoryRegistryError::InvalidSchemaMetadata {
            stable_key: stable_key.to_string(),
            reason: "schema_fingerprint must be at most 256 bytes",
        });
    }
    if !fingerprint.is_ascii() {
        return Err(MemoryRegistryError::InvalidSchemaMetadata {
            stable_key: stable_key.to_string(),
            reason: "schema_fingerprint must be ASCII",
        });
    }
    if fingerprint.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(MemoryRegistryError::InvalidSchemaMetadata {
            stable_key: stable_key.to_string(),
            reason: "schema_fingerprint must not contain ASCII control characters",
        });
    }

    Ok(())
}

fn invalid_stable_key<T>(stable_key: &str, reason: &'static str) -> Result<T, MemoryRegistryError> {
    Err(MemoryRegistryError::InvalidStableKey {
        stable_key: stable_key.to_string(),
        reason,
    })
}

fn validate_id_authority(
    id: u8,
    crate_name: &str,
    stable_key: &str,
) -> Result<(), MemoryRegistryError> {
    if stable_key.starts_with("canic.") {
        if !crate_name.starts_with("canic") {
            return Err(MemoryRegistryError::RangeAuthorityViolation {
                stable_key: stable_key.to_string(),
                id,
                reason: "canic.* keys may only be declared by Canic framework crates",
            });
        }

        if id <= CANIC_FRAMEWORK_MAX_ID {
            return Ok(());
        }

        return Err(MemoryRegistryError::RangeAuthorityViolation {
            stable_key: stable_key.to_string(),
            id,
            reason: "canic.* keys must use ids 0-99",
        });
    }

    if (APPLICATION_MIN_ID..INTERNAL_RESERVED_MEMORY_ID).contains(&id) {
        return Ok(());
    }

    Err(MemoryRegistryError::RangeAuthorityViolation {
        stable_key: stable_key.to_string(),
        id,
        reason: "application keys must use ids 100-254",
    })
}

fn canonical_segment(value: &str) -> String {
    let mut out = String::new();
    let mut last_was_underscore = false;

    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            out.push(ch);
            last_was_underscore = false;
        } else if !last_was_underscore {
            out.push('_');
            last_was_underscore = true;
        }
    }

    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        return "unnamed".to_string();
    }
    if trimmed.as_bytes()[0].is_ascii_digit() {
        return format!("n_{trimmed}");
    }
    trimmed.to_string()
}

const fn validate_range_excludes_reserved_internal_id(
    _start: u8,
    end: u8,
) -> Result<(), MemoryRegistryError> {
    if end == INTERNAL_RESERVED_MEMORY_ID {
        return Err(MemoryRegistryError::ReservedInternalId {
            id: INTERNAL_RESERVED_MEMORY_ID,
        });
    }
    Ok(())
}

fn validate_range_excludes_layout_metadata(
    crate_name: &str,
    start: u8,
    end: u8,
) -> Result<(), MemoryRegistryError> {
    let requested = MemoryRange { start, end };
    if !ranges_overlap(requested, MEMORY_LAYOUT_RESERVED_RANGE) {
        return Ok(());
    }

    if crate_name == ledger::MEMORY_LAYOUT_LEDGER_OWNER
        && start == ledger::MEMORY_LAYOUT_RESERVED_MIN
        && end == ledger::MEMORY_LAYOUT_RESERVED_MAX
    {
        return Ok(());
    }

    Err(MemoryRegistryError::HistoricalRangeConflict {
        existing_crate: ledger::MEMORY_LAYOUT_LEDGER_OWNER.to_string(),
        existing_start: ledger::MEMORY_LAYOUT_RESERVED_MIN,
        existing_end: ledger::MEMORY_LAYOUT_RESERVED_MAX,
        new_crate: crate_name.to_string(),
        new_start: start,
        new_end: end,
    })
}

fn validate_id_excludes_layout_metadata(
    crate_name: &str,
    id: u8,
) -> Result<(), MemoryRegistryError> {
    if !MEMORY_LAYOUT_RESERVED_RANGE.contains(id)
        || crate_name == ledger::MEMORY_LAYOUT_LEDGER_OWNER
    {
        return Ok(());
    }

    Err(MemoryRegistryError::IdOwnedByOther {
        crate_name: crate_name.to_string(),
        id,
        owner: ledger::MEMORY_LAYOUT_LEDGER_OWNER.to_string(),
        owner_start: ledger::MEMORY_LAYOUT_RESERVED_MIN,
        owner_end: ledger::MEMORY_LAYOUT_RESERVED_MAX,
    })
}

fn validate_registration_range(crate_name: &str, id: u8) -> Result<(), MemoryRegistryError> {
    let mut has_range = false;
    let mut owner_match = false;
    let mut owner_for_id: Option<(String, MemoryRange)> = None;

    RESERVED_RANGES.with_borrow(|ranges| {
        for (owner, range) in ranges {
            if owner == crate_name {
                has_range = true;
                if range.contains(id) {
                    owner_match = true;
                    break;
                }
            }

            if owner_for_id.is_none() && range.contains(id) {
                owner_for_id = Some((owner.clone(), *range));
            }
        }
    });

    if owner_match {
        return Ok(());
    }

    if !has_range {
        return Err(MemoryRegistryError::NoReservedRange {
            crate_name: crate_name.to_string(),
            id,
        });
    }

    if let Some((owner, range)) = owner_for_id {
        return Err(MemoryRegistryError::IdOwnedByOther {
            crate_name: crate_name.to_string(),
            id,
            owner,
            owner_start: range.start,
            owner_end: range.end,
        });
    }

    Err(MemoryRegistryError::IdOutOfRange {
        crate_name: crate_name.to_string(),
        id,
    })
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_in_range() {
        reset_for_tests();

        MemoryRegistry::reserve_range("crate_a", 100, 102).expect("reserve range");
        MemoryRegistry::register(101, "crate_a", "slot").expect("register in range");
    }

    #[test]
    fn rejects_unreserved() {
        reset_for_tests();

        let err = MemoryRegistry::register(100, "crate_a", "slot").expect_err("missing range");
        assert!(matches!(err, MemoryRegistryError::NoReservedRange { .. }));
    }

    #[test]
    fn rejects_other_owner() {
        reset_for_tests();

        MemoryRegistry::reserve_range("crate_a", 100, 102).expect("reserve range A");
        MemoryRegistry::reserve_range("crate_b", 110, 112).expect("reserve range B");

        let err = MemoryRegistry::register(101, "crate_b", "slot").expect_err("owned by other");
        assert!(matches!(err, MemoryRegistryError::IdOwnedByOther { .. }));
    }

    #[test]
    fn export_ids_by_range_groups_entries() {
        reset_for_tests();

        MemoryRegistry::reserve_range("crate_a", 100, 102).expect("reserve range A");
        MemoryRegistry::reserve_range("crate_b", 110, 112).expect("reserve range B");
        MemoryRegistry::register(100, "crate_a", "a100").expect("register a100");
        MemoryRegistry::register(111, "crate_b", "b111").expect("register b111");

        let snapshots = MemoryRegistry::export_ids_by_range();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].entries.len(), 1);
        assert_eq!(snapshots[1].entries.len(), 1);
    }

    #[test]
    fn historical_range_conflict_survives_runtime_reset() {
        reset_for_tests();

        MemoryRegistry::reserve_range("crate_a", 100, 102).expect("reserve range A");
        reset_runtime_for_tests();

        let err = MemoryRegistry::reserve_range("crate_b", 101, 103)
            .expect_err("historical range overlap should fail");
        assert!(matches!(
            err,
            MemoryRegistryError::HistoricalRangeConflict { .. }
        ));
    }

    #[test]
    fn historical_id_conflict_survives_runtime_reset() {
        reset_for_tests();

        MemoryRegistry::reserve_range("crate_a", 100, 102).expect("reserve range A");
        MemoryRegistry::register_with_key_metadata(
            100,
            "crate_a",
            "slot",
            "app.crate_a.slot.v1",
            Some(1),
            Some("sha256:aaa"),
        )
        .expect("register slot");
        reset_runtime_for_tests();

        MemoryRegistry::reserve_range("crate_a", 100, 102).expect("reserve range A again");
        let err =
            MemoryRegistry::register_with_key(100, "crate_a", "other", "app.crate_a.other.v1")
                .expect_err("historical id label drift should fail");
        assert!(matches!(
            err,
            MemoryRegistryError::HistoricalIdConflict { .. }
        ));
    }

    #[test]
    fn stable_key_allows_owner_and_label_metadata_drift() {
        reset_for_tests();

        MemoryRegistry::reserve_range("crate_a", 100, 102).expect("reserve range A");
        MemoryRegistry::register_with_key(100, "crate_a", "slot", "app.crate_a.slot.v1")
            .expect("register slot");
        reset_runtime_for_tests();

        MemoryRegistry::reserve_range("crate_renamed", 100, 102).expect("reserve range A again");
        MemoryRegistry::register_with_key_metadata(
            100,
            "crate_renamed",
            "SlotRenamed",
            "app.crate_a.slot.v1",
            Some(2),
            Some("sha256:bbb"),
        )
        .expect("stable key should survive owner/label drift");

        let entry = MemoryRegistry::get(100).expect("entry should exist");
        assert_eq!(entry.crate_name, "crate_renamed");
        assert_eq!(entry.label, "SlotRenamed");
        assert_eq!(entry.stable_key, "app.crate_a.slot.v1");
        assert_eq!(entry.schema_version, Some(2));
        assert_eq!(entry.schema_fingerprint.as_deref(), Some("sha256:bbb"));
    }

    #[test]
    fn rejects_invalid_schema_metadata() {
        reset_for_tests();

        MemoryRegistry::reserve_range("crate_a", 100, 102).expect("reserve range A");
        let err = MemoryRegistry::register_with_key_metadata(
            100,
            "crate_a",
            "slot",
            "app.crate_a.slot.v1",
            Some(0),
            None,
        )
        .expect_err("zero schema version should fail");
        assert!(matches!(
            err,
            MemoryRegistryError::InvalidSchemaMetadata { .. }
        ));

        let err = MemoryRegistry::register_with_key_metadata(
            100,
            "crate_a",
            "slot",
            "app.crate_a.slot.v1",
            Some(1),
            Some("fingerprint\nwith-control"),
        )
        .expect_err("control characters should fail");
        assert!(matches!(
            err,
            MemoryRegistryError::InvalidSchemaMetadata { .. }
        ));
    }

    #[test]
    fn rejects_non_canic_key_below_application_range() {
        reset_for_tests();

        MemoryRegistry::reserve_range("crate_a", 1, 4).expect("reserve low framework range");
        let err = MemoryRegistry::register_with_key(1, "crate_a", "slot", "app.crate_a.slot.v1")
            .expect_err("application key below 100 should fail");
        assert!(matches!(
            err,
            MemoryRegistryError::RangeAuthorityViolation { .. }
        ));
    }

    #[test]
    fn rejects_canic_key_above_framework_range() {
        reset_for_tests();

        MemoryRegistry::reserve_range("canic-core", 100, 102).expect("reserve app range");
        let err =
            MemoryRegistry::register_with_key(100, "canic-core", "slot", "canic.core.slot.v1")
                .expect_err("canic key above 99 should fail");
        assert!(matches!(
            err,
            MemoryRegistryError::RangeAuthorityViolation { .. }
        ));
    }

    #[test]
    fn rejects_canic_namespace_from_non_framework_crate() {
        reset_for_tests();

        MemoryRegistry::reserve_range("crate_a", 11, 12).expect("reserve framework range");
        let err = MemoryRegistry::register_with_key(11, "crate_a", "slot", "canic.core.slot.v1")
            .expect_err("non-framework crate must not claim canic namespace");
        assert!(matches!(
            err,
            MemoryRegistryError::RangeAuthorityViolation { .. }
        ));
    }

    #[test]
    fn rejects_non_canonical_stable_key() {
        reset_for_tests();

        MemoryRegistry::reserve_range("crate_a", 100, 102).expect("reserve range A");
        let err = MemoryRegistry::register_with_key(100, "crate_a", "slot", "App.Crate.Slot.v1")
            .expect_err("uppercase stable key should fail");
        assert!(matches!(err, MemoryRegistryError::InvalidStableKey { .. }));
    }

    #[test]
    fn internal_layout_ledger_uses_only_id_zero_self_record() {
        reset_for_tests();

        MemoryRegistry::reserve_internal_layout_ledger().expect("reserve ledger");

        assert_eq!(
            MemoryRegistry::export_ranges(),
            vec![(
                ledger::MEMORY_LAYOUT_LEDGER_OWNER.to_string(),
                MemoryRange { start: 0, end: 0 },
            )]
        );
        assert_eq!(
            MemoryRegistry::get(0).map(|entry| entry.stable_key),
            Some(ledger::MEMORY_LAYOUT_LEDGER_STABLE_KEY.to_string())
        );
        for id in 1..=4 {
            assert!(MemoryRegistry::get(id).is_none());
        }
    }

    #[test]
    fn historical_stable_key_conflict_survives_runtime_reset() {
        reset_for_tests();

        MemoryRegistry::reserve_range("crate_a", 100, 102).expect("reserve range A");
        MemoryRegistry::register_with_key(100, "crate_a", "slot", "app.crate_a.slot.v1")
            .expect("register slot");
        reset_runtime_for_tests();

        MemoryRegistry::reserve_range("crate_a", 100, 102).expect("reserve range A again");
        let err = MemoryRegistry::register_with_key(101, "crate_a", "slot", "app.crate_a.slot.v1")
            .expect_err("stable key must not move to another ID");
        assert!(matches!(
            err,
            MemoryRegistryError::HistoricalStableKeyConflict { .. }
        ));
    }

    #[test]
    fn rejects_internal_reserved_id_on_register() {
        reset_for_tests();

        MemoryRegistry::reserve_range("crate_a", 5, 254).expect("reserve range");
        let err = MemoryRegistry::register(u8::MAX, "crate_a", "slot")
            .expect_err("reserved id should be rejected");
        assert!(matches!(
            err,
            MemoryRegistryError::ReservedInternalId { .. }
        ));
    }

    #[test]
    fn rejects_layout_metadata_range_reservation() {
        reset_for_tests();

        let err = MemoryRegistry::reserve_range("crate_a", 0, 4)
            .expect_err("layout metadata range must not be reservable by applications");
        assert!(matches!(
            err,
            MemoryRegistryError::HistoricalRangeConflict { .. }
        ));
    }

    #[test]
    fn rejects_layout_metadata_range_overlap() {
        reset_for_tests();

        let err = MemoryRegistry::reserve_range("crate_a", 0, 8)
            .expect_err("layout metadata overlap must not be reservable by applications");
        assert!(matches!(
            err,
            MemoryRegistryError::HistoricalRangeConflict { .. }
        ));
    }

    #[test]
    fn rejects_layout_metadata_range_on_deferred_reservation() {
        reset_for_tests();

        let err = defer_reserve_range("crate_a", 0, 4)
            .expect_err("layout metadata range should fail before init");
        assert!(matches!(
            err,
            MemoryRegistryError::HistoricalRangeConflict { .. }
        ));
    }

    #[test]
    fn rejects_layout_metadata_id_on_register() {
        reset_for_tests();

        MemoryRegistry::reserve_range("crate_a", 100, 108).expect("reserve range");
        let err = MemoryRegistry::register(0, "crate_a", "slot")
            .expect_err("layout metadata ID must not be registrable by applications");
        assert!(matches!(err, MemoryRegistryError::IdOwnedByOther { .. }));
    }

    #[test]
    fn rejects_layout_metadata_id_on_deferred_register() {
        reset_for_tests();

        let err = defer_register(0, "crate_a", "slot")
            .expect_err("layout metadata ID should fail before init");
        assert!(matches!(err, MemoryRegistryError::IdOwnedByOther { .. }));
    }

    #[test]
    fn rejects_internal_reserved_id_on_range_reservation() {
        reset_for_tests();

        let err = MemoryRegistry::reserve_range("crate_a", 250, u8::MAX)
            .expect_err("reserved internal id must not be reservable");
        assert!(matches!(
            err,
            MemoryRegistryError::ReservedInternalId { .. }
        ));
    }

    #[test]
    fn rejects_internal_reserved_id_on_deferred_register() {
        reset_for_tests();

        let err = defer_register(u8::MAX, "crate_a", "slot")
            .expect_err("reserved id should fail before init");
        assert!(matches!(
            err,
            MemoryRegistryError::ReservedInternalId { .. }
        ));
    }

    #[test]
    fn rejects_internal_reserved_id_on_deferred_range_reservation() {
        reset_for_tests();

        let err = defer_reserve_range("crate_a", 240, u8::MAX)
            .expect_err("reserved id should fail before init");
        assert!(matches!(
            err,
            MemoryRegistryError::ReservedInternalId { .. }
        ));
    }
}
