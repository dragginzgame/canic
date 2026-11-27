use super::*;
use canic_host::Plan;

#[test]
fn standby_pool_preview_uses_the_prepared_principals_and_terms() {
    use canic_management_protocol::stored::{StandbyImport, StandbyPolicy};

    let policy = StandbyPolicy {
        root: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        minimum_available: 2,
        maximum_creations: 3,
        expires_at_ns: 123,
        minimum_import_cycles: 17.try_into().unwrap(),
        imports: vec![StandbyImport {
            principal: "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
            previous_root: Some("r7inp-6aaaa-aaaaa-aaabq-cai".into()),
        }],
    };
    let plan = canic_host::FleetPlanning::PlacementPools {
        routes: Default::default(),
        app: "miners".into(),
        fleet: "demo".into(),
        network: Network::Mainnet,
        controller: "operator".into(),
        capacity: None,
        increases: vec![],
        shards: vec![],
        singleton: None,
        instances: None,
        workers: vec![],
        standby: Some(canic_host::PoolGrant {
            approval: policy,
            request: vec![],
        }),
    };
    let (operation, output) = plan_preview(&plan, Path::new(".")).unwrap();
    assert!(operation.is_none());
    for line in [
        "Placement approvals · miners/demo",
        "Network: IC mainnet",
        "Controller: operator",
        "Standby pool Root: rrkah-fqaaa-aaaaa-aaaaq-cai",
        "Min available spares: 2 (subnet fleet) · at most 3 automatic creations · deadline 123 Unix ns",
        "Import ryjl3-tyaaa-aaaaa-aaaba-cai · minimum 17 cycles",
        "Previous Root: r7inp-6aaaa-aaaaa-aaabq-cai",
    ] {
        assert!(output.contains(line), "{output}");
    }
}

#[test]
fn stored_delivery_keeps_pin_evidence_separate_from_current_cache_and_retry_authority() {
    use canic_management_protocol::stored::{Delivery, PinState};
    let mut delivery: Delivery = serde_json::from_value(serde_json::json!({
        "reference": {
            "store": "aaaaa-aa",
            "manifest": {
                "release": {"name": "app", "version": "v1"},
                "hash": vec![1; 32], "bytes": 8, "chunks": vec![[1; 32]],
            },
        },
        "operation": vec![2; 32], "state": "Pending", "cache_deadline": null,
    }))
    .unwrap();
    for status in [
        ManagementStatus::Creation(canic_management_protocol::Status {
            phase: canic_management_protocol::Phase::UploadIssued,
            ..Default::default()
        }),
        ManagementStatus::Upgrade(canic_management_protocol::upgrade::Status {
            phase: canic_management_protocol::upgrade::Phase::UploadIssued,
            ..Default::default()
        }),
    ] {
        for (state, expected) in [
            (PinState::Pending, "not dispatched in this observation"),
            (PinState::Issued, "issued; retention outcome unknown"),
            (
                PinState::Retained,
                "retained for this exact operation and release",
            ),
        ] {
            delivery.state = state;
            delivery.cache_deadline = (state == PinState::Retained).then_some(123);
            let detail = StatusDetail::StoredChild {
                maintenance: None,
                delivery: Some(Box::new(delivery.clone())),
            };
            let before = serde_json::to_value(&detail).unwrap();
            let text = management::render(Some(&status), &detail, false);
            assert!(text.contains(expected), "{text}");
            assert!(text.contains("prepare cache"));
            assert!(text.contains("Cache contents are not observed here"));
            assert!(text.contains("do not authorize re-preparation or installation replay"));
            if state == PinState::Retained {
                assert!(text.contains("1970-01-01 00:00:00 UTC (original)"));
                assert!(!text.contains("no dispatch retained"));
            } else {
                assert!(text.contains("no dispatch retained in this observation"));
            }
            assert_eq!(serde_json::to_value(&detail).unwrap(), before);
        }
    }
    let unknown = management::render(
        None,
        &StatusDetail::StoredChild {
            delivery: None,
            maintenance: None,
        },
        false,
    );
    assert!(unknown.contains("pin and cache preparation are unknown"));
    assert!(!unknown.contains("Store pin:"));
    assert!(!management::render(None, &StatusDetail::Deployment {}, false).contains("Store"));
}

// Existing JSON-shaped examples remain output-contract fixtures; production callers
// now carry typed evidence and never decode their own presentation output.
fn fixture(value: &serde_json::Value) -> Result<CanisterStatus> {
    let decode = |error| Error::Supplier(Box::new(error));
    let plan: Plan = serde_json::from_value(value["plan"].clone()).map_err(decode)?;
    let management = if value["management"].is_null() {
        None
    } else if plan.target().is_some() {
        Some(ManagementStatus::Upgrade(
            serde_json::from_value(value["management"].clone()).map_err(decode)?,
        ))
    } else {
        Some(ManagementStatus::Creation(
            serde_json::from_value(value["management"].clone()).map_err(decode)?,
        ))
    };
    let detail = if plan.store_upgrade_root().is_some() {
        StatusDetail::StoreMaintenance {
            store_maintenance: serde_json::from_value(value["store_maintenance"].clone())
                .map_err(decode)?,
        }
    } else if plan.publication().is_some() {
        StatusDetail::Publication {
            publication: serde_json::from_value(value["publication"].clone()).map_err(decode)?,
        }
    } else {
        StatusDetail::Deployment {}
    };
    Ok(CanisterStatus {
        operation: value["operation"].as_str().unwrap().into(),
        plan,
        effects: serde_json::from_value(value["effects"].clone()).map_err(decode)?,
        receipt: serde_json::from_value(value["receipt"].clone()).map_err(decode)?,
        management,
        detail,
    })
}

fn render(value: &serde_json::Value, journal: &Path) -> Result<String> {
    super::render(
        &OperationStatus::Canister(Box::new(fixture(value)?)),
        journal,
    )
}

fn state() -> serde_json::Value {
    serde_json::json!({
        "operation": "a".repeat(64),
        "plan": Plan {
            version: 1,
            network: Network::Mainnet,
            controller: "aaaaa-aa".into(),
            artifact: canic_host::Artifact { sha256: "b".repeat(64), bytes: 8 },
            action: canic_host::PlanAction::Create {bootstrap: None, cycles: 1_000_000_000_000},
            created_at_ns: 1_000_000_000,
            expires_at_ns: 2_000_000_000,
            deployment: Some(canic_host::DeploymentName {
                app: "profiles".into(), fleet: "demo".into(), canister: "worker".into(),
            }),
        },
        "effects": { "create": "not_submitted", "install": "not_submitted" },
        "receipt": null,
    })
}

fn store_state() -> serde_json::Value {
    let mut state = state();
    state["plan"]["action"] = serde_json::json!({"contract":"canic:store-maintenance.v1","approval":{
        "sequence":1, "target":{"canister":"ryjl3-tyaaa-aaaaa-aaaba-cai","module":"22".repeat(32)},
        "root":{"canister":"rrkah-fqaaa-aaaaa-aaaaq-cai","module":"11".repeat(32)}
    }});
    let plan: Plan = serde_json::from_value(state["plan"].clone()).unwrap();
    state["store_maintenance"] =
        serde_json::to_value(canic_management_protocol::store_maintenance::Status {
            approval: canic_management_protocol::store_maintenance::Approval {
                store: plan.target().unwrap().clone(),
                previous_module: [0x22; 32],
                module_hash: [0xbb; 32],
                attempt: serde_json::from_value(serde_json::json!({
                    "sequence": 1, "operation": ([1; 32]),
                }))
                .unwrap(),
                expires_at_ns: plan.expires_at_ns,
            },
            operator: plan.controller.clone(),
            phase: canic_management_protocol::store_maintenance::Phase::Verified,
            stop_request: Some([2; 32]),
            failure: None,
            preservation: None,
            installation: None,
        })
        .unwrap();
    state
}

#[test]
fn store_catalog_summary_distinguishes_partial_absent_and_completed_scans() {
    use canic_management_protocol::store_maintenance::{Installation, Preservation, Status};
    let mut status: Status =
        serde_json::from_value(store_state()["store_maintenance"].clone()).unwrap();
    let mut scan: Preservation = serde_json::from_value(serde_json::json!({
        "catalog": {"quota": {"reserved_bytes": 1024, "releases": 8},
            "reserved_bytes": 128, "releases": 3, "artifacts": 1, "chunks": 1},
        "after": null, "observed": 2, "digest": ([0; 32]), "complete": false,
        "after_removed": false
    }))
    .unwrap();
    status.preservation = Some(scan.clone());
    let text = super::store::render(&status, Some(0), false);
    assert!(text.contains("before 2/3 releases (incomplete); after not retained"));
    assert!(text.contains("Continuation:"));
    scan.observed = 3;
    scan.complete = true;
    status.preservation = Some(scan.clone());
    status.installation = Some(Installation {
        upload_request: [1; 32],
        request: Some([2; 32]),
        sender_version: Some(1),
        start_request: Some([3; 32]),
        verification: Some(scan),
        activation_request: Some([4; 32]),
    });
    let text = super::store::render(&status, Some(0), true);
    assert!(text.contains("before 3/3 releases (complete); after 3/3 releases (complete)"));
    assert!(!text.contains("Continuation:"));
}

#[test]
fn store_upgrade_status_exposes_phase_without_claiming_completion() {
    let mut state = store_state();
    state["store_maintenance"]["approval"]["attempt"]["sequence"] = 2.into();
    let text = render(&state, Path::new("journal")).unwrap();
    assert!(text.contains("Store maintenance: Verified (attempt 2)"));
    assert!(text.contains("Root: rrkah-fqaaa-aaaaa-aaaaq-cai"));
    assert!(text.contains("deployment completion is not established"));
    state["store_maintenance"] = serde_json::Value::Null;
    assert!(
        render(&state, Path::new("journal"))
            .unwrap()
            .contains("no retained observation")
    );
}

#[test]
fn store_scan_summary_keeps_unknown_and_rejected_attempts_visible() {
    let mut state = store_state();
    let effects = state["effects"].as_object_mut().unwrap();
    for index in 0..512 {
        for step in ["observe", "continue"] {
            effects.insert(format!("store/{index}/{step}"), "reply_retained".into());
        }
    }
    for (index, status) in [
        "signed_attempt_retained_outcome_unknown",
        "signed_attempt_retained_outcome_unknown",
        "rejection_retained",
        "authority_retained_not_signed",
    ]
    .iter()
    .enumerate()
    {
        effects.insert(format!("store/{}/continue", 512 + index), (*status).into());
    }
    let before = state.clone();
    let text = render(&state, Path::new("journal")).unwrap();
    for (description, count) in [
        ("Reply retained; not itself proof of success", "1024"),
        ("Signed attempt retained; outcome UNKNOWN", "2"),
        ("Rejection retained", "1"),
        ("Recovery request prepared; not signed", "1"),
    ] {
        assert!(
            text.lines()
                .any(|line| line.starts_with(description) && line.ends_with(count))
        );
    }
    assert!(text.lines().count() < 40);
    assert!(!text.contains("store/511"));
    assert!(text.contains("expiry does not establish failure or authorize a replacement"));
    assert!(text.contains("deployment completion is not established"));
    assert_eq!(
        state, before,
        "presentation must not change the complete JSON evidence"
    );
}

#[test]
fn store_without_prepared_attempts_does_not_claim_no_effect() {
    let mut state = store_state();
    state["store_maintenance"] = serde_json::Value::Null;
    let text = render(&state, Path::new("journal")).unwrap();
    assert!(text.contains("No retained request attempts"));
    assert!(text.contains("no retained observation"));
    assert!(text.contains("deployment completion is not established"));
}

#[test]
fn store_continuations_observe_issued_effects_even_after_dispatch_expiry() {
    use canic_management_protocol::store_maintenance::{Phase, Status};
    let mut status: Status =
        serde_json::from_value(store_state()["store_maintenance"].clone()).unwrap();
    status.approval.expires_at_ns = 100;
    status.failure = Some(canic_management_protocol::Failure::Uncertain);
    for phase in [
        Phase::Closing,
        Phase::StopIssued,
        Phase::UploadIssued,
        Phase::InstallIssued,
        Phase::StartIssued,
        Phase::ActivateIssued,
        Phase::Verifying,
        Phase::Ready,
    ] {
        status.phase = phase;
        for now in [Some(99), Some(100), None] {
            assert_eq!(
                store::continuation(&status, now).0,
                "reconcile",
                "{phase:?}"
            );
        }
        if phase != Phase::Ready {
            assert!(store::render(&status, Some(100), false).contains("Dispatch window expired"));
        }
    }
}

#[test]
fn store_continuations_never_suggest_fresh_effects_after_expiry_or_failure() {
    use canic_management_protocol::store_maintenance::{Phase, Status};
    let mut status: Status =
        serde_json::from_value(store_state()["store_maintenance"].clone()).unwrap();
    status.approval.expires_at_ns = 100;
    for phase in [
        Phase::Reserved,
        Phase::Closed,
        Phase::Stopped,
        Phase::Uploaded,
        Phase::Installed,
        Phase::Verified,
    ] {
        status.phase = phase;
        status.failure = None;
        assert_eq!(
            store::continuation(&status, Some(99)).0,
            "apply",
            "{phase:?}"
        );
        assert_eq!(
            store::continuation(&status, Some(100)).0,
            "status",
            "{phase:?}"
        );
        assert!(
            store::continuation(&status, Some(100))
                .1
                .contains("expired")
        );
        assert_eq!(store::continuation(&status, None).0, "status");
        status.failure = Some(canic_management_protocol::Failure::Mismatch);
        assert_eq!(store::continuation(&status, Some(99)).0, "status");
        assert!(store::continuation(&status, Some(99)).1.contains("failure"));
    }
}

#[test]
fn store_apply_followup_keeps_scope_and_signer_only_for_signed_commands() {
    use canic_management_protocol::store_maintenance::{Phase, Status};
    use clap::Parser as _;
    let cli = crate::args::Cli::try_parse_from([
        "canic",
        "fleet",
        "apply",
        "original",
        "--config",
        "project's config.toml",
        "--app",
        "demo",
        "--fleet",
        "local",
        "--state-dir",
        "journal path",
        "--identity",
        "operator",
        "--icp-bin",
        "signer path",
    ])
    .unwrap();
    let crate::args::Commands::Fleet(command) = cli.command else {
        unreachable!()
    };
    for (phase, deadline, expected) in [
        (Phase::InstallIssued, 1, "reconcile"),
        (Phase::Verified, u64::MAX, "apply"),
        (Phase::Verified, 1, "status"),
    ] {
        let mut status: Status =
            serde_json::from_value(store_state()["store_maintenance"].clone()).unwrap();
        status.phase = phase;
        status.approval.expires_at_ns = deadline;
        let outcome = canic_host::ApplyOutcome::StorePending {
            operation: "original".into(),
            root: "root".into(),
            store_maintenance: Box::new(status),
        };
        let text = crate::progress::applied(&outcome, &command);
        assert!(text.contains(&format!("canic fleet {expected} original")));
        assert!(text.contains("--config 'project'\"'\"'s config.toml'"));
        assert!(text.contains("--state-dir 'journal path' --app demo --fleet local"));
        assert_eq!(
            text.contains("--identity operator --icp-bin 'signer path'"),
            expected != "status"
        );
    }
}

#[test]
fn retained_replies_and_expiry_never_claim_deployment_success() {
    let mut state = state();
    for (evidence, expected) in [
        ("not_submitted", "Not submitted"),
        (
            "reply_retained",
            "Reply retained; not itself proof of success",
        ),
        ("rejection_retained", "Rejection retained"),
        (
            "signed_attempt_retained_outcome_unknown",
            "Signed attempt retained; outcome UNKNOWN",
        ),
    ] {
        state["effects"]["create"] = evidence.into();
        state["effects"]["install"] = evidence.into();
        let before = state.clone();
        let text = render(&state, Path::new("journal")).unwrap();
        assert!(text.contains(expected));
        assert!(text.contains("(expired)"));
        assert!(text.contains("deployment completion is not established"));
        assert!(text.contains("no network observation"));
        assert!(!text.contains("Deployment verified"));
        if evidence == "signed_attempt_retained_outcome_unknown" {
            assert!(text.contains("may have executed"));
            assert!(text.contains("expiry does not establish failure or authorize a replacement"));
        }
        assert_eq!(state, before);
    }
}

#[test]
fn upgrade_summary_keeps_previous_module_and_unexpected_creation_evidence() {
    let mut state = managed_state(true);
    state["effects"]["create"] = "not_submitted".into();
    let text = render(&state, Path::new("journal")).unwrap();
    assert!(text.contains("Planned action: Root upgrades its existing child"));
    assert!(text.contains("Upgrade target: ryjl3-tyaaa-aaaaa-aaaba-cai"));
    assert!(text.contains(&format!("Required previous module: {}", "c".repeat(64))));
    assert!(text.contains("Creation debit ceiling: 0 cycles"));
    assert!(text.contains("Not applicable to this upgrade (not submitted)"));
    state["effects"]["create"] = "signed_attempt_retained_outcome_unknown".into();
    let text = render(&state, Path::new("journal")).unwrap();
    assert!(!text.contains("Not applicable"));
    assert!(text.contains("outcome UNKNOWN"));
}

#[test]
fn receipts_are_historical_and_display_escaping_preserves_raw_evidence() {
    let mut state = state();
    state["receipt"] = serde_json::json!({
        "operation": "a".repeat(64), "canister": "aaaaa-aa",
        "module_hash": "b".repeat(64), "controller": "hostile\n\u{1b}\u{202e}",
    });
    let before = state.clone();
    let text = render(&state, Path::new("journal\n\u{1b}\u{202e}")).unwrap();
    assert!(text.contains("Receipt: retained (historical deployment evidence)"));
    assert!(text.contains("Canister: aaaaa-aa"));
    assert!(
        text.contains("Current code, controllers, readiness and balances are not checked here")
    );
    assert!(text.contains("journal\\n\\u{1b}\\u{202e}"));
    assert!(text.contains("hostile\\n\\u{1b}\\u{202e}"));
    assert!(!text.contains('\u{1b}'));
    assert!(!text.contains('\u{202e}'));
    assert_eq!(state, before);
}

fn managed_state(upgrade: bool) -> serde_json::Value {
    let mut value = state();
    let owner = serde_json::json!({"child_evidence_only":false, "root":"rrkah-fqaaa-aaaaa-aaaaq-cai", "root_module":"d".repeat(64),
        "execute_at_ns":1_500_000_000_u64, "slot":canic_management_protocol::children::ChildId::FIRST, "source":{
            "store":{"canister":"aaaaa-aa","module":"f".repeat(64)}, "manifest":{"release":{"name":"worker","version":"v1"},"hash":([1;32]),"bytes":8,"chunks":[([1;32])]}}});
    value["plan"]["action"] = if upgrade {
        serde_json::json!({"contract":"canic:store-child-upgrade.v1","approval":{"owner":owner,"target":{"canister":"ryjl3-tyaaa-aaaaa-aaaba-cai","module":"c".repeat(64)}}})
    } else {
        serde_json::json!({"contract":"canic:store-child-creation.v1","approval":owner})
    };
    value["effects"] = serde_json::json!({"stage":"reply_retained"});
    value
}

#[test]
fn previous_child_approvals_are_refused_before_status_projection() {
    for (upgrade, contract) in [
        (false, "canic:child-creation.v1"),
        (true, "canic:child-upgrade.v1"),
    ] {
        let mut state = managed_state(upgrade);
        state["plan"]["action"]["contract"] = contract.into();
        assert!(render(&state, Path::new("journal")).is_err());
    }
}

#[test]
fn unresolved_creation_keeps_principal_refund_and_dispatch_uncertainty_visible() {
    use canic_management_protocol::{Failure, Phase, Status};
    let mut state = managed_state(false);
    state["management"] = serde_json::to_value(Status {
        phase: Phase::CreateIssued,
        attempts: [1, 0, 0],
        requests: [Some([5; 32]), None, None],
        failure: Some(Failure::Uncertain),
        ..Default::default()
    })
    .unwrap();
    let before = state.clone();
    let text = render(&state, Path::new("journal")).unwrap();
    for expected in [
        "ROOT EFFECT",
        "DISPATCHES",
        "REQUEST HASH",
        "Retained child: unknown",
        "Creation refund: unknown",
        "call outcome is uncertain",
        "no second creation is authorized",
        "deployment completion is not established",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
    assert!(!text.contains("Creation refund: 0"));
    assert_eq!(state, before);
    state["management"]["worker"] = "child\n\u{1b}".into();
    state["management"]["creation_refund"] = 0.into();
    let text = render(&state, Path::new("journal")).unwrap();
    assert!(text.contains("Creation refund: 0 cycles"));
    assert!(text.contains("child\\n\\u{1b}"));
    assert!(!text.contains('\u{1b}'));
}

#[test]
fn upgrade_diagnostics_do_not_turn_failure_expiry_or_funding_into_retry_permission() {
    use canic_management_protocol::{
        Failure,
        upgrade::{Phase, Status},
    };
    let mut state = managed_state(true);
    for (phase, failure, expected) in [
        (
            Phase::InstallIssued,
            Failure::Rejected(5),
            "original effect limits still apply",
        ),
        (
            Phase::Verified,
            Failure::Mismatch,
            "A trapped owner may still need maintenance",
        ),
        (
            Phase::Expired,
            Failure::InvalidReply,
            "expiry does not undo issued effects",
        ),
        (
            Phase::ReserveBlocked,
            Failure::Uncertain,
            "adding cycles does not reset",
        ),
    ] {
        state["management"] = serde_json::to_value(Status {
            phase,
            attempts: [1, 1],
            requests: [Some([7; 32]); 2],
            previous_version: Some(9),
            failure: Some(failure),
            ..Default::default()
        })
        .unwrap();
        let before = state.clone();
        let text = render(&state, Path::new("journal")).unwrap();
        assert!(text.contains(expected), "{text}");
        assert!(text.contains("Child version: 9 → unknown"));
        assert!(text.contains("Retained failure:"));
        assert!(text.contains("no retry authority granted"));
        assert!(!text.contains("retain completion. Root readiness"));
        assert_eq!(state, before);
    }
    state["management"]["failure"] = serde_json::json!({"Rejected":5});
    assert!(
        render(&state, Path::new("journal"))
            .unwrap()
            .contains("provider rejected the call (code 5)")
    );
}

#[test]
fn ready_root_evidence_requires_a_separate_receipt_and_unknown_evidence_stays_unknown() {
    use canic_management_protocol::upgrade::{Phase, Status};
    let mut state = managed_state(true);
    state["management"] = serde_json::Value::Null;
    let text = render(&state, Path::new("journal")).unwrap();
    assert!(text.contains("admission and progress are unknown"));
    assert!(!text.contains("Root readiness alone"));
    state["management"] = serde_json::to_value(Status {
        phase: Phase::Ready,
        attempts: [1, 1],
        requests: [Some([1; 32]); 2],
        previous_version: Some(9),
        observed_version: Some(10),
        ..Default::default()
    })
    .unwrap();
    let text = render(&state, Path::new("journal")).unwrap();
    assert!(text.contains("Child version: 9 → 10"));
    assert!(text.contains("Root readiness alone is not a receipt"));
    state["receipt"] = serde_json::json!({"operation":"a".repeat(64), "canister":"child", "module_hash":"b".repeat(64), "controller":"root"});
    let text = render(&state, Path::new("journal")).unwrap();
    assert!(text.contains("use fleet show for current observations"));
    assert!(!text.contains("Root readiness alone is not a receipt"));
    state["management"]["failure"] = "future-unknown-failure".into();
    assert!(render(&state, Path::new("journal")).is_err());
}

#[test]
fn fleet_budget_and_utc_labels_preserve_exact_boundaries() {
    assert_eq!(
        super::dispatch(100, 100),
        "when approved prerequisites are ready"
    );
    assert_eq!(
        super::dispatch(100, 200),
        "1970-01-01 00:00:00 UTC (original admission cutoff)"
    );
    assert_eq!(super::cycles(0), "0 cycles");
    assert_eq!(
        super::cycles(3_000_000_000_000),
        "3T cycles (3000000000000)"
    );
    assert_eq!(
        super::cycles(999_999_999_999),
        "999.999999999B cycles (999999999999)"
    );
    assert_eq!(
        super::cycles(1_000_000_000_001),
        "1.000000000001T cycles (1000000000001)"
    );
    assert_eq!(
        super::cycles(u64::MAX),
        "18446744.073709551615T cycles (18446744073709551615)"
    );
    assert_eq!(super::utc(0), "1970-01-01 00:00:00 UTC");
    assert_eq!(
        super::utc(951_782_400_000_000_000),
        "2000-02-29 00:00:00 UTC"
    );
    assert_eq!(super::utc(u64::MAX), "2554-07-21 23:34:33 UTC");
}
