//! Stable operator results derived from the workflow's returned authority.
//!
//! This projection supplies argument arrays; it never grants new approval or reads journals.

#[cfg(test)]
mod tests;

use super::EnsureOptions;
use canic_host::fleet_ensure::model::{FleetEnsurePlan, FleetEnsurePlanScope, FleetEnsureReport};
use serde::Serialize;

/// Current operation phase, independent of human headings and storage layout.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Phase {
    Infrastructure,
    Import,
    Fleet,
}

/// A bounded operator decision distinguishing review, new approval and resumption.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ActionKind {
    Apply,
    Resume,
    Review,
    ReviewFunding,
}

/// Execute the argument vector directly in the same workspace, without a shell.
#[derive(Serialize)]
pub(super) struct NextAction {
    pub kind: ActionKind,
    pub requires_approval: bool,
    pub executable: Option<&'static str>,
    pub arguments: Vec<String>,
}

/// Public completion and next-action contract; nullable identities stay explicit.
#[derive(Serialize)]
pub(super) struct OperationResult {
    pub phase: Phase,
    pub operation_id: Option<String>,
    pub plan_sha256: Option<String>,
    pub review_sha256: Option<String>,
    pub phase_completed: bool,
    pub fleet_completed: bool,
    pub next_action: Option<NextAction>,
}

pub(super) fn command(
    environment: &str,
    icp: &str,
    identity: Option<&str>,
    subcommand: &str,
    fleet: &str,
) -> Vec<String> {
    let mut args = vec![
        "--environment".into(),
        environment.into(),
        "--icp".into(),
        icp.into(),
        "fleet".into(),
        subcommand.into(),
        fleet.into(),
        "--json".into(),
    ];
    if let Some(identity) = identity {
        args.extend(["--identity".into(), identity.into()]);
    }
    args
}

pub(super) fn action(kind: ActionKind, arguments: Vec<String>) -> NextAction {
    NextAction {
        kind,
        requires_approval: matches!(kind, ActionKind::Apply | ActionKind::ReviewFunding),
        executable: (!arguments.is_empty()).then_some("canic"),
        arguments,
    }
}

pub(super) fn ensure_command(options: &EnsureOptions, environment: &str) -> Vec<String> {
    let mut args = command(
        environment,
        &options.icp,
        options.identity.as_deref(),
        "ensure",
        &options.fleet,
    );
    args.extend([
        "--desired".into(),
        options.desired.to_string_lossy().into_owned(),
    ]);
    args
}

pub(super) fn reinstall_review_command(options: &EnsureOptions, environment: &str) -> Vec<String> {
    let mut args = ensure_command(options, environment);
    args.extend([
        "--reinstall".into(),
        "--source".into(),
        options.source.to_string_lossy().into_owned(),
        "--seed".into(),
        options.seed.to_string_lossy().into_owned(),
    ]);
    args
}

pub(super) fn ensure(
    report: &FleetEnsureReport,
    options: &EnsureOptions,
    reinstall: bool,
) -> OperationResult {
    let infrastructure = report.plan.scope == FleetEnsurePlanScope::InfrastructureBootstrap;
    let fleet_completed = report.terminal && report.plan.scope == FleetEnsurePlanScope::Full;
    let mut args = ensure_command(options, &report.plan.environment);
    let next_action = if fleet_completed {
        None
    } else if report.funding_review.is_some() {
        // A funding review has a separate approval surface. Do not suggest an
        // ordinary apply as authority for another transfer.
        Some(action(ActionKind::ReviewFunding, Vec::new()))
    } else if report.terminal {
        if reinstall {
            args = reinstall_review_command(options, &report.plan.environment);
        }
        Some(action(ActionKind::Review, args))
    } else {
        args.extend(["--apply".into(), report.plan.plan_sha256.clone()]);
        let kind = if options.apply.as_deref() == Some(&report.plan.plan_sha256) {
            ActionKind::Resume
        } else {
            ActionKind::Apply
        };
        Some(action(kind, args))
    };
    OperationResult {
        phase: if infrastructure {
            Phase::Infrastructure
        } else {
            Phase::Fleet
        },
        operation_id: Some(report.plan.operation_id.clone()),
        plan_sha256: Some(report.plan.plan_sha256.clone()),
        review_sha256: None,
        phase_completed: report.terminal,
        fleet_completed,
        next_action,
    }
}

pub(super) fn bootstrap(
    plan: &FleetEnsurePlan,
    completed: bool,
    review_sha256: Option<String>,
    applying: bool,
    arguments: Vec<String>,
) -> OperationResult {
    OperationResult {
        phase: Phase::Infrastructure,
        operation_id: Some(plan.operation_id.clone()),
        plan_sha256: Some(plan.plan_sha256.clone()),
        review_sha256,
        phase_completed: completed,
        fleet_completed: false,
        next_action: (!completed).then(|| {
            action(
                if applying {
                    ActionKind::Resume
                } else {
                    ActionKind::Apply
                },
                arguments,
            )
        }),
    }
}

pub(super) fn import(
    digest: String,
    completed: bool,
    applying: bool,
    mut arguments: Vec<String>,
    reinstall: bool,
) -> OperationResult {
    let next_action = if completed {
        if reinstall {
            Some(action(ActionKind::Review, arguments))
        } else {
            None
        }
    } else {
        arguments.extend(["--apply".into(), digest.clone()]);
        Some(action(
            if applying {
                ActionKind::Resume
            } else {
                ActionKind::Apply
            },
            arguments,
        ))
    };
    OperationResult {
        phase: Phase::Import,
        operation_id: None,
        plan_sha256: None,
        review_sha256: Some(digest),
        phase_completed: completed,
        fleet_completed: false,
        next_action,
    }
}
