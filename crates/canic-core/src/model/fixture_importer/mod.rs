//! Module: model::fixture_importer
//!
//! Responsibility: enforce one heap registration and exact in-flight consumer lease.
//! Does not own: application callbacks, boundary conversion, transport or scheduling.
//! Boundary: the application owns durable progress; this model serializes delivery.

use crate::ids::{ManagedCanisterBinding, ReleaseBuildId};

/// Exact installation and source authority retained by a heap fetch attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureImportAuthority {
    pub target: ManagedCanisterBinding,
    pub installation: [u8; 32],
    pub release_build_id: ReleaseBuildId,
    pub content_id: [u8; 32],
}

/// Exact heap attempt identity, including installation authority for late cleanup fencing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureImportLease {
    generation: u64,
    authority: Box<FixtureImportAuthority>,
}

/// Closed registry failures, converted to public consumer errors by ops.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixtureImporterRegistryError {
    Busy,
    Registration,
}

/// One participant per heap; ops supplies its callback handle without model dependencies.
pub struct FixtureImporterRegistry<I> {
    importer: Option<I>,
    generation: u64,
    active: Option<FixtureImportLease>,
}

impl<I: Copy> FixtureImporterRegistry<I> {
    /// Start a fresh heap with no registered participant or active fetch.
    pub const fn new() -> Self {
        Self {
            importer: None,
            generation: 0,
            active: None,
        }
    }

    /// Read the real heap lease for controlled interruption qualification.
    #[cfg(feature = "internal-test-fixtures")]
    pub const fn fetch_in_flight(&self) -> bool {
        self.active.is_some()
    }

    /// Reject replacement of a participant until the heap is reconstructed.
    pub const fn register(&mut self, importer: I) -> Result<(), FixtureImporterRegistryError> {
        if self.importer.is_some() {
            return Err(FixtureImporterRegistryError::Registration);
        }
        self.importer = Some(importer);
        Ok(())
    }

    /// Copy the handle so callers never hold a registry borrow across callbacks.
    pub const fn importer(&self) -> Option<I> {
        self.importer
    }

    /// Issue one unique lease without wrapping the attempt generation.
    pub fn acquire(
        &mut self,
        authority: FixtureImportAuthority,
    ) -> Result<FixtureImportLease, FixtureImporterRegistryError> {
        if self.active.is_some() {
            return Err(FixtureImporterRegistryError::Busy);
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(FixtureImporterRegistryError::Busy)?;
        let lease = FixtureImportLease {
            generation,
            authority: Box::new(authority),
        };
        self.generation = generation;
        self.active = Some(lease.clone());
        Ok(lease)
    }

    /// Expired business attempts invalidate the old heap token before a successor fetch.
    pub fn abandon(&mut self) {
        self.active = None;
    }

    /// Require the exact attempt and installation authority to remain active.
    pub fn is_current(&self, lease: &FixtureImportLease) -> bool {
        self.active.as_ref() == Some(lease)
    }

    /// Late cleanup must never release a successor attempt.
    pub fn release(&mut self, lease: &FixtureImportLease) {
        if self.is_current(lease) {
            self.active = None;
        }
    }
}
