//! Module: fleet_ensure::workflow::retained_plan
//!
//! Responsibility: explain unreadable retained operation documents through source-evidence inspection.
//! Does not own: executable decoding, recovery admission, document mutation or remote effects.
//! Boundary: source evidence can recommend a separate review, never authorize plan execution.

use crate::fleet_ensure::{
    model::{FleetEnsureJournalRecord, FleetEnsurePlan},
    ops::{EnsurePaths, EnsureStateError, read_journal, read_plan, reinstall::source},
    workflow::EnsureWorkflowError,
};

pub(super) fn read<E: std::error::Error + 'static>(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
) -> Result<Option<FleetEnsurePlan>, EnsureWorkflowError<E>> {
    read_plan(paths).map_err(|error| diagnose(paths, environment, fleet, error))
}

pub(super) fn journal<E: std::error::Error + 'static>(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
) -> Result<Option<FleetEnsureJournalRecord>, EnsureWorkflowError<E>> {
    read_journal(paths).map_err(|error| diagnose(paths, environment, fleet, error))
}

fn diagnose<E: std::error::Error + 'static>(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
    error: EnsureStateError,
) -> EnsureWorkflowError<E> {
    match source::read(paths, environment, fleet) {
        Ok(evidence) => EnsureWorkflowError::RetainedActivationReviewRequired {
            operation_id: evidence.operation_id,
            plan_sha256: evidence.plan_sha256,
            source_document_sha256: evidence.plan_document_sha256,
            source: Box::new(error),
        },
        Err(_) => EnsureWorkflowError::RetainedPlanUnreadable {
            source: Box::new(error),
        },
    }
}
