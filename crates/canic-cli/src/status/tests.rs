use super::*;

fn state() -> serde_json::Value {
    serde_json::json!({
        "operation": "a".repeat(64),
        "plan": Plan {
            version: 2,
        management: None,
            network: Network::Mainnet,
            controller: "aaaaa-aa".into(),
            artifact: canic_host::Artifact { sha256: "b".repeat(64), bytes: 8 },
            target: None,
            previous_module: None,
            creation_cycles: 1_000_000_000_000,
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
    let mut state = state();
    state["plan"]["target"] = "aaaaa-aa".into();
    state["plan"]["previous_module"] = "c".repeat(64).into();
    state["plan"]["creation_cycles"] = 0.into();
    let text = render(&state, Path::new("journal")).unwrap();
    assert!(text.contains("Planned action: upgrade existing canister"));
    assert!(text.contains("Upgrade target: aaaaa-aa"));
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
    value["plan"]["management"] = serde_json::json!({
        "root": "rrkah-fqaaa-aaaaa-aaaaq-cai", "root_module": "d".repeat(64),
        "execute_at_ns": 1_500_000_000_u64,
    });
    value["effects"] = serde_json::json!({"stage":"reply_retained"});
    if upgrade {
        value["plan"]["target"] = "ryjl3-tyaaa-aaaaa-aaaba-cai".into();
        value["plan"]["previous_module"] = "c".repeat(64).into();
        value["plan"]["creation_cycles"] = 0.into();
    }
    value
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
            "persistent uncertainty requires owner recovery review",
        ),
        (
            Phase::Verified,
            Failure::Mismatch,
            "no CLI recovery command is qualified yet",
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
