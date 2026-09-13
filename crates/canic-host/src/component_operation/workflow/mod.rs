//! Review and reconcile one Root-owned Component operation.
//! Every submission follows durable intent; response loss retains the same identity.

use crate::{
    component_operation::{
        ComponentOperationError,
        model::{ComponentAuthorityRecord, ComponentOperationRecord},
        ops::{self, ComponentTransport},
        policy,
    },
    durable_io::lock_file,
};
use std::path::Path;

fn fleet_lock(
    root: &Path,
    environment: &str,
    fleet: &str,
) -> Result<std::fs::File, ComponentOperationError> {
    Ok(crate::fleet_ensure::ops::lock_operation(
        &crate::fleet_ensure::ops::EnsurePaths::under(root, environment, fleet),
    )?)
}

/// Produce or reopen one exact review without submitting a Root command.
pub fn plan<T: ComponentTransport>(
    root: &Path,
    name: &str,
    authority: ComponentAuthorityRecord,
    transport: &mut T,
) -> Result<ComponentOperationRecord, ComponentOperationError> {
    let path = ops::record_path(root, &authority.environment, &authority.fleet, name)?;
    let _fleet_lock = fleet_lock(root, &authority.environment, &authority.fleet)?;
    let _lock = lock_file(&path.with_extension("lock"))?;
    let record = match ops::read(&path)? {
        Some(record) => {
            if record.plan.name != name {
                return Err(ComponentOperationError::Integrity);
            }
            policy::validate_authority(&record.plan.authority, &authority)?;
            record
        }
        None => ops::new_record(name, authority)?,
    };
    let observed = transport.observe(&record.plan)?;
    policy::validate_authority(&record.plan.authority, &observed.authority)?;
    if record.submission_attempts == 0 && observed.ready_assets == 0 {
        return Err(ComponentOperationError::Capacity);
    }
    ops::write(&path, &record)?;
    Ok(record)
}

/// Advance once through the existing Root operation. Repeat the exact review to resume.
/// A successful incomplete result preserves progress and requires no new operation name.
pub fn apply<T: ComponentTransport>(
    root: &Path,
    environment: &str,
    fleet: &str,
    name: &str,
    review: &str,
    transport: &mut T,
) -> Result<ComponentOperationRecord, ComponentOperationError> {
    let path = ops::record_path(root, environment, fleet, name)?;
    let _fleet_lock = fleet_lock(root, environment, fleet)?;
    let _lock = lock_file(&path.with_extension("lock"))?;
    let mut record = ops::read(&path)?.ok_or(ComponentOperationError::Missing)?;
    if record.plan.name != name
        || record.plan.authority.environment != environment
        || record.plan.authority.fleet != fleet
    {
        return Err(ComponentOperationError::Integrity);
    }
    if record.plan.review_sha256 != review {
        return Err(ComponentOperationError::Review);
    }
    if record
        .progress
        .as_ref()
        .is_some_and(|progress| progress.complete)
    {
        return Ok(record);
    }
    let observed = transport.observe(&record.plan)?;
    policy::validate_authority(&record.plan.authority, &observed.authority)?;
    let progress = transport.progress(&record.plan)?.progress;
    if let Some(progress) = progress {
        if record.submission_attempts == 0 {
            return Err(ComponentOperationError::Progress);
        }
        policy::validate_progress(&record.plan.authority, record.progress.as_ref(), &progress)?;
        let complete = progress.complete;
        ops::retain_progress(&path, &mut record, progress)?;
        if complete {
            return Ok(record);
        }
    } else {
        if record.progress.is_some() {
            return Err(ComponentOperationError::Progress);
        }
        if observed.ready_assets == 0 {
            return Err(ComponentOperationError::Capacity);
        }
    }
    // The durable allocation survives a Root restart; its heap timer does not.
    // Replaying the same command resumes that allocation without reserving another.
    ops::reserve(&path, &mut record)?;
    transport.submit(&record.plan)?;
    Ok(record)
}

/// Refresh existing progress without submitting a command or allocating an identity.
pub fn status<T: ComponentTransport>(
    root: &Path,
    environment: &str,
    fleet: &str,
    name: &str,
    transport: &mut T,
) -> Result<ComponentOperationRecord, ComponentOperationError> {
    let path = ops::record_path(root, environment, fleet, name)?;
    let _fleet_lock = fleet_lock(root, environment, fleet)?;
    let _lock = lock_file(&path.with_extension("lock"))?;
    let mut record = ops::read(&path)?.ok_or(ComponentOperationError::Missing)?;
    if record.plan.name != name
        || record.plan.authority.environment != environment
        || record.plan.authority.fleet != fleet
    {
        return Err(ComponentOperationError::Integrity);
    }
    let observed = transport.observe(&record.plan)?;
    policy::validate_authority(&record.plan.authority, &observed.authority)?;
    match transport.progress(&record.plan)?.progress {
        Some(progress) => {
            if record.submission_attempts == 0 {
                return Err(ComponentOperationError::Progress);
            }
            policy::validate_progress(&record.plan.authority, record.progress.as_ref(), &progress)?;
            ops::retain_progress(&path, &mut record, progress)?;
        }
        None if record.progress.is_some() => return Err(ComponentOperationError::Progress),
        None => {}
    }
    Ok(record)
}
