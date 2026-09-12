//! Module: fleet_ensure::workflow::retained_plan
//!
//! Responsibility: explain an unreadable retained plan through existing source-evidence inspection.
//! Does not own: executable decoding, recovery admission, document mutation or remote effects.
//! Boundary: source evidence can recommend a separate review, never authorize plan execution.

use crate::fleet_ensure::{
    model::FleetEnsurePlan,
    ops::{EnsurePaths, read_plan, reinstall::source},
    workflow::EnsureWorkflowError,
};

pub(super) fn read<E: std::error::Error + 'static>(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
) -> Result<Option<FleetEnsurePlan>, EnsureWorkflowError<E>> {
    read_plan(paths).map_err(|error| match source::read(paths, environment, fleet) {
        Ok(evidence) => EnsureWorkflowError::RetainedActivationReviewRequired {
            operation_id: evidence.operation_id,
            plan_sha256: evidence.plan_sha256,
            source_document_sha256: evidence.plan_document_sha256,
            source: Box::new(error),
        },
        Err(_) => EnsureWorkflowError::RetainedPlanUnreadable {
            source: Box::new(error),
        },
    })
}
