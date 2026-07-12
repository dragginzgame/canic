//! Module: state_manifest::aggregation
//!
//! Responsibility: aggregate audit status, project next actions, and order
//! state-manifest checks deterministically.
//! Does not own: check construction, report serialization, package resolution,
//! or rendering.
//! Boundary: consumes typed audit checks and returns deterministic report
//! outcome values without side effects.

use super::{StateAuditCheck, StateAuditStatus};

pub(super) fn aggregate_status(checks: &[StateAuditCheck]) -> StateAuditStatus {
    if checks.is_empty() {
        return StateAuditStatus::NotEvaluated;
    }
    if checks
        .iter()
        .any(|check| check.status == StateAuditStatus::Fail)
    {
        return StateAuditStatus::Fail;
    }
    if checks
        .iter()
        .any(|check| check.status == StateAuditStatus::Warn)
    {
        return StateAuditStatus::Warn;
    }
    StateAuditStatus::Pass
}

pub(super) fn next_actions(status: StateAuditStatus, role: Option<&str>) -> Vec<String> {
    let scope = role.unwrap_or("project");
    match status {
        StateAuditStatus::Pass => vec![format!(
            "state metadata declarations for {scope} have no blocking findings"
        )],
        StateAuditStatus::Warn => vec![format!(
            "review warning checks before using {scope} state metadata as an upgrade gate"
        )],
        StateAuditStatus::Fail => vec![format!(
            "fix failing state metadata checks before upgrade or release gating for {scope}"
        )],
        StateAuditStatus::NotEvaluated => vec![format!(
            "declare state metadata before auditing {scope} upgrade safety"
        )],
    }
}

pub(super) fn sort_checks(checks: &mut [StateAuditCheck]) {
    checks.sort_by(|left, right| {
        (
            status_rank(left.status),
            left.category,
            left.code,
            left.subject.as_str(),
        )
            .cmp(&(
                status_rank(right.status),
                right.category,
                right.code,
                right.subject.as_str(),
            ))
    });
}

const fn status_rank(status: StateAuditStatus) -> u8 {
    match status {
        StateAuditStatus::Fail => 0,
        StateAuditStatus::Warn => 1,
        StateAuditStatus::NotEvaluated => 2,
        StateAuditStatus::Pass => 3,
    }
}
