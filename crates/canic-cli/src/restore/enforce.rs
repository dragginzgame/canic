use canic_backup::restore::{RestorePlan, RestoreRunResponse};

use super::{RestoreCommandError, RestorePlanOptions, RestoreRunOptions, RestoreStatusOptions};

// Enforce caller-requested native runner requirements after output is emitted.
pub(super) fn enforce_restore_run_requirements(
    options: &RestoreRunOptions,
    run: &RestoreRunResponse,
) -> Result<(), RestoreCommandError> {
    enforce_restore_requirements(
        options.require_ready,
        options.require_complete,
        options.require_no_attention,
        run,
    )
}

pub(super) fn enforce_restore_status_requirements(
    options: &RestoreStatusOptions,
    run: &RestoreRunResponse,
) -> Result<(), RestoreCommandError> {
    enforce_restore_requirements(
        options.require_ready,
        options.require_complete,
        options.require_no_attention,
        run,
    )
}

fn enforce_restore_requirements(
    require_ready: bool,
    require_complete: bool,
    require_no_attention: bool,
    run: &RestoreRunResponse,
) -> Result<(), RestoreCommandError> {
    enforce_restore_ready(require_ready, run)?;
    if require_complete && !run.complete {
        return Err(RestoreCommandError::RestoreApplyIncomplete {
            backup_id: run.backup_id.clone(),
            completed_operations: run.completed_operations,
            operation_count: run.operation_count,
        });
    }

    if require_no_attention && run.attention_required {
        return Err(RestoreCommandError::RestoreApplyReportNeedsAttention {
            backup_id: run.backup_id.clone(),
            outcome: run.outcome.clone(),
        });
    }

    Ok(())
}

fn enforce_restore_ready(
    require_ready: bool,
    run: &RestoreRunResponse,
) -> Result<(), RestoreCommandError> {
    if require_ready && !run.ready {
        return Err(RestoreCommandError::RestoreApplyNotReady {
            backup_id: run.backup_id.clone(),
            reasons: run.blocked_reasons.clone(),
        });
    }

    Ok(())
}

// Enforce caller-requested restore plan requirements after the plan is emitted.
pub(super) fn enforce_restore_plan_requirements(
    options: &RestorePlanOptions,
    plan: &RestorePlan,
) -> Result<(), RestoreCommandError> {
    if !options.require_restore_ready || plan.readiness_summary.ready {
        return Ok(());
    }

    Err(RestoreCommandError::RestoreNotReady {
        backup_id: plan.backup_id.clone(),
        reasons: plan.readiness_summary.reasons.clone(),
    })
}
