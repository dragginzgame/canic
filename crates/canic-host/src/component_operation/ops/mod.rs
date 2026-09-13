//! Component operation persistence and transport boundary.
//! Durable publication uses the shared host I/O owner.

mod progress;
pub mod transport;

pub use progress::project_progress;

use crate::{
    component_operation::{
        ComponentOperationError,
        model::{
            ComponentAuthorityRecord, ComponentOperationRecord, ComponentPlanRecord,
            ComponentProgressRecord,
        },
        policy::validate_label,
        view::{ComponentObservation, ComponentProgressObservation},
    },
    durable_io::{read_regular_bytes, write_bytes},
};
use canic_core::cdk::utils::hash::sha256_hex;
use std::path::{Path, PathBuf};

const MAX_RECORD_BYTES: usize = 4 * 1024 * 1024;

/// Transport performs individual reads/commands; workflow owns ordering and retry.
pub trait ComponentTransport {
    /// Read current release, placement, controller and available-pool authority.
    fn observe(
        &mut self,
        plan: &ComponentPlanRecord,
    ) -> Result<ComponentObservation, ComponentOperationError>;
    /// Read this exact operation; only typed absence permits submission.
    fn progress(
        &mut self,
        plan: &ComponentPlanRecord,
    ) -> Result<ComponentProgressObservation, ComponentOperationError>;
    /// Submit the retained operation identity once.
    fn submit(&mut self, plan: &ComponentPlanRecord) -> Result<(), ComponentOperationError>;
}

/// Select one confined same-Fleet local operation document.
pub fn record_path(
    root: &Path,
    environment: &str,
    fleet: &str,
    name: &str,
) -> Result<PathBuf, ComponentOperationError> {
    for value in [environment, fleet, name] {
        validate_label(value)?;
    }
    Ok(root
        .join(".canic/component-operations")
        .join(environment)
        .join(fleet)
        .join(format!("{name}.json")))
}

/// Hash immutable review data, excluding its self-referential digest.
pub fn review_digest(plan: &ComponentPlanRecord) -> Result<String, ComponentOperationError> {
    let mut canonical = plan.clone();
    canonical.review_sha256.clear();
    Ok(sha256_hex(&serde_json::to_vec(&canonical)?))
}

/// Create a fresh operation identity only before its first durable publication.
pub fn new_record(
    name: &str,
    authority: ComponentAuthorityRecord,
) -> Result<ComponentOperationRecord, ComponentOperationError> {
    let mut operation_id = [0; 32];
    getrandom::fill(&mut operation_id).map_err(std::io::Error::other)?;
    let mut plan = ComponentPlanRecord {
        schema_version: 1,
        name: name.to_string(),
        operation_id,
        authority,
        review_sha256: String::new(),
    };
    plan.review_sha256 = review_digest(&plan)?;
    let mut record = ComponentOperationRecord {
        document_sha256: String::new(),
        plan,
        submission_attempts: 0,
        progress: None,
    };
    seal(&mut record)?;
    Ok(record)
}

fn document_digest(record: &ComponentOperationRecord) -> Result<String, ComponentOperationError> {
    let mut canonical = record.clone();
    canonical.document_sha256.clear();
    Ok(sha256_hex(&serde_json::to_vec(&canonical)?))
}

fn seal(record: &mut ComponentOperationRecord) -> Result<(), ComponentOperationError> {
    record.document_sha256 = document_digest(record)?;
    Ok(())
}

/// Read and verify one bounded current record without following a symlink.
pub fn read(path: &Path) -> Result<Option<ComponentOperationRecord>, ComponentOperationError> {
    let bytes = match read_regular_bytes(path, MAX_RECORD_BYTES) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let record: ComponentOperationRecord = serde_json::from_slice(&bytes)?;
    if record.plan.schema_version != 1
        || record.plan.operation_id == [0; 32]
        || review_digest(&record.plan)? != record.plan.review_sha256
        || document_digest(&record)? != record.document_sha256
    {
        return Err(ComponentOperationError::Integrity);
    }
    if let Some(progress) = &record.progress {
        if record.submission_attempts == 0 {
            return Err(ComponentOperationError::Integrity);
        }
        crate::component_operation::policy::validate_progress(
            &record.plan.authority,
            None,
            progress,
        )?;
    }
    Ok(Some(record))
}

/// Atomically retain the complete document before any subsequent effect.
pub fn write(
    path: &Path,
    record: &ComponentOperationRecord,
) -> Result<(), ComponentOperationError> {
    let bytes = serde_json::to_vec_pretty(record)?;
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(ComponentOperationError::Integrity);
    }
    Ok(write_bytes(path, &bytes)?)
}

/// Persist one submission reservation, including attempts whose response is lost.
pub fn reserve(
    path: &Path,
    record: &mut ComponentOperationRecord,
) -> Result<(), ComponentOperationError> {
    record.submission_attempts = record
        .submission_attempts
        .checked_add(1)
        .ok_or(ComponentOperationError::Integrity)?;
    seal(record)?;
    write(path, record)
}

/// Retain correlated monotonic Root progress in the same operation document.
pub fn retain_progress(
    path: &Path,
    record: &mut ComponentOperationRecord,
    progress: ComponentProgressRecord,
) -> Result<(), ComponentOperationError> {
    record.progress = Some(progress);
    seal(record)?;
    write(path, record)
}
