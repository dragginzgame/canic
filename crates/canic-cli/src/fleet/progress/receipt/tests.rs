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
