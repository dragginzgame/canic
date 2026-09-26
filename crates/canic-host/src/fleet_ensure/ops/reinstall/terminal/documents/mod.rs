//! Module: fleet_ensure::ops::reinstall::terminal::documents
//!
//! Responsibility: capture bounded completed document identities and exact byte bindings.
//! Does not own: receipt admission, source execution, controller inference or live verification.
//! Boundary: historical desired and registry payloads remain opaque JSON evidence.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::read_regular_bytes,
    fleet_ensure::{
        model::{FleetTerminalSourceRecord, MAX_FLEET_ENSURE_PROTOCOL_STEPS},
        ops::{EnsurePaths, EnsureStateError, is_sha256},
        view::terminal_source::CompletedDocumentsView,
    },
};
use canic_core::cdk::utils::hash::sha256_hex;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};

const MAXIMUM_DOCUMENT_BYTES: usize = 32 * 1024 * 1024;
const MAXIMUM_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;

/// Read source claims without decoding either desired authority or executable phases.
/// Receipt and plan semantic digests must still be verified before any handoff review.
pub(in crate::fleet_ensure) fn read(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
) -> Result<CompletedDocumentsView, EnsureStateError> {
    let mut remaining = MAXIMUM_SNAPSHOT_BYTES;
    let plan_bytes = bytes(&paths.plan, &mut remaining)?;
    let journal_bytes = bytes(&paths.journal, &mut remaining)?;
    let state_bytes = bytes(&paths.state, &mut remaining)?;
    let plan: Value = decode(&plan_bytes)?;
    let journal: Value = decode(&journal_bytes)?;
    let state: Value = decode(&state_bytes)?;
    let source: PlanIdentity = decode(&plan_bytes)?;
    let journal_identity: JournalIdentity = decode(&journal_bytes)?;
    let expected_journal = JournalIdentity {
        schema_version: 1,
        fleet: fleet.to_string(),
        operation_id: source.operation_id.clone(),
        plan_sha256: source.plan_sha256.clone(),
        completion: "converged".into(),
    };
    let path_matches =
        source.schema_version == 1 && source.environment == environment && source.fleet == fleet;
    let labels_valid = [
        &source.operation_id,
        &source.plan_sha256,
        &source.desired_sha256,
    ]
    .into_iter()
    .all(|value| is_sha256(value));
    let state_matches = state.get("schema_version") == Some(&Value::from(1))
        && state.get("fleet").and_then(Value::as_str) == Some(fleet);
    if !path_matches || !labels_valid || !state_matches || journal_identity != expected_journal {
        return Err(invalid());
    }
    let phases = journal
        .get("successor_phases")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    if phases.len() > MAX_FLEET_ENSURE_PROTOCOL_STEPS {
        return Err(invalid());
    }
    let mut bindings = FleetTerminalSourceRecord {
        operation_id: source.operation_id.clone(),
        plan_sha256: source.plan_sha256.clone(),
        plan_document_sha256: sha256_hex(&plan_bytes),
        journal_document_sha256: sha256_hex(&journal_bytes),
        state_document_sha256: sha256_hex(&state_bytes),
        phase_document_sha256: BTreeMap::new(),
    };
    let desired = plan
        .get("reviewed_desired")
        .filter(|value| value.is_object())
        .ok_or_else(invalid)?;
    for reference in phases {
        let label = reference
            .get("plan_sha256")
            .and_then(Value::as_str)
            .ok_or_else(invalid)?;
        if !is_sha256(label) || bindings.phase_document_sha256.contains_key(label) {
            return Err(invalid());
        }
        let path = paths
            .plan
            .with_file_name("phases")
            .join(format!("{label}.json"));
        let phase_bytes = bytes(&path, &mut remaining)?;
        let phase_identity: PlanIdentity = decode(&phase_bytes)?;
        let mut expected = source.clone();
        expected.plan_sha256 = label.to_string();
        let phase: Value = decode(&phase_bytes)?;
        if phase_identity != expected || phase.get("reviewed_desired") != Some(desired) {
            return Err(invalid());
        }
        bindings
            .phase_document_sha256
            .insert(label.to_string(), sha256_hex(&phase_bytes));
    }
    Ok(CompletedDocumentsView {
        bindings,
        plan,
        journal,
        state,
    })
}

/// The identity shared by a completed source and each of its successor phases.
#[derive(Clone, Deserialize, Eq, PartialEq)]
struct PlanIdentity {
    schema_version: u16,
    environment: String,
    fleet: String,
    operation_id: String,
    plan_sha256: String,
    desired_sha256: String,
    planned_at_time: u64,
}

/// Journal claim only; the receipt owner must independently verify completion.
#[derive(Deserialize, Eq, PartialEq)]
struct JournalIdentity {
    schema_version: u16,
    fleet: String,
    operation_id: String,
    plan_sha256: String,
    completion: String,
}

fn bytes(path: &Path, remaining: &mut usize) -> Result<Vec<u8>, EnsureStateError> {
    let bytes =
        read_regular_bytes(path, MAXIMUM_DOCUMENT_BYTES.min(*remaining)).map_err(|source| {
            EnsureStateError::Io {
                path: path.to_path_buf(),
                source,
            }
        })?;
    *remaining = remaining.checked_sub(bytes.len()).ok_or_else(invalid)?;
    Ok(bytes)
}

fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, EnsureStateError> {
    serde_json::from_slice(bytes).map_err(|_| invalid())
}

const fn invalid() -> EnsureStateError {
    EnsureStateError::InvalidTerminalSource
}
