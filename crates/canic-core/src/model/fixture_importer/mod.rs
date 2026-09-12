//! Module: model::fixture_importer
//!
//! Responsibility: own one heap registration and exact in-flight consumer lease.
//! Does not own: application progress, database writes, transport or timer scheduling.
//! Boundary: the application owns the durable cursor; the lease only serializes delivery.

use crate::dto::fixture_provisioning::{
    FixtureAssignment, FixtureImportError, FixtureImportProgress, FixtureTargetBinding,
};
use std::cell::RefCell;

/// Synchronous application participant for bounded import and validation steps.
///
/// Register once from the existing synchronous lifecycle participant, after restoring
/// the database. Mutating methods must commit rows and their checkpoint in one message.
/// Canic traps a returned error or invalid postcondition to roll back that message.
/// Each validation step must inspect bounded stored data; completion must not rescan it.
pub trait FixtureImporter: Sync {
    /// Read the durable checkpoint without changing it; `None` means not begun.
    fn progress(
        &self,
        assignment: &FixtureAssignment,
    ) -> Result<Option<FixtureImportProgress>, FixtureImportError>;
    /// Initialize an absent checkpoint for this exact assignment without wiping existing data.
    fn begin(&self, assignment: &FixtureAssignment) -> Result<(), FixtureImportError>;
    /// Apply one verified chunk and advance the same durable checkpoint exactly once.
    fn apply_chunk(
        &self,
        assignment: &FixtureAssignment,
        index: u32,
        bytes: &[u8],
    ) -> Result<(), FixtureImportError>;
    /// Validate a bounded portion of stored data and eventually commit the exact receipt.
    fn validate_step(&self, assignment: &FixtureAssignment) -> Result<(), FixtureImportError>;
}

/// Exact heap attempt identity, including installation authority for late cleanup fencing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureImportLease {
    generation: u64,
    binding: Box<FixtureTargetBinding>,
}

#[derive(Default)]
struct Registry {
    importer: Option<&'static dyn FixtureImporter>,
    generation: u64,
    active: Option<FixtureImportLease>,
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
}

/// One application importer per heap; restart reconstructs registration through lifecycle.
pub struct FixtureImporterRegistry;

impl FixtureImporterRegistry {
    /// Read the real heap lease for controlled interruption qualification.
    #[cfg(feature = "internal-test-fixtures")]
    pub fn fetch_in_flight() -> bool {
        REGISTRY.with_borrow(|registry| registry.active.is_some())
    }

    pub fn register(importer: &'static dyn FixtureImporter) -> Result<(), FixtureImportError> {
        REGISTRY.with_borrow_mut(|registry| {
            if registry.importer.is_some() {
                return Err(FixtureImportError::Registration);
            }
            registry.importer = Some(importer);
            Ok(())
        })
    }

    pub fn importer() -> Option<&'static dyn FixtureImporter> {
        REGISTRY.with_borrow(|registry| registry.importer)
    }

    pub fn acquire(
        binding: &FixtureTargetBinding,
    ) -> Result<FixtureImportLease, FixtureImportError> {
        REGISTRY.with_borrow_mut(|registry| {
            if registry.active.is_some() {
                return Err(FixtureImportError::Busy);
            }
            let generation = registry
                .generation
                .checked_add(1)
                .ok_or(FixtureImportError::Busy)?;
            let lease = FixtureImportLease {
                generation,
                binding: Box::new(binding.clone()),
            };
            registry.generation = generation;
            registry.active = Some(lease.clone());
            Ok(lease)
        })
    }

    /// Expired business attempts invalidate the old heap token before a successor fetch.
    pub fn abandon() {
        REGISTRY.with_borrow_mut(|registry| registry.active = None);
    }

    pub fn is_current(lease: &FixtureImportLease) -> bool {
        REGISTRY.with_borrow(|registry| registry.active.as_ref() == Some(lease))
    }

    pub fn release(lease: &FixtureImportLease) {
        REGISTRY.with_borrow_mut(|registry| {
            if registry.active.as_ref() == Some(lease) {
                registry.active = None;
            }
        });
    }
}
