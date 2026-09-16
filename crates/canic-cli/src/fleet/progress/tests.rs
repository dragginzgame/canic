use super::*;
use canic_core::dto::component_provisioning::FleetComponentProvisioningPhase;
use canic_host::fleet_ensure::dto::{FleetEnsurePhase, FleetProvisioningProgress};

fn waiting(elapsed_seconds: u64) -> FleetEnsureProgress {
    FleetEnsureProgress {
        operation_id: "operation".into(),
        plan_sha256: "plan".into(),
        phase: FleetEnsurePhase::WorkloadProvisioning,
        applied_effects: 51,
        reviewed_effects: 53,
        state: FleetEnsureProgressState::AwaitingProgress {
            elapsed_seconds,
            provisioning: None,
        },
    }
}

#[test]
fn unchanged_polls_emit_only_at_heartbeat_and_keep_current_details() {
    let mut output = ProgressOutput::default();
    let start = Instant::now();
    let emitted = (0..=65)
        .filter_map(|seconds| {
            let progress = waiting(seconds);
            output
                .should_emit(&progress, start + Duration::from_secs(seconds))
                .then(|| super::super::render_progress(&progress, true))
        })
        .map(|line| serde_json::from_str::<serde_json::Value>(&line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        emitted
            .iter()
            .map(|event| event["progress"]["state"]["elapsed_seconds"]
                .as_u64()
                .unwrap())
            .collect::<Vec<_>>(),
        [0, 30, 60]
    );
    assert!(
        emitted
            .iter()
            .all(|event| event["progress"]["phase"] == "workload_provisioning")
    );
}

#[test]
fn meaningful_changes_emit_immediately_and_restart_heartbeat() {
    let start = Instant::now();
    let mut output = ProgressOutput::default();
    let mut progress = waiting(0);
    assert!(output.should_emit(&progress, start));
    progress.applied_effects += 1;
    assert!(output.should_emit(&progress, start + Duration::from_secs(1)));
    progress.reviewed_effects += 1;
    assert!(output.should_emit(&progress, start + Duration::from_secs(2)));
    progress.phase = FleetEnsurePhase::TerminalVerification;
    assert!(output.should_emit(&progress, start + Duration::from_secs(3)));
    progress.plan_sha256 = "successor-plan".into();
    assert!(output.should_emit(&progress, start + Duration::from_secs(4)));
    progress.operation_id = "new-operation".into();
    assert!(output.should_emit(&progress, start + Duration::from_secs(5)));
    assert!(!output.should_emit(&progress, start + Duration::from_secs(34)));
    assert!(output.should_emit(&progress, start + Duration::from_secs(35)));
}

#[test]
fn provisioning_changes_are_meaningful_but_elapsed_time_is_not() {
    let start = Instant::now();
    let mut output = ProgressOutput::default();
    let mut progress = waiting(0);
    assert!(output.should_emit(&progress, start));
    let mut provisioning = FleetProvisioningProgress {
        phase: FleetComponentProvisioningPhase::ActivatingRuntimes,
        root_batch_count: 2,
        accepted_root_count: 0,
        provisioned_root_count: 0,
        directory_confirmed_root_count: 0,
        directory_confirmation_root_count: 2,
        runtime_activated_root_count: 0,
        component_count: 3,
    };
    progress.state = FleetEnsureProgressState::AwaitingProgress {
        elapsed_seconds: 1,
        provisioning: Some(provisioning.clone()),
    };
    assert!(output.should_emit(&progress, start + Duration::from_secs(1)));
    provisioning.accepted_root_count = 1;
    progress.state = FleetEnsureProgressState::AwaitingProgress {
        elapsed_seconds: 2,
        provisioning: Some(provisioning.clone()),
    };
    assert!(output.should_emit(&progress, start + Duration::from_secs(2)));
    progress.state = FleetEnsureProgressState::AwaitingProgress {
        elapsed_seconds: 3,
        provisioning: Some(provisioning),
    };
    assert!(!output.should_emit(&progress, start + Duration::from_secs(3)));
}

#[test]
fn actionable_and_terminal_events_are_never_suppressed() {
    let start = Instant::now();
    let mut output = ProgressOutput::default();
    for state in [
        FleetEnsureProgressState::Advancing,
        FleetEnsureProgressState::FundingRequired,
        FleetEnsureProgressState::PrerequisiteComplete,
        FleetEnsureProgressState::ReviewRequired {
            reason:
                canic_host::fleet_ensure::model::FleetEnsureSuccessorReviewReason::AdditionalEffect,
            review: None,
        },
        FleetEnsureProgressState::Complete,
    ] {
        let mut progress = waiting(0);
        assert!(output.should_emit(&progress, start));
        progress.state = state;
        assert!(output.should_emit(&progress, start));
        assert!(output.should_emit(&progress, start));
    }
}
