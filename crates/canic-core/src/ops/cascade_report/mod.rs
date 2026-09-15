//! Module: ops::cascade_report
//!
//! Responsibility: bound, validate and combine state-cascade observations.
//! Does not own: transport, target selection, state mutation or retry scheduling.
//! Boundary: reports distinguish observed application from an unconfirmed call.

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::{
        cascade::{StateCascadeReport, StateCascadeTargetOutcome, StateCascadeTargetResult},
        error::Error,
        state::{FleetCommandExecutionResponse, FleetCommandResponse},
    },
    protocol::CASCADE_SNAPSHOT_MAX_BYTES,
};
use std::collections::BTreeSet;

// Reserve space for this report's Candid type envelope and Principal/result rows.
// This limits returned detail, never the targets executed. The wire-bound test
// covers full reports; omitted details remain explicit in the aggregate counts.
pub const MAX_REPORTED_STATE_TARGETS: usize = (CASCADE_SNAPSHOT_MAX_BYTES - 1024) / 128;

/// Stateless conversion and bounded aggregation of observed cascade results.
pub struct StateCascadeReportOps;

impl StateCascadeReportOps {
    pub fn applied(canister_id: Principal, error: Option<Error>) -> StateCascadeReport {
        let result = error.map_or(
            StateCascadeTargetResult::Applied,
            StateCascadeTargetResult::AppliedWithReconciliationFailure,
        );
        Self::single(canister_id, result)
    }

    pub fn unconfirmed(canister_id: Principal, error: InternalError) -> StateCascadeReport {
        Self::single(
            canister_id,
            StateCascadeTargetResult::Unconfirmed(error.into()),
        )
    }

    fn single(canister_id: Principal, result: StateCascadeTargetResult) -> StateCascadeReport {
        let (successful_targets, reconciliation_failures, unconfirmed_targets) = match &result {
            StateCascadeTargetResult::Applied => (1, 0, 0),
            StateCascadeTargetResult::AppliedWithReconciliationFailure(_) => (0, 1, 0),
            StateCascadeTargetResult::Unconfirmed(_) => (0, 0, 1),
        };
        StateCascadeReport {
            successful_targets,
            reconciliation_failures,
            unconfirmed_targets,
            omitted_targets: 0,
            targets: vec![StateCascadeTargetOutcome {
                canister_id,
                result,
            }],
        }
    }

    pub fn validate_reply(
        expected: Principal,
        report: &StateCascadeReport,
    ) -> Result<(), InternalError> {
        Self::validate(report)?;
        match report.targets.first() {
            Some(first)
                if first.canister_id == expected
                    && !matches!(first.result, StateCascadeTargetResult::Unconfirmed(_)) =>
            {
                Ok(())
            }
            _ => Err(InternalError::conflict()),
        }
    }

    fn validate(report: &StateCascadeReport) -> Result<(), InternalError> {
        let total = report
            .successful_targets
            .checked_add(report.reconciliation_failures)
            .and_then(|count| count.checked_add(report.unconfirmed_targets))
            .ok_or_else(InternalError::conflict)?;
        let described = (report.targets.len() as u64)
            .checked_add(report.omitted_targets)
            .ok_or_else(InternalError::conflict)?;
        if report.targets.len() > MAX_REPORTED_STATE_TARGETS || total != described {
            return Err(InternalError::conflict());
        }
        let mut identities = BTreeSet::new();
        let mut successes = 0;
        let mut failures = 0;
        let mut unconfirmed = 0;
        for target in &report.targets {
            if !identities.insert(target.canister_id) {
                return Err(InternalError::conflict());
            }
            match target.result {
                StateCascadeTargetResult::Applied => successes += 1,
                StateCascadeTargetResult::AppliedWithReconciliationFailure(_) => failures += 1,
                StateCascadeTargetResult::Unconfirmed(_) => unconfirmed += 1,
            }
        }
        if successes > report.successful_targets
            || failures > report.reconciliation_failures
            || unconfirmed > report.unconfirmed_targets
        {
            return Err(InternalError::conflict());
        }
        Ok(())
    }

    pub fn merge(
        accumulated: &mut StateCascadeReport,
        incoming: StateCascadeReport,
    ) -> Result<(), InternalError> {
        Self::validate(&incoming)?;
        let known = accumulated
            .targets
            .iter()
            .map(|entry| entry.canister_id)
            .collect::<BTreeSet<_>>();
        if incoming
            .targets
            .iter()
            .any(|entry| known.contains(&entry.canister_id))
        {
            return Err(InternalError::conflict());
        }
        let successful_targets = accumulated
            .successful_targets
            .checked_add(incoming.successful_targets)
            .ok_or_else(InternalError::conflict)?;
        let reconciliation_failures = accumulated
            .reconciliation_failures
            .checked_add(incoming.reconciliation_failures)
            .ok_or_else(InternalError::conflict)?;
        let unconfirmed_targets = accumulated
            .unconfirmed_targets
            .checked_add(incoming.unconfirmed_targets)
            .ok_or_else(InternalError::conflict)?;
        let available = MAX_REPORTED_STATE_TARGETS.saturating_sub(accumulated.targets.len());
        let dropped = incoming.targets.len().saturating_sub(available) as u64;
        let omitted_targets = accumulated
            .omitted_targets
            .checked_add(incoming.omitted_targets)
            .and_then(|count| count.checked_add(dropped))
            .ok_or_else(InternalError::conflict)?;
        successful_targets
            .checked_add(reconciliation_failures)
            .and_then(|count| count.checked_add(unconfirmed_targets))
            .ok_or_else(InternalError::conflict)?;

        accumulated.successful_targets = successful_targets;
        accumulated.reconciliation_failures = reconciliation_failures;
        accumulated.unconfirmed_targets = unconfirmed_targets;
        accumulated.omitted_targets = omitted_targets;
        accumulated
            .targets
            .extend(incoming.targets.into_iter().take(available));
        Ok(())
    }

    pub fn require_complete(report: &StateCascadeReport) -> Result<(), InternalError> {
        Self::validate(report)?;
        if report.reconciliation_failures == 0 && report.unconfirmed_targets == 0 {
            return Ok(());
        }
        for target in &report.targets {
            match target.result {
                StateCascadeTargetResult::Applied => {}
                StateCascadeTargetResult::AppliedWithReconciliationFailure(error)
                | StateCascadeTargetResult::Unconfirmed(error) => {
                    return Err(InternalError::observed_public(error));
                }
            }
        }
        Err(InternalError::conflict())
    }

    pub const fn command_response(
        change: FleetCommandResponse,
        propagation: StateCascadeReport,
        reconciliation_error: Option<Error>,
    ) -> FleetCommandExecutionResponse {
        FleetCommandExecutionResponse {
            change,
            propagation,
            reconciliation_error,
        }
    }
}
