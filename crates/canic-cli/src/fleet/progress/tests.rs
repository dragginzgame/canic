use super::*;
use canic_core::dto::component_provisioning::FleetComponentProvisioningPhase;
use canic_host::fleet_ensure::dto::{FleetEnsurePhase, FleetProvisioningProgress};

fn waiting(elapsed_seconds: u64) -> FleetEnsureProgress {
    FleetEnsureProgress {
        next_action: None,
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
        pending_root_failure: None,
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

fn activating() -> FleetEnsureProgress {
    let mut event = waiting(33);
    event.applied_effects = 70;
    event.reviewed_effects = 72;
    event.state = FleetEnsureProgressState::AwaitingProgress {
        elapsed_seconds: 33,
        provisioning: Some(FleetProvisioningProgress {
            pending_root_failure: None,
            phase: FleetComponentProvisioningPhase::ActivatingRuntimes,
            root_batch_count: 1,
            accepted_root_count: 1,
            provisioned_root_count: 1,
            directory_confirmed_root_count: 1,
            directory_confirmation_root_count: 1,
            runtime_activated_root_count: 0,
            component_count: 3,
        }),
    };
    event
}

#[test]
fn supplied_advancing_sequence_is_one_milestone_then_readable_stage_changes() {
    let now = Instant::now();
    let mut output = ProgressOutput::default();
    let mut event = activating();
    event.phase = FleetEnsurePhase::ControlPlane;
    event.state = FleetEnsureProgressState::Advancing;
    let mut emitted = Vec::new();
    for count in 48..=70 {
        event.applied_effects = count;
        if output.should_emit(&event, now + Duration::from_millis(u64::from(count))) {
            emitted.push(count);
        }
    }
    assert_eq!(emitted, [48]);
    for phase in [
        FleetComponentProvisioningPhase::ProvisioningRoots,
        FleetComponentProvisioningPhase::ComponentsProvisioned,
        FleetComponentProvisioningPhase::ConfirmingDirectories,
        FleetComponentProvisioningPhase::DirectoriesConfirmed,
        FleetComponentProvisioningPhase::ActivatingRuntimes,
    ] {
        let mut event = activating();
        if let FleetEnsureProgressState::AwaitingProgress {
            provisioning: Some(detail),
            ..
        } = &mut event.state
        {
            detail.phase = phase;
        }
        assert!(output.should_emit(&event, now + Duration::from_secs(1)));
    }
    let lines = render::panel(&activating(), Duration::ZERO, Duration::from_secs(33));
    assert!(lines.len() <= 12);
    assert!(lines.iter().all(|line| line.len() < 80));
    let panel = lines.join("\n");
    assert!(panel.contains("Waiting for application services to start"));
    assert!(panel.contains("Start application services       Waiting 0/1"));
    assert!(panel.contains("Components in scope: 3"));
    assert!(panel.contains("70/72"));
    assert!(!panel.contains("Deployment verified"));
}

#[test]
fn local_animation_marks_stale_observations_without_inventing_backend_progress() {
    let event = activating();
    let fresh =
        render::panel(&event, Duration::from_millis(250), Duration::from_secs(5)).join("\n");
    let stale = render::panel(&event, Duration::from_secs(31), Duration::from_secs(36)).join("\n");
    assert!(fresh.contains("/ Waiting"));
    assert!(stale.contains("! Waiting"));
    assert!(stale.contains("observation: 31s old (stale)"));
    assert!(stale.contains("33s awaiting this effect here"));
    assert!(!stale.contains("64s awaiting"));
    assert_eq!(event.applied_effects, 70);
}

#[test]
fn unknown_readiness_and_all_applied_counts_never_claim_deployment_success() {
    let mut event = waiting(4);
    event.applied_effects = 53;
    let text = render::panel(&event, Duration::ZERO, Duration::ZERO).join("\n");
    assert!(text.contains("Readiness detail unavailable"));
    assert!(text.contains("53/53"));
    assert!(!text.contains("Deployment verified"));
    event.state = FleetEnsureProgressState::PrerequisiteComplete;
    assert!(render::plain(&event).contains("verification pending"));
    event.state = FleetEnsureProgressState::Complete;
    assert!(render::plain(&event).contains("Deployment verified"));
}

#[test]
fn successor_denominator_and_operator_boundaries_are_immediate() {
    let now = Instant::now();
    let mut output = ProgressOutput::default();
    let mut event = activating();
    event.state = FleetEnsureProgressState::Advancing;
    assert!(output.should_emit(&event, now));
    event.reviewed_effects = 80;
    assert!(output.should_emit(&event, now));
    for state in [
        FleetEnsureProgressState::FundingRequired,
        FleetEnsureProgressState::PrerequisiteComplete,
        FleetEnsureProgressState::Complete,
    ] {
        event.state = state;
        assert!(output.should_emit(&event, now));
    }
}

#[test]
fn reported_retry_is_distinct_from_waiting_and_keeps_root_evidence_in_json() {
    let mut event = activating();
    let failure = canic_core::dto::component_provisioning::FleetComponentProvisioningRootFailure {
        origin: None,
        fleet_subnet_root: candid::Principal::anonymous(),
        stage: canic_core::dto::component_provisioning::FleetComponentProvisioningRetryStage::RuntimeActivation,
        diagnostic_code: 42,
        failed_at_ns: 123,
    };
    if let FleetEnsureProgressState::AwaitingProgress {
        provisioning: Some(detail),
        ..
    } = &mut event.state
    {
        detail.pending_root_failure = Some(failure);
    }
    assert!(render::plain(&event).contains(&failure.fleet_subnet_root.to_text()));
    let json: serde_json::Value = serde_json::from_str(&render_progress(&event, true)).unwrap();
    assert_eq!(
        json["progress"]["state"]["provisioning"]["pending_root_failure"]["diagnostic_code"],
        42
    );
}

#[test]
fn live_selection_respects_redirection_and_limited_terminals() {
    assert!(terminal::supports_live(true, Some("xterm-256color"), false));
    assert!(!terminal::supports_live(false, Some("xterm"), false));
    assert!(!terminal::supports_live(true, Some("dumb"), false));
    assert!(!terminal::supports_live(true, None, false));
    assert!(!terminal::supports_live(true, Some("xterm"), true));
    assert!(!render::plain(&activating()).contains('\x1b'));
}

#[test]
fn repaint_bounds_names_and_resizes_without_erasing_prior_messages() {
    let mut painter = terminal::Painter::default();
    let mut bytes = Vec::new();
    let lines = vec!["unsafe\x1b[2J\nname界".repeat(20), "second".into()];
    painter.paint(&mut bytes, &lines, (80, 24)).unwrap();
    let first = String::from_utf8(bytes.clone()).unwrap();
    assert!(first.lines().all(|line| line.len() <= 79));
    assert!(!first.contains('\x1b'));
    bytes.clear();
    painter.paint(&mut bytes, &lines, (24, 24)).unwrap();
    let resized = String::from_utf8(bytes).unwrap();
    assert_eq!(resized.matches("\x1b[1A").count(), 5);
    let mut bytes = Vec::new();
    painter.clear(&mut bytes, (24, 24)).unwrap();
    assert_eq!(
        String::from_utf8(bytes).unwrap().matches("\x1b[1A").count(),
        2
    );
    let mut bytes = Vec::new();
    painter.clear(&mut bytes, (24, 24)).unwrap();
    assert!(bytes.is_empty());
}

#[test]
fn broken_repaint_returns_typed_io_error_without_terminal_mode_changes() {
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut painter = terminal::Painter::default();
    let error = painter
        .paint(&mut Broken, &["progress".into()], (80, 24))
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    let mut cleanup = Vec::new();
    painter.clear(&mut cleanup, (80, 24)).unwrap();
    let cleanup = String::from_utf8(cleanup).unwrap();
    assert!(!cleanup.contains("?25"));
    assert!(!cleanup.contains("?1049"));
}

#[test]
fn callback_transports_preserve_json_and_finish_before_following_output() {
    const CHILD: &str = "CANIC_PROGRESS_TEST_TRANSPORT";
    if let Ok(mode) = std::env::var(CHILD) {
        replay_callbacks(&mode);
        return;
    }
    for mode in ["plain", "json"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "fleet::progress::tests::callback_transports_preserve_json_and_finish_before_following_output", "--nocapture"])
            .env(CHILD, mode)
            .env("TERM", "xterm")
            .output().unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stderr).unwrap();
        assert!(!text.contains('\x1b'));
        if mode == "json" {
            let events = text
                .lines()
                .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event["event"] == "fleet_ensure_progress")
                    .count(),
                25
            );
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event["event"] == "fleet_ensure_observation")
                    .count(),
                2
            );
        } else {
            assert_eq!(text.matches("Preparing Fleet services").count(), 1);
            assert!(text.contains("Operator funding required"));
            assert_eq!(text.matches("remote call attempts").count(), 1);
            assert!(!text.contains("cached reads"));
            assert!(text.ends_with("following error or recovery command stays visible\n"));
        }
    }
}

fn replay_callbacks(mode: &str) {
    let session = ProgressSession::new(mode == "json");
    let sink = session.sink();
    for stage in [
        FleetObservationStage::OperatorBalance,
        FleetObservationStage::Planning,
    ] {
        sink.observation(FleetObservationTiming {
            span_id: 1,
            parent_span_id: None,
            identity_lookup_millis: 0,
            stage,
            parent_stage: if stage == FleetObservationStage::Planning {
                None
            } else {
                Some(FleetObservationStage::Planning)
            },
            elapsed_millis: 500,
            remote_call_attempts: 25,
            identity_lookup_attempts: 1,
            cached_read_hits: 2,
            succeeded: Some(true),
        });
    }
    let mut event = activating();
    event.phase = FleetEnsurePhase::ControlPlane;
    event.state = FleetEnsureProgressState::Advancing;
    for count in 48..=70 {
        event.applied_effects = count;
        sink.progress(event.clone());
    }
    sink.progress(activating());
    if mode == "cancel" {
        thread::sleep(Duration::from_secs(10));
    }
    if mode == "resize" {
        assert!(
            std::process::Command::new("stty")
                .args(["cols", "24", "rows", "12"])
                .status()
                .unwrap()
                .success()
        );
        thread::sleep(Duration::from_millis(350));
        assert!(
            std::process::Command::new("stty")
                .args(["cols", "80", "rows", "24"])
                .status()
                .unwrap()
                .success()
        );
    }
    if mode == "live" || mode == "resize" {
        thread::sleep(Duration::from_millis(350));
    }
    event.state = FleetEnsureProgressState::FundingRequired;
    sink.progress(event);
    drop(session);
    if mode != "json" {
        eprintln!("following error or recovery command stays visible");
    }
}

#[test]
fn retry_timestamps_do_not_reset_the_last_real_transition() {
    let event = activating();
    let mut repeated = event.clone();
    if let FleetEnsureProgressState::AwaitingProgress {
        provisioning: Some(detail),
        ..
    } = &mut repeated.state
    {
        detail.pending_root_failure = Some(canic_core::dto::component_provisioning::FleetComponentProvisioningRootFailure {
            origin: None, fleet_subnet_root: candid::Principal::anonymous(),
            stage: canic_core::dto::component_provisioning::FleetComponentProvisioningRetryStage::RuntimeActivation,
            diagnostic_code: 42, failed_at_ns: 123,
        });
    }
    assert_eq!(transition_identity(&event), transition_identity(&repeated));
    let mut output = ProgressOutput::default();
    let now = Instant::now();
    assert!(output.should_emit(&event, now));
    assert!(output.should_emit(&repeated, now));
}

#[test]
fn plain_heartbeat_reports_staleness_even_without_new_host_observations() {
    let event = activating();
    let mut output = ProgressOutput::default();
    let now = Instant::now();
    assert!(output.should_emit(&event, now));
    assert!(!output.should_emit(&event, now + Duration::from_secs(29)));
    assert!(output.should_emit(&event, now + Duration::from_secs(30)));
    let heartbeat = render::milestone(&event, Duration::from_secs(30), Duration::from_secs(40));
    assert!(heartbeat.contains("observation: 30s old (stale)"));
    assert!(heartbeat.contains("last change: 40s ago"));
    assert!(heartbeat.contains("33s awaiting this effect here"));
}
