//! Admit current replacement while preserving uncertain paid effects and opaque predecessor bytes.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    model::{
        DesiredFleet, clean_reinstall::CleanReinstallRecord,
        completed_operation::CompletedOperationRetirementRecord,
    },
    ops::{
        self, EnsurePaths, EnsureStateError,
        operation_selection::{
            self,
            retirement::{intent_path, invalid, pending, recover},
        },
        write_current,
    },
};
use canic_core::cdk::utils::hash::sha256_hex;
use serde_json::Value;
use std::{fs, path::Path};

/// The existing Fleet lock fences old execution throughout current custody qualification.
pub(in crate::fleet_ensure) struct ResetRetirement<'a> {
    _lock: fs::File,
    paths: &'a EnsurePaths,
    environment: String,
    fleet: String,
    replacement_sha256: String,
    has_predecessor: bool,
}

impl<'a> ResetRetirement<'a> {
    /// Qualify replacement bytes before interpreting any predecessor effect envelope.
    pub(in crate::fleet_ensure) fn prepare(
        paths: &'a EnsurePaths,
        desired: &DesiredFleet,
    ) -> Result<Option<Self>, EnsureStateError> {
        ops::resolve_desired_artifacts(&paths.workspace, desired)?;
        let replacement_sha256 = sha256_hex(&serde_json::to_vec(desired).map_err(|_| invalid())?);
        let lock = ops::lock_fleet_file_without_recovery(paths)?;
        ops::clean_reinstall::cancellation::require_no_pending(paths)?;
        if let Some(record) = pending(paths)? {
            if record.replacement_sha256 != replacement_sha256 {
                return Err(invalid());
            }
            recover(paths)?;
        }
        if current_selection(paths, desired)? {
            return Ok(None);
        }
        let has_predecessor = operation_selection::has_retained_files(paths)?;
        require_reconciled_effects(paths)?;
        Ok(Some(Self {
            _lock: lock,
            paths,
            environment: desired.environment.clone(),
            fleet: desired.fleet.clone(),
            replacement_sha256,
            has_predecessor,
        }))
    }

    /// Caller has established current physical custody while this owner holds the Fleet lock.
    pub(in crate::fleet_ensure) fn finish(
        self,
        desired: &DesiredFleet,
        policy: &Path,
        seed: &Path,
    ) -> Result<CleanReinstallRecord, EnsureStateError> {
        if sha256_hex(&serde_json::to_vec(desired).map_err(|_| invalid())?)
            != self.replacement_sha256
        {
            return Err(invalid());
        }
        if self.has_predecessor {
            let archive_sha256 = operation_selection::archive::capture_reset(
                self.paths,
                &self.environment,
                &self.fleet,
            )?;
            let record = CompletedOperationRetirementRecord {
                schema_version: 1,
                environment: self.environment,
                fleet: self.fleet,
                archive_sha256,
                replacement_sha256: self.replacement_sha256,
            };
            write_current(&intent_path(self.paths)?, &record)?;
            recover(self.paths)?;
        }
        ops::clean_reinstall::bind_locked(self.paths, desired, policy, seed)
    }
}

fn current_selection(
    paths: &EnsurePaths,
    desired: &DesiredFleet,
) -> Result<bool, EnsureStateError> {
    let selection = match ops::clean_reinstall::read(paths) {
        Ok(selection) => selection,
        Err(EnsureStateError::Decode { .. } | EnsureStateError::InvalidTerminalSource) => None,
        Err(error) => return Err(error),
    };
    if !selection.is_some_and(|selection| selection.desired.desired() == desired) {
        return Ok(false);
    }
    // A stale selection from a previous reset does not own a later ordinary plan.
    let current = match operation_selection::clean_reinstall_current(paths) {
        Ok(current) => current,
        Err(EnsureStateError::Decode { .. } | EnsureStateError::InvalidTerminalSource) => false,
        Err(error) => return Err(error),
    };
    if !current {
        return Ok(false);
    }
    // Only readable current execution can be resumed. A matching selection marker
    // does not turn damaged application documents into a prerequisite for reset.
    // Failure here falls through to the independent paid-effect envelope check.
    if ops::read_plan(paths).is_err()
        || ops::read_journal(paths).is_err()
        || ops::read_state(paths, &desired.fleet).is_err()
    {
        return Ok(false);
    }
    // An unpaid successor can coexist with its completed infrastructure journal;
    // only matching operation and plan identities prove this Fleet is terminal.
    Ok(!operation_selection::completed_fleet(
        paths,
        &desired.environment,
        &desired.fleet,
    )?)
}

/// Inspect paid-effect envelopes only; neither old plans nor application state are decoded.
fn require_reconciled_effects(paths: &EnsurePaths) -> Result<(), EnsureStateError> {
    if let Some(journal) = evidence(&paths.journal)? {
        check_effects(&paths.journal, &journal, "effects")?;
        if let Some(reviews) = journal.get("funding_reviews") {
            let reviews = reviews
                .as_array()
                .ok_or_else(|| uncertain(&paths.journal, "funding_reviews"))?;
            for (index, review) in reviews.iter().enumerate() {
                check_effect(
                    &paths.journal,
                    review.get("effect"),
                    &format!("funding_reviews/{index}/effect"),
                    false,
                )?;
                if let Some(mint) = review.get("operator_mint").filter(|v| !v.is_null())
                    && mint.get("transfer_argument").is_some_and(|v| !v.is_null())
                    && mint.get("receipt").is_none_or(Value::is_null)
                {
                    return Err(uncertain(
                        &paths.journal,
                        &format!("funding_reviews/{index}/operator_mint"),
                    ));
                }
            }
        }
    }
    let path = paths.plan.with_file_name("capacity-import.json");
    if let Some(import) = evidence(&path)? {
        let handoffs = import
            .get("handoffs")
            .and_then(Value::as_array)
            .ok_or_else(|| uncertain(&path, "handoffs"))?;
        for (index, handoff) in handoffs.iter().enumerate() {
            check_effect(
                &path,
                handoff.get("effect"),
                &format!("handoffs/{index}/effect"),
                true,
            )?;
        }
        // Root-owned reset progress cannot create or delete physical IDs. Stopping
        // and reinstalling that same Root discards its old import reservation;
        // it does not require a terminal receipt or replenish the old budget.
    }
    Ok(())
}

fn evidence(path: &Path) -> Result<Option<Value>, EnsureStateError> {
    operation_selection::read(path).map_err(|error| match error {
        EnsureStateError::Decode { .. } => uncertain(path, "unreadable paid-effect envelope"),
        other => other,
    })
}

fn check_effects(path: &Path, record: &Value, field: &str) -> Result<(), EnsureStateError> {
    let effects = record
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| uncertain(path, field))?;
    for (index, effect) in effects.iter().enumerate() {
        if effect.is_null() {
            return Err(uncertain(path, &format!("{field}/{index}")));
        }
        check_effect(path, Some(effect), &format!("{field}/{index}"), false)?;
    }
    Ok(())
}

fn check_effect(
    path: &Path,
    effect: Option<&Value>,
    label: &str,
    unsubmitted_intent: bool,
) -> Result<(), EnsureStateError> {
    match effect {
        Some(Value::Null) => Ok(()),
        Some(effect) if matches!(effect.get("state").and_then(Value::as_str), Some("applied")) => {
            Ok(())
        }
        Some(effect)
            if unsubmitted_intent
                && effect.get("state").and_then(Value::as_str) == Some("intent") =>
        {
            Ok(())
        }
        _ => Err(uncertain(path, label)),
    }
}

fn uncertain(path: &Path, effect: &str) -> EnsureStateError {
    EnsureStateError::ResetUncertainEffect {
        path: path.into(),
        effect: effect.into(),
    }
}
