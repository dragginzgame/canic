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
