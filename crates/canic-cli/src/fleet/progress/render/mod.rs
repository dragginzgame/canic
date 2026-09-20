//! Module: fleet::progress::render
//!
//! Responsibility: render truthful human descriptions from typed host progress.
//! Does not own: backend readiness, progress inference or deployment completion.
//! Boundary: Root counts stay distinct from Components and reviewed effects from time.

use canic_core::dto::component_provisioning::FleetComponentProvisioningPhase;
use canic_host::fleet_ensure::dto::{
    FleetEnsureActionKind, FleetEnsurePhase, FleetEnsureProgress, FleetEnsureProgressState,
    FleetProvisioningProgress,
};
use std::{fmt::Write as _, time::Duration};

pub(in crate::fleet) fn plain(progress: &FleetEnsureProgress) -> String {
    let mut line = format!(
        "Fleet deployment: {}; reviewed effects {}/{}",
        job(progress),
        progress.applied_effects,
        progress.reviewed_effects
    );
    if let Some(action) = &progress.next_action {
        let _ = write!(
            line,
            "; next reviewed work: {} ({})",
            action_label(action.kind),
            safe_text(&action.target)
        );
    }
    if let FleetEnsureProgressState::AwaitingProgress {
        elapsed_seconds,
        provisioning,
    } = &progress.state
    {
        let _ = write!(line, "; {elapsed_seconds}s awaiting this effect here");
        if let Some(detail) = provisioning {
            let _ = write!(
                line,
                "; prepared Roots {}/{}; registered Roots {}/{}; active Roots {}/{}; Components in scope {}",
                detail.provisioned_root_count,
                detail.root_batch_count,
                detail.directory_confirmed_root_count,
                detail.directory_confirmation_root_count,
                detail.runtime_activated_root_count,
                detail.root_batch_count,
                detail.component_count
            );
            if let Some(retry) = detail.pending_root_failure {
                let _ = write!(
                    line,
                    "; reported retry for Root {} ({:?}, code {})",
                    retry.fleet_subnet_root, retry.stage, retry.diagnostic_code
                );
            }
        } else {
            line.push_str("; readiness detail unavailable");
        }
    }
    if let FleetEnsureProgressState::ReviewRequired {
        review: Some(review),
        ..
    } = &progress.state
    {
        let _ = write!(line, "; {review}");
    }
    line
}

pub(super) fn milestone(
    progress: &FleetEnsureProgress,
    age: Duration,
    unchanged: Duration,
) -> String {
    let mut line = plain(progress);
    if age.as_secs() >= 30 {
        let _ = write!(
            line,
            "; observation: {}s old (stale); last change: {}s ago",
            age.as_secs(),
            unchanged.as_secs()
        );
    }
    line
}

pub(super) fn panel(
    progress: &FleetEnsureProgress,
    age: Duration,
    unchanged: Duration,
) -> Vec<String> {
    let stale = age.as_secs() >= 30;
    let marker = if stale {
        "!"
    } else if matches!(
        progress.state,
        FleetEnsureProgressState::AwaitingProgress { .. }
    ) {
        ["|", "/", "-", "\\"][(age.as_millis() / 250 % 4) as usize]
    } else {
        " "
    };
    let mut lines = vec![format!("Fleet deployment    {marker} {}", job(progress))];
    if let Some(action) = &progress.next_action {
        lines.push(format!(
            "Next reviewed: {} ({})",
            action_label(action.kind),
            safe_text(&action.target)
        ));
    }
    if let FleetEnsureProgressState::AwaitingProgress {
        elapsed_seconds,
        provisioning,
    } = &progress.state
    {
        lines.push(String::new());
        lines.push(format!("{:<32} State / observed Roots", "Stage"));
        if let Some(detail) = provisioning {
            lines.extend(stages(detail));
            lines.push("Final checks                     Pending".into());
            if let Some(retry) = detail.pending_root_failure {
                lines.push(format!(
                    "Reported retry: {:?}, code {}",
                    retry.stage, retry.diagnostic_code
                ));
            }
            lines.push(format!(
                "{elapsed_seconds}s awaiting this effect here; Components in scope: {}",
                detail.component_count
            ));
        } else {
            lines.push("Readiness detail unavailable".into());
            lines.push(format!(
                "{elapsed_seconds}s awaiting this effect here (last reported)"
            ));
        }
    }
    lines.push(effect_bar(progress));
    lines.push(format!(
        "Last change: {}s ago; observation: {}s old{}",
        unchanged.as_secs(),
        age.as_secs(),
        if stale { " (stale)" } else { "" }
    ));
    lines
}

fn stages(detail: &FleetProvisioningProgress) -> [String; 3] {
    [
        stage(
            "Prepare application canisters",
            detail.provisioned_root_count,
            detail.root_batch_count,
            matches!(
                detail.phase,
                FleetComponentProvisioningPhase::RootsAccepted
                    | FleetComponentProvisioningPhase::ProvisioningRoots
            ),
        ),
        stage(
            "Register services",
            detail.directory_confirmed_root_count,
            detail.directory_confirmation_root_count,
            matches!(
                detail.phase,
                FleetComponentProvisioningPhase::ComponentsProvisioned
                    | FleetComponentProvisioningPhase::ServiceTopologyPublished
                    | FleetComponentProvisioningPhase::ConfirmingDirectories
            ),
        ),
        stage(
            "Start application services",
            detail.runtime_activated_root_count,
            detail.root_batch_count,
            matches!(
                detail.phase,
                FleetComponentProvisioningPhase::DirectoriesConfirmed
                    | FleetComponentProvisioningPhase::ActivatingRuntimes
            ),
        ),
    ]
}

fn stage(label: &str, done: u32, total: u32, waiting: bool) -> String {
    let state = if total == 0 || done > total {
        "Unknown"
    } else if done == total {
        "Done"
    } else if waiting {
        "Waiting"
    } else {
        "Pending"
    };
    format!("{label:<32} {state:<7} {done}/{total}")
}

fn effect_bar(progress: &FleetEnsureProgress) -> String {
    let filled = if progress.reviewed_effects == 0 {
        0
    } else {
        (u128::from(progress.applied_effects) * 10 / progress.reviewed_effects as u128).min(10)
            as usize
    };
    format!(
        "Reviewed effects: {}/{} [{}{}] (not deployment %)",
        progress.applied_effects,
        progress.reviewed_effects,
        "#".repeat(filled),
        "-".repeat(10 - filled)
    )
}

const fn job(progress: &FleetEnsureProgress) -> &'static str {
    match &progress.state {
        FleetEnsureProgressState::Complete => "Deployment verified",
        FleetEnsureProgressState::FundingRequired => "Operator funding required",
        FleetEnsureProgressState::ReviewRequired { .. } => "Operator review required",
        FleetEnsureProgressState::PrerequisiteComplete => {
            "Prerequisites complete; deployment verification pending"
        }
        FleetEnsureProgressState::AwaitingProgress {
            provisioning: Some(detail),
            ..
        } => provisioning_job(detail.phase),
        FleetEnsureProgressState::AwaitingProgress { .. } => match progress.phase {
            FleetEnsurePhase::TerminalVerification => "Waiting for final verification",
            FleetEnsurePhase::PoolReadiness => "Waiting for pool readiness",
            _ => "Waiting for observed progress",
        },
        FleetEnsureProgressState::Advancing => match progress.phase {
            FleetEnsurePhase::Infrastructure => "Preparing infrastructure",
            FleetEnsurePhase::ImportReconciliation => "Reconciling pool canisters",
            FleetEnsurePhase::ControlPlane => "Preparing Fleet services and artifacts",
            FleetEnsurePhase::WorkloadProvisioning => "Preparing application services",
            FleetEnsurePhase::PoolReadiness => "Checking pool readiness",
            FleetEnsurePhase::TerminalVerification | FleetEnsurePhase::Complete => {
                "Verifying deployment"
            }
        },
    }
}

const fn provisioning_job(phase: FleetComponentProvisioningPhase) -> &'static str {
    match phase {
        FleetComponentProvisioningPhase::Planned
        | FleetComponentProvisioningPhase::AcceptingRoots => "Waiting for Roots to accept work",
        FleetComponentProvisioningPhase::RootsAccepted
        | FleetComponentProvisioningPhase::ProvisioningRoots => "Preparing application canisters",
        FleetComponentProvisioningPhase::ComponentsProvisioned
        | FleetComponentProvisioningPhase::ServiceTopologyPublished
        | FleetComponentProvisioningPhase::ConfirmingDirectories => "Registering services",
        FleetComponentProvisioningPhase::DirectoriesConfirmed
        | FleetComponentProvisioningPhase::ActivatingRuntimes => {
            "Waiting for application services to start"
        }
        FleetComponentProvisioningPhase::RuntimesActivated => {
            "Application services started; verification pending"
        }
    }
}

const fn action_label(kind: FleetEnsureActionKind) -> &'static str {
    match kind {
        FleetEnsureActionKind::ActivateRegistry => "Activate Fleet registry",
        FleetEnsureActionKind::ActivateRegistryMirror => "Activate registry mirror",
        FleetEnsureActionKind::AdoptStore => "Connect artifact Store",
        FleetEnsureActionKind::BootstrapStore => "Prepare artifact Store",
        FleetEnsureActionKind::Create => "Create canister",
        FleetEnsureActionKind::Delete => "Delete canister",
        FleetEnsureActionKind::Fund => "Fund canister",
        FleetEnsureActionKind::FundEstate => "Fund Root estate",
        FleetEnsureActionKind::Install => "Install canister",
        FleetEnsureActionKind::JoinRoot => "Register Root",
        FleetEnsureActionKind::MaintainPoolReadiness => "Prepare ready pool canisters",
        FleetEnsureActionKind::ObservePoolReadiness => "Verify pool readiness",
        FleetEnsureActionKind::PrepareComponentRegistry => "Prepare service registry",
        FleetEnsureActionKind::PrepareStoreFixture => "Prepare artifact upload",
        FleetEnsureActionKind::Protocol => "Apply reviewed protocol action",
        FleetEnsureActionKind::ProvisionComponents => "Provision application services",
        FleetEnsureActionKind::PublishStoreChunk
        | FleetEnsureActionKind::PublishStoreFixtureChunk => "Upload artifact chunk",
        FleetEnsureActionKind::ReconcilePoolAsset => "Reconcile pool canister",
        FleetEnsureActionKind::SealAuthority => "Seal canister authority",
        FleetEnsureActionKind::SetControllers => "Set canister controllers",
        FleetEnsureActionKind::Start => "Start canister",
        FleetEnsureActionKind::Stop => "Stop canister",
        FleetEnsureActionKind::SynchronizeRegistry => "Synchronize registry",
        FleetEnsureActionKind::Transfer => "Transfer cycles",
    }
}

fn safe_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_control() { '?' } else { ch })
        .collect()
}
