use super::{ledger, policy};
use ic_memory::{AllocationDeclaration, DeclarationSnapshotError, SchemaMetadata};
#[cfg(test)]
use std::cell::RefCell;
use std::sync::Mutex;
use thiserror::Error as ThisError;

///
/// PendingRegistration
///
/// One stable-memory declaration collected before bootstrap validation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PendingRegistration {
    crate_name: String,
    declaration: AllocationDeclaration,
}

impl PendingRegistration {
    fn from_parts(
        id: u8,
        crate_name: &str,
        label: &str,
        stable_key: &str,
        schema_version: Option<u32>,
        schema_fingerprint: Option<&str>,
    ) -> Result<Self, MemoryRegistryError> {
        let schema = SchemaMetadata::new(schema_version, schema_fingerprint.map(str::to_string))
            .map_err(|_| MemoryRegistryError::InvalidDeclaration {
                stable_key: stable_key.to_string(),
                reason: "schema metadata rejected by ic-memory",
            })?;
        let declaration =
            AllocationDeclaration::memory_manager_with_schema(stable_key, id, label, schema)
                .map_err(|err| memory_registry_error_from_declaration_error(err, stable_key))?;
        policy::validate_stable_key_authority(id, crate_name, stable_key)?;

        Ok(Self {
            crate_name: crate_name.to_string(),
            declaration,
        })
    }

    pub(crate) fn internal_layout_ledger() -> Result<Self, MemoryRegistryError> {
        Self::from_parts(
            ledger::MEMORY_LAYOUT_LEDGER_ID,
            ledger::MEMORY_LAYOUT_LEDGER_OWNER,
            ledger::MEMORY_LAYOUT_LEDGER_LABEL,
            ledger::MEMORY_LAYOUT_LEDGER_STABLE_KEY,
            None,
            None,
        )
    }

    pub(crate) fn crate_name(&self) -> &str {
        &self.crate_name
    }

    pub(crate) const fn declaration(&self) -> &AllocationDeclaration {
        &self.declaration
    }
}

///
/// MemoryRegistryError
///
/// Errors returned when a memory ID declaration is invalid.

#[derive(Debug, ThisError)]
pub enum MemoryRegistryError {
    /// A declaration was rejected before or during `ic-memory` validation.
    #[error("memory declaration rejected for stable key '{stable_key}': {reason}")]
    InvalidDeclaration {
        stable_key: String,
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
        "memory registration after bootstrap is sealed is not allowed: {registrations} registration(s)"
    )]
    RegistrationAfterBootstrap {
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

#[cfg(test)]
thread_local! {
    static PENDING_REGISTRATIONS: RefCell<Vec<PendingRegistration>> = const { RefCell::new(Vec::new()) };
}

static STATIC_DECLARATIONS: Mutex<Vec<PendingRegistration>> = Mutex::new(Vec::new());

/// Declare a macro-owned stable-memory slot with an explicit ABI-stable key.
#[doc(hidden)]
pub fn declare_memory_slot_with_key(
    id: u8,
    crate_name: &str,
    label: &str,
    stable_key: &str,
) -> Result<(), MemoryRegistryError> {
    declare_memory_slot_with_key_metadata(id, crate_name, label, stable_key, None, None)
}

/// Declare a macro-owned stable-memory slot with optional schema metadata.
#[doc(hidden)]
pub fn declare_memory_slot_with_key_metadata(
    id: u8,
    crate_name: &str,
    label: &str,
    stable_key: &str,
    schema_version: Option<u32>,
    schema_fingerprint: Option<&str>,
) -> Result<(), MemoryRegistryError> {
    let registration = PendingRegistration::from_parts(
        id,
        crate_name,
        label,
        stable_key,
        schema_version,
        schema_fingerprint,
    )?;

    STATIC_DECLARATIONS
        .lock()
        .expect("static memory declaration queue poisoned")
        .push(registration);

    Ok(())
}

#[cfg(test)]
pub fn defer_register_with_key(
    id: u8,
    crate_name: &str,
    label: &str,
    stable_key: &str,
) -> Result<(), MemoryRegistryError> {
    defer_register_with_key_metadata(id, crate_name, label, stable_key, None, None)
}

#[cfg(test)]
pub fn defer_register_with_key_metadata(
    id: u8,
    crate_name: &str,
    label: &str,
    stable_key: &str,
    schema_version: Option<u32>,
    schema_fingerprint: Option<&str>,
) -> Result<(), MemoryRegistryError> {
    let registration = PendingRegistration::from_parts(
        id,
        crate_name,
        label,
        stable_key,
        schema_version,
        schema_fingerprint,
    )?;

    PENDING_REGISTRATIONS.with_borrow_mut(|regs| {
        regs.push(registration);
    });

    Ok(())
}

/// Snapshot macro-owned stable-memory declarations.
#[must_use]
pub(crate) fn static_declarations() -> Vec<PendingRegistration> {
    STATIC_DECLARATIONS
        .lock()
        .expect("static memory declaration queue poisoned")
        .clone()
}

#[cfg(test)]
pub(crate) fn drain_pending_registrations() -> Vec<PendingRegistration> {
    PENDING_REGISTRATIONS.with_borrow_mut(std::mem::take)
}

#[cfg(test)]
/// Clear registry and pending queues for isolated unit tests.
pub fn reset_for_tests() {
    reset_runtime_for_tests();
    ledger::reset_for_tests();
    super::runtime::registry::reset_initialized_for_tests();
}

#[cfg(test)]
fn reset_runtime_for_tests() {
    PENDING_REGISTRATIONS.with_borrow_mut(Vec::clear);
}

pub(super) fn memory_registry_error_from_declaration_error(
    err: DeclarationSnapshotError,
    stable_key: &str,
) -> MemoryRegistryError {
    let (stable_key, reason) = if let DeclarationSnapshotError::Key(err) = err {
        (err.stable_key, err.reason)
    } else {
        (stable_key.to_string(), "declaration rejected by ic-memory")
    };

    MemoryRegistryError::InvalidDeclaration { stable_key, reason }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defers_valid_application_declaration() {
        reset_for_tests();

        defer_register_with_key(100, "crate_a", "slot", "app.crate_a.slot.v1")
            .expect("register valid app slot");

        let registrations = drain_pending_registrations();
        assert_eq!(registrations.len(), 1);
        assert_eq!(
            registrations[0].declaration().stable_key().as_str(),
            "app.crate_a.slot.v1"
        );
    }

    #[test]
    fn rejects_ic_memory_key_from_non_ic_memory_owner() {
        reset_for_tests();

        let err = defer_register_with_key(10, "canic-core", "slot", "ic_memory.future.v1")
            .expect_err("only ic-memory may declare ic_memory keys");
        assert!(matches!(
            err,
            MemoryRegistryError::RangeAuthorityViolation { .. }
        ));
    }
}
