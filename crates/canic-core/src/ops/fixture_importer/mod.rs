//! Module: ops::fixture_importer
//!
//! Responsibility: validate application progress and invoke synchronous importer steps.
//! Does not own: application records, transport orchestration or scheduling.
//! Boundary: callback errors and invalid postconditions trap before partial writes commit.

#[cfg(test)]
mod tests;

use crate::{
    dto::fixture_provisioning::{
        FixtureAssignment, FixtureImportError, FixtureImportFailure, FixtureImportProgress,
        FixtureProvisioningStatus, FixtureStoreError,
    },
    model::fixture_importer::{FixtureImportLease, FixtureImporterRegistry},
    ops::{ic::IcOps, storage::fleet_activation::FleetActivationOps},
};

pub use crate::model::fixture_importer::FixtureImporter;

/// Scoped ownership of one fetch; IC callback cleanup also releases this exact lease.
pub struct ImportLease(FixtureImportLease);

impl ImportLease {
    pub fn acquire(assignment: &FixtureAssignment) -> Result<Self, FixtureImportError> {
        FixtureImporterRegistry::acquire(&assignment.grant.binding).map(Self)
    }

    pub fn require_current(&self) -> Result<(), FixtureImportError> {
        if !FixtureImporterRegistry::is_current(&self.0) {
            return Err(FixtureImportError::Authority);
        }
        Ok(())
    }
}

impl Drop for ImportLease {
    fn drop(&mut self) {
        FixtureImporterRegistry::release(&self.0);
    }
}

/// Project the protected source without creating a second import checkpoint.
pub fn assignment() -> Result<Option<Box<FixtureAssignment>>, FixtureImportError> {
    FleetActivationOps::component_fixture_assignment().map_err(runtime_error)
}

/// Observe application progress and reject mismatched or prematurely completed evidence.
pub fn progress(
    importer: &dyn FixtureImporter,
    assignment: &FixtureAssignment,
) -> Result<Option<FixtureImportProgress>, FixtureImportError> {
    let observed = importer.progress(assignment)?;
    if let Some(progress) = &observed {
        validate_progress(assignment, progress)?;
    }
    Ok(observed)
}

pub fn validate_progress(
    assignment: &FixtureAssignment,
    progress: &FixtureImportProgress,
) -> Result<(), FixtureImportError> {
    if progress.binding != assignment.grant.binding {
        return Err(FixtureImportError::Authority);
    }
    if progress.next_chunk as usize > assignment.descriptor.chunks.len() {
        return Err(FixtureImportError::Progress);
    }
    if let Some(receipt) = &progress.receipt {
        if receipt.binding != assignment.grant.binding
            || receipt.completion_summary != assignment.descriptor.completion_summary
        {
            return Err(FixtureImportError::Receipt);
        }
        if progress.next_chunk as usize != assignment.descriptor.chunks.len() {
            return Err(FixtureImportError::Progress);
        }
    }
    Ok(())
}

pub fn status(
    assignment: &FixtureAssignment,
    importer: &dyn FixtureImporter,
) -> Result<FixtureProvisioningStatus, FixtureImportError> {
    let observed = progress(importer, assignment)?;
    Ok(match observed {
        Some(FixtureImportProgress {
            receipt: Some(receipt),
            ..
        }) => FixtureProvisioningStatus::Complete(receipt),
        other => FixtureProvisioningStatus::Pending(other.map(Box::new)),
    })
}

/// Begin exactly once, enforcing the callback's local postcondition before committing.
pub fn begin(importer: &dyn FixtureImporter, assignment: &FixtureAssignment) {
    require_callback(importer.begin(assignment));
    require_position(importer, assignment, 0, false);
}

/// Commit exactly one chunk and require its checkpoint in the same message.
pub fn apply_chunk(
    importer: &dyn FixtureImporter,
    assignment: &FixtureAssignment,
    index: u32,
    bytes: &[u8],
) {
    require_callback(importer.apply_chunk(assignment, index, bytes));
    let next = index
        .checked_add(1)
        .unwrap_or_else(|| fail(FixtureImportError::Progress));
    require_position(importer, assignment, next, false);
}

/// Validate one bounded slice; only the application's durable receipt can finish it.
pub fn validate_step(importer: &dyn FixtureImporter, assignment: &FixtureAssignment) {
    require_callback(importer.validate_step(assignment));
    let next = u32::try_from(assignment.descriptor.chunks.len())
        .unwrap_or_else(|_| fail(FixtureImportError::Progress));
    require_position(importer, assignment, next, true);
}

fn require_position(
    importer: &dyn FixtureImporter,
    assignment: &FixtureAssignment,
    next: u32,
    allow_receipt: bool,
) {
    let observed = progress(importer, assignment)
        .unwrap_or_else(|error| fail(error))
        .unwrap_or_else(|| fail(FixtureImportError::Progress));
    if observed.next_chunk != next || (!allow_receipt && observed.receipt.is_some()) {
        fail(FixtureImportError::Progress);
    }
}

fn require_callback(result: Result<(), FixtureImportError>) {
    if let Err(error) = result {
        fail(error);
    }
}

fn fail(error: FixtureImportError) -> ! {
    IcOps::trap(format!("fixture importer callback failed: {error:?}"))
}

/// Register through the sole heap model owner.
pub fn register(importer: &'static dyn FixtureImporter) -> Result<(), FixtureImportError> {
    FixtureImporterRegistry::register(importer)
}

/// Read the one registered participant without holding a borrow across callbacks.
pub fn registered() -> Option<&'static dyn FixtureImporter> {
    FixtureImporterRegistry::importer()
}

/// Require Active runtime infrastructure independently of application data readiness.
pub fn require_active() -> Result<(), FixtureImportError> {
    let active = FleetActivationOps::status(false).map_err(runtime_error)?;
    if active.phase != crate::dto::fleet_activation::FleetActivationPhase::Active {
        return Err(FixtureImportError::Authority);
    }
    Ok(())
}

fn runtime_error(
    error: crate::ops::storage::fleet_activation::FleetActivationOpsError,
) -> FixtureImportError {
    FixtureImportError::Runtime(
        crate::InternalError::from(crate::ops::storage::StorageOpsError::from(error)).into(),
    )
}

/// Invalidate a stale heap fetch only after the durable owner proves expiry.
pub fn abandon_expired_fetch() {
    FixtureImporterRegistry::abandon();
}

/// Classify returned failures; callback traps remain uncertain work for recovery.
pub fn permanent_failure(error: FixtureImportError) -> Option<FixtureImportFailure> {
    use FixtureImportFailure as F;
    Some(match error {
        FixtureImportError::Busy
        | FixtureImportError::ImporterMissing
        | FixtureImportError::NotReady
        | FixtureImportError::Transport(_)
        | FixtureImportError::Source(FixtureStoreError::NotReady) => return None,
        FixtureImportError::Application { code } => F::Application { code },
        FixtureImportError::Authority => F::Authority,
        FixtureImportError::Codec(error) => F::Codec {
            code: error.raw_code(),
        },
        FixtureImportError::Progress => F::Progress,
        FixtureImportError::Receipt => F::Receipt,
        FixtureImportError::Registration => F::Registration,
        FixtureImportError::Runtime(error) => F::Runtime {
            code: error.raw_code(),
        },
        FixtureImportError::SourceRejected(error) => F::SourceRejected {
            code: error.raw_code(),
        },
        FixtureImportError::Source(error) => match error {
            FixtureStoreError::Authority => F::SourceAuthority,
            FixtureStoreError::Bounds => F::SourceBounds,
            FixtureStoreError::Capacity => F::SourceCapacity,
            FixtureStoreError::Conflict => F::SourceConflict,
            FixtureStoreError::Content => F::SourceContent,
            FixtureStoreError::NotFound => F::SourceNotFound,
            FixtureStoreError::Sequence => F::SourceSequence,
            FixtureStoreError::NotReady => unreachable!(),
        },
    })
}

/// Observe the existing lease without changing transport or application progress.
#[cfg(feature = "internal-test-fixtures")]
pub fn fetch_in_flight() -> bool {
    FixtureImporterRegistry::fetch_in_flight()
}
