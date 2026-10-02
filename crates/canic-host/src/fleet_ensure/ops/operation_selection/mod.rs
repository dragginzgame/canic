//! Select retained work by its operation envelope before decoding executable authority.
//!
//! Completion is historical metadata. This read does not admit a reset, validate
//! an old executable contract, or establish live custody and cycle conservation.

pub(in crate::fleet_ensure) mod archive;
pub(in crate::fleet_ensure) mod retirement;
#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    model::FleetEnsureCompletion,
    ops::{EnsurePaths, EnsureStateError, is_sha256},
    view::readiness::{InfrastructureFundingUnavailable, RetainedReadinessOperation},
};
use serde_json::Value;
use std::path::Path;

/// Detect retained authority without parsing an obsolete or incomplete execution schema.
pub(in crate::fleet_ensure) fn has_retained_files(
    paths: &EnsurePaths,
) -> Result<bool, EnsureStateError> {
    let directory = paths
        .plan
        .parent()
        .ok_or(EnsureStateError::InvalidTerminalSource)?;
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(source) => {
            return Err(EnsureStateError::Io {
                path: directory.into(),
                source,
            });
        }
    };
    for entry in entries {
        let entry = entry.map_err(|source| EnsureStateError::Io {
            path: directory.into(),
            source,
        })?;
        // Local metadata, editor backups and discarded temporary writes do not
        // establish an installation. Keep unreadable owned records as evidence.
        if operation_evidence_name(&entry.file_name()) {
            let path = entry.path();
            let kind = entry.file_type().map_err(|source| EnsureStateError::Io {
                path: path.clone(),
                source,
            })?;
            if kind.is_dir() {
                let mut contents =
                    std::fs::read_dir(&path).map_err(|source| EnsureStateError::Io {
                        path: path.clone(),
                        source,
                    })?;
                if contents
                    .next()
                    .transpose()
                    .map_err(|source| EnsureStateError::Io { path, source })?
                    .is_none()
                {
                    continue;
                }
            }
            return Ok(true);
        }
    }
    Ok(false)
}

fn operation_evidence_name(name: &std::ffi::OsStr) -> bool {
    matches!(
        name.to_str(),
        Some(
            "activation-reset-adoption.json"
                | "activation-reset-evidence"
                | "activation-reset-review.json"
                | "capacity-import-history"
                | "capacity-import-surveys"
                | "capacity-import.json"
                | "clean-reinstall-custody.json"
                | "clean-reinstall-desired.json"
                | "clean-reinstall.json"
                | "infrastructure-bootstrap-completed"
                | "infrastructure-bootstrap-inspections"
                | "infrastructure-bootstrap-publications"
                | "infrastructure-bootstrap-receipts"
                | "infrastructure-bootstrap-survey.json"
                | "infrastructure-bootstrap-surveys"
                | "journal.json"
                | "phases"
                | "plan.json"
                | "state.json"
                | "terminal-receipts"
        )
    )
}

/// Retirement history still requires explicit physical inventory after an interrupted local cut.
pub(in crate::fleet_ensure) fn has_retirement_history(
    paths: &EnsurePaths,
) -> Result<bool, EnsureStateError> {
    let path = retirement::history(paths)?.join("retirements");
    path.try_exists()
        .map_err(|source| EnsureStateError::Io { path, source })
}

/// Read only the completion envelope; unfinished work retains its current recovery owner.
pub(in crate::fleet_ensure) fn completed(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
) -> Result<Option<RetainedReadinessOperation>, EnsureStateError> {
    let Some(journal) = read(&paths.journal)? else {
        return Ok(None);
    };
    if journal.get("completion").and_then(Value::as_str) != Some("converged") {
        return Ok(None);
    }
    let invalid = || EnsureStateError::InvalidTerminalSource;
    let operation_id = text(&journal, "operation_id")?;
    let plan_sha256 = text(&journal, "plan_sha256")?;
    let plan = read(&paths.plan)?.ok_or_else(invalid)?;
    let identity_matches = text(&journal, "fleet")? == fleet
        && text(&plan, "fleet")? == fleet
        && text(&plan, "environment")? == environment;
    if !identity_matches || !is_sha256(operation_id) || !is_sha256(plan_sha256) {
        return Err(invalid());
    }
    if text(&plan, "operation_id")? != operation_id || text(&plan, "plan_sha256")? != plan_sha256 {
        // A newly reviewed plan can precede creation of its journal. Its current
        // owner must inspect it; the previous completion cannot retire that plan.
        return Ok(None);
    }
    // A terminal label cannot hide an explicitly unresolved effect. Do not decode
    // action payloads, successor contracts, desired state or application schemas.
    let effects = journal
        .get("effects")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    if effects
        .iter()
        .any(|effect| effect.get("state").and_then(Value::as_str) != Some("applied"))
    {
        return Err(invalid());
    }
    Ok(Some(RetainedReadinessOperation {
        operation_id: operation_id.into(),
        plan_sha256: plan_sha256.into(),
        completion: FleetEnsureCompletion::Converged,
    }))
}

pub(in crate::fleet_ensure::ops) fn read(path: &Path) -> Result<Option<Value>, EnsureStateError> {
    let bytes = crate::durable_io::read_optional_regular_bytes_bounded(path, 32 * 1024 * 1024)
        .map_err(|_| EnsureStateError::InvalidTerminalSource)?;
    bytes
        .map(|bytes| {
            serde_json::from_slice(&bytes).map_err(|source| EnsureStateError::Decode {
                path: path.to_path_buf(),
                source,
            })
        })
        .transpose()
}

/// Explain a missing forecast from bounded operation metadata without admitting execution.
pub(in crate::fleet_ensure) fn infrastructure_funding_unavailable(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
) -> Result<InfrastructureFundingUnavailable, EnsureStateError> {
    let unavailable = InfrastructureFundingUnavailable::FleetNotCompleted;
    if read(&paths.journal)?.is_some() {
        return Ok(unavailable);
    }
    let Some(plan) = read(&paths.plan)? else {
        return Ok(unavailable);
    };
    if text(&plan, "scope")? != "infrastructure_bootstrap" {
        return Ok(unavailable);
    }
    let operation_id = text(&plan, "operation_id")?;
    let plan_sha256 = text(&plan, "plan_sha256")?;
    let identity_matches =
        text(&plan, "environment")? == environment && text(&plan, "fleet")? == fleet;
    if !identity_matches || !is_sha256(operation_id) || !is_sha256(plan_sha256) {
        return Err(EnsureStateError::InvalidTerminalSource);
    }
    Ok(
        InfrastructureFundingUnavailable::RetainedInfrastructureReview {
            operation_id: operation_id.into(),
            plan_sha256: plan_sha256.into(),
        },
    )
}

/// Completed import metadata retires its executable receipt format from admission decisions.
/// An explicitly unresolved host effect still belongs to its original recovery owner.
pub(in crate::fleet_ensure::ops) fn capacity_import_in_progress(
    paths: &EnsurePaths,
) -> Result<bool, EnsureStateError> {
    let Some(record) = read(&paths.plan.with_file_name("capacity-import.json"))? else {
        return Ok(false);
    };
    let approved = record
        .get("approved")
        .and_then(Value::as_bool)
        .ok_or(EnsureStateError::InvalidTerminalSource)?;
    let handoffs = record
        .get("handoffs")
        .and_then(Value::as_array)
        .ok_or(EnsureStateError::InvalidTerminalSource)?;
    if !approved {
        let reservation = record
            .get("reservation")
            .is_some_and(|value| !value.is_null());
        let operation = record.get("operation").filter(|value| !value.is_null());
        let issued = operation.is_some_and(|operation| {
            operation
                .get("submissions")
                .and_then(Value::as_object)
                .is_some_and(|values| !values.is_empty())
                || operation
                    .get("publication_started")
                    .and_then(Value::as_bool)
                    == Some(true)
                || receipt_present(operation, "settled_status_candid_hex")
                || receipt_present(operation, "released_status_candid_hex")
        });
        if reservation
            || issued
            || handoffs
                .iter()
                .any(|handoff| handoff.get("effect") != Some(&Value::Null))
        {
            return Err(EnsureStateError::InvalidTerminalSource);
        }
        return Ok(false);
    }
    let Some(operation) = record.get("operation") else {
        return Ok(true);
    };
    let released = operation
        .get("publication_started")
        .and_then(Value::as_bool)
        == Some(true)
        && operation
            .get("publication_complete")
            .and_then(Value::as_bool)
            == Some(true);
    if !released
        || !receipt_present(operation, "settled_status_candid_hex")
        || !receipt_present(operation, "released_status_candid_hex")
    {
        return Ok(true);
    }
    let Some(sources) = record
        .get("plan")
        .and_then(|plan| plan.get("sources"))
        .and_then(Value::as_array)
    else {
        return Err(EnsureStateError::InvalidTerminalSource);
    };
    let covered = !sources.is_empty()
        && sources.len() == handoffs.len()
        && sources.iter().zip(handoffs).all(|(source, handoff)| {
            let source = source
                .get("binding")
                .and_then(|binding| binding.get("canister_id"));
            source.is_some_and(|id| !id.is_null() && handoff.get("canister_id") == Some(id))
        });
    if !covered
        || handoffs.iter().any(|handoff| match handoff.get("effect") {
            Some(Value::Null) => false,
            Some(effect) => effect.get("state").and_then(Value::as_str) != Some("applied"),
            None => true,
        })
    {
        return Err(EnsureStateError::InvalidTerminalSource);
    }
    Ok(false)
}

fn receipt_present(operation: &Value, field: &str) -> bool {
    operation
        .get(field)
        .and_then(Value::as_str)
        .is_some_and(|value| {
            !value.is_empty()
                && value.len().is_multiple_of(2)
                && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, EnsureStateError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or(EnsureStateError::InvalidTerminalSource)
}

/// A completed workload operation can be retired without loading its executable contract.
pub(in crate::fleet_ensure) fn completed_fleet(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
) -> Result<bool, EnsureStateError> {
    if completed(paths, environment, fleet)?.is_none() {
        return Ok(false);
    }
    Ok(read(&paths.plan)?
        .is_some_and(|plan| plan.get("scope").and_then(Value::as_str) == Some("full")))
}

/// Route only the current reset's setup or its retained convergence operation.
/// This envelope check selects an owner; that owner still validates executable authority.
pub(in crate::fleet_ensure) fn clean_reinstall_current(
    paths: &EnsurePaths,
) -> Result<bool, EnsureStateError> {
    if read(&paths.plan.with_file_name("clean-reinstall.json"))?.is_none() {
        return Ok(false);
    }
    let Some(plan) = read(&paths.plan)? else {
        return Ok(true);
    };
    if text(&plan, "scope")? == "infrastructure_bootstrap" {
        return Ok(true);
    }
    let archive = paths
        .plan
        .with_file_name("infrastructure-bootstrap-completed");
    let mut entries = match std::fs::read_dir(&archive) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err(EnsureStateError::InvalidTerminalSource),
    };
    let invalid = || EnsureStateError::InvalidTerminalSource;
    let entry = entries.next().ok_or_else(invalid)?.map_err(|_| invalid())?;
    if entries.next().is_some() || !entry.file_type().map_err(|_| invalid())?.is_dir() {
        return Err(invalid());
    }
    let setup = read(&entry.path().join("plan.json"))?.ok_or_else(invalid)?;
    let setup_digest = text(&setup, "plan_sha256")?;
    let setup_operation = text(&setup, "operation_id")?;
    if !is_sha256(setup_digest)
        || !is_sha256(setup_operation)
        || entry.file_name() != setup_digest
        || text(&setup, "scope")? != "infrastructure_bootstrap"
        || text(&setup, "environment")? != text(&plan, "environment")?
        || text(&setup, "fleet")? != text(&plan, "fleet")?
    {
        return Err(invalid());
    }
    Ok(setup_operation == text(&plan, "operation_id")?)
}
