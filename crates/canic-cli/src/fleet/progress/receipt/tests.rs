use super::*;
use canic_host::fleet_ensure::dto::FleetObservationStage;

fn fixture() -> (PathBuf, Receipt, PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "canic-timing-receipt-{}-{}",
        std::process::id(),
        NEXT_FILE.fetch_add(1, Ordering::Relaxed)
    ));
    let (receipt, path) = Receipt::create(
        &root,
        &Invocation {
            command: CommandKind::Ensure,
            fleet: "fleet",
            environment: "local",
            desired_sha256: Some("desired"),
            applied_plan_sha256: Some("reviewed"),
            reinstall: false,
            next_review_command: "canic fleet ensure fleet --desired desired.toml",
        },
    )
    .unwrap();
    (root, receipt, path)
}

fn observation(succeeded: Option<bool>) -> FleetObservationTiming {
    FleetObservationTiming {
        span_id: 1,
        parent_span_id: None,
        stage: FleetObservationStage::Planning,
        parent_stage: None,
        elapsed_millis: 12,
        remote_call_attempts: 3,
        identity_lookup_attempts: 1,
        identity_lookup_millis: 2,
        cached_read_hits: 1,
        succeeded,
    }
}

fn read(path: &Path) -> Vec<serde_json::Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn screen_owned_receipt_failures_are_retained_without_stderr_writes() {
    const CHILD: &str = "CANIC_RECEIPT_SCREEN_FAILURE";
    if let Ok(mode) = std::env::var(CHILD) {
        let (root, mut receipt, path) = fixture();
        let before = fs::read(&path).unwrap();
        receipt.file = File::open(&path).unwrap();
        receipt.defer_error_output();
        if mode == "write" {
            receipt.observation(&observation(Some(true)));
        } else {
            receipt.finish(None);
        }
        assert!(receipt.has_failed());
        assert_eq!(fs::read(&path).unwrap(), before);
        drop(receipt);
        fs::remove_dir_all(root).unwrap();
        return;
    }
    for mode in ["write", "finalization"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "fleet::progress::receipt::tests::screen_owned_receipt_failures_are_retained_without_stderr_writes", "--nocapture"])
            .env(CHILD, mode).output().unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn paired_events_retain_clocks_input_binding_and_failure_without_prose() {
    let (root, mut receipt, path) = fixture();
    receipt.observation(&observation(None));
    receipt.observation(&observation(Some(false)));
    receipt.finish(None);
    drop(receipt);
    let events = read(&path);
    assert_eq!(events[0]["data"]["applied_plan_sha256"], "reviewed");
    assert_eq!(events[1]["data"]["succeeded"], serde_json::Value::Null);
    assert_eq!(events[2]["data"]["succeeded"], false);
    assert_eq!(events[1]["data"]["span_id"], events[2]["data"]["span_id"]);
    assert_eq!(events.last().unwrap()["data"]["state"], "failed");
    assert!(
        events
            .windows(2)
            .all(|pair| pair[0]["elapsed_micros"].as_u64() <= pair[1]["elapsed_micros"].as_u64())
    );
    assert!(
        events
            .iter()
            .all(|event| event["utc_unix_millis"].as_u64().unwrap() > 0)
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unfinished_invocation_retains_start_before_drop_and_never_claims_completion() {
    let (root, mut receipt, path) = fixture();
    receipt.observation(&observation(None));
    let active = read(&path);
    assert_eq!(active.last().unwrap()["event"], "fleet_ensure_observation");
    assert!(active.last().unwrap()["data"]["succeeded"].is_null());
    drop(receipt);
    assert_eq!(read(&path).last().unwrap()["data"]["state"], "incomplete");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn capped_receipt_retains_outcome_and_reports_missing_evidence() {
    let (root, mut receipt, path) = fixture();
    let payload = "x".repeat(MAX_EVENT_BYTES / 2);
    for _ in 0..300 {
        receipt.record("bounded_fixture", &payload);
    }
    receipt.finish(None);
    drop(receipt);
    assert!(fs::metadata(&path).unwrap().len() <= u64::try_from(MAX_BYTES).unwrap());
    let events = read(&path);
    let last = events.last().unwrap();
    assert_eq!(last["event"], "invocation_finished");
    assert!(last["data"]["omitted_events"].as_u64().unwrap() > 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn process_interruption_keeps_partial_receipt_and_continuation_uses_a_new_file() {
    const CHILD_ROOT: &str = "CANIC_RECEIPT_INTERRUPT_ROOT";
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        let root = PathBuf::from(root);
        let (mut receipt, path) = Receipt::create(
            &root,
            &Invocation {
                command: CommandKind::Ensure,
                fleet: "fleet",
                environment: "local",
                desired_sha256: Some("desired"),
                applied_plan_sha256: Some("reviewed"),
                reinstall: false,
                next_review_command: "review",
            },
        )
        .unwrap();
        receipt.observation(&observation(None));
        fs::write(root.join("ready"), path.to_str().unwrap()).unwrap();
        loop {
            std::thread::park();
        }
    }
    let (root, receipt, first_path) = fixture();
    drop(receipt);
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", std::thread::current().name().unwrap()])
        .env(CHILD_ROOT, &root)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let started = Instant::now();
    while !root.join("ready").exists() && started.elapsed().as_secs() < 10 {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    child.kill().unwrap();
    child.wait().unwrap();
    let interrupted = fs::read_to_string(root.join("ready")).unwrap();
    let events = read(Path::new(&interrupted));
    assert_ne!(Path::new(&interrupted), first_path);
    assert_eq!(events.last().unwrap()["event"], "fleet_ensure_observation");
    assert!(events.last().unwrap()["data"]["succeeded"].is_null());
    let prior = fs::read(&interrupted).unwrap();
    let (mut resumed, resumed_path) = Receipt::create(
        &root,
        &Invocation {
            command: CommandKind::Ensure,
            fleet: "fleet",
            environment: "local",
            desired_sha256: Some("desired"),
            applied_plan_sha256: Some("reviewed"),
            reinstall: false,
            next_review_command: "review",
        },
    )
    .unwrap();
    resumed.observation(&observation(None));
    resumed.observation(&observation(Some(true)));
    resumed.close("completed", None);
    assert_ne!(resumed_path, Path::new(&interrupted));
    assert_eq!(fs::read(&interrupted).unwrap(), prior);
    assert_eq!(
        read(&resumed_path)[0]["data"]["applied_plan_sha256"],
        events[0]["data"]["applied_plan_sha256"]
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn repeated_activity_is_not_confirmed_remote_advancement() {
    let (root, mut receipt, path) = fixture();
    let mut progress = FleetEnsureProgress {
        next_action: None,
        operation_id: "op".into(),
        plan_sha256: "plan".into(),
        phase: canic_host::fleet_ensure::dto::FleetEnsurePhase::WorkloadProvisioning,
        state: FleetEnsureProgressState::AwaitingProgress {
            elapsed_seconds: 1,
            provisioning: None,
        },
        applied_effects: 1,
        reviewed_effects: 3,
    };
    receipt.progress(&progress);
    progress.state = FleetEnsureProgressState::AwaitingProgress {
        elapsed_seconds: 30,
        provisioning: None,
    };
    receipt.progress(&progress);
    progress.applied_effects = 2;
    receipt.progress(&progress);
    progress.plan_sha256 = "successor".into();
    progress.applied_effects = 3;
    receipt.progress(&progress);
    let events = read(&path);
    assert_eq!(
        events
            .iter()
            .filter(|event| event["event"] == "fleet_ensure_progress")
            .map(|event| event["data"]["confirmed_remote_advancement"]
                .as_bool()
                .unwrap())
            .collect::<Vec<_>>(),
        [false, false, true, false]
    );
    drop(receipt);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn retry_deadline_changes_are_retained_without_claiming_remote_advancement() {
    use canic_core::dto::component_provisioning::{
        FleetComponentProvisioningPhase, FleetComponentProvisioningRetryStage,
        FleetComponentProvisioningRootFailure, ProvisioningFailureOrigin, ProvisioningFailureStage,
        ProvisioningRetryCategory,
    };
    use canic_host::fleet_ensure::dto::{FleetEnsurePhase, FleetProvisioningProgress};
    let (root, mut receipt, path) = fixture();
    for retry_at_ns in [Some(1_000_000_123_u64), Some(2_000_000_123), None] {
        receipt.progress(&FleetEnsureProgress {
            next_action: None,
            operation_id: "op".into(),
            plan_sha256: "plan".into(),
            phase: FleetEnsurePhase::WorkloadProvisioning,
            applied_effects: 1,
            reviewed_effects: 3,
            state: FleetEnsureProgressState::AwaitingProgress {
                elapsed_seconds: 30,
                provisioning: Some(FleetProvisioningProgress {
                    phase: FleetComponentProvisioningPhase::ActivatingRuntimes,
                    root_batch_count: 1,
                    accepted_root_count: 1,
                    provisioned_root_count: 1,
                    directory_confirmed_root_count: 1,
                    directory_confirmation_root_count: 1,
                    runtime_activated_root_count: 0,
                    component_count: 2,
                    pending_root_failure: Some(FleetComponentProvisioningRootFailure {
                        origin: Some(ProvisioningFailureOrigin {
                            failed_at_ns: 123,
                            retry_at_ns,
                            stage: ProvisioningFailureStage::ComponentMembership,
                            target: candid::Principal::from_slice(&[8]),
                            operation_id: [9; 32],
                            diagnostic_code: 137,
                            retry_category: ProvisioningRetryCategory::Backoff,
                        }),
                        fleet_subnet_root: candid::Principal::from_slice(&[7]),
                        stage: FleetComponentProvisioningRetryStage::RuntimeActivation,
                        diagnostic_code: 137,
                        failed_at_ns: 124,
                    }),
                }),
            },
        });
        let events = read(&path);
        let data = &events.last().unwrap()["data"];
        assert_eq!(data["confirmed_remote_advancement"], false);
        assert_eq!(
            data["progress"]["state"]["provisioning"]["pending_root_failure"]["origin"]["retry_at_ns"],
            serde_json::to_value(retry_at_ns).unwrap()
        );
    }
    drop(receipt);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn protected_request_receipts_preserve_child_subject_parent_and_incomplete_pairs() {
    use canic_host::icp::{IcpRequestKind, IcpRequestTiming};
    let (root, mut receipt, path) = fixture();
    receipt.observation(&observation(None));
    let endpoint = candid::Principal::from_slice(&[1]);
    let child = candid::Principal::from_slice(&[2]);
    let mut request = IcpRequestTiming {
        request_id: 10,
        parent_request_id: None,
        kind: IcpRequestKind::CanisterInspection,
        target: Some(endpoint.to_text()),
        subject: Some(child),
        method: None,
        elapsed_micros: 0,
        in_flight: 1,
        succeeded: None,
    };
    receipt.request(&request);
    request.request_id = 11;
    request.parent_request_id = Some(10);
    request.kind = IcpRequestKind::Query;
    request.method = Some("canic_observability".into());
    receipt.request(&request);
    request.elapsed_micros = 20;
    request.succeeded = Some(false);
    receipt.request(&request);
    // Leave the inclusive inspection unmatched, as an interrupted invocation would.
    receipt.close("interrupted", None);
    drop(receipt);
    let events = read(&path);
    let requests = events
        .iter()
        .filter(|event| event["event"] == "icp_request_timing")
        .collect::<Vec<_>>();
    for event in &requests {
        assert_eq!(event["data"]["parent_span_id"], 1);
        assert_eq!(event["data"]["request"]["target"], endpoint.to_text());
        assert_eq!(event["data"]["request"]["subject"], child.to_text());
    }
    assert!(requests[0]["data"]["request"]["succeeded"].is_null());
    assert_eq!(requests[1]["data"]["request"]["parent_request_id"], 10);
    assert_eq!(requests[2]["data"]["request"]["succeeded"], false);
    assert_eq!(events.last().unwrap()["data"]["state"], "interrupted");
    fs::remove_dir_all(root).unwrap();
}
