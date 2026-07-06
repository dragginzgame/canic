use super::*;

#[test]
fn parse_bootstrap_status_accepts_plain_record() {
    let status = parse_bootstrap_status_value(&json!({
        "ready": false,
        "phase": "root:init:create_canisters",
        "last_error": null
    }))
    .expect("plain bootstrap status must parse");

    assert!(!status.ready);
    assert_eq!(status.phase, "root:init:create_canisters");
    assert_eq!(status.last_error, None);
}

#[test]
fn parse_bootstrap_status_accepts_wrapped_ok_record() {
    let status = parse_bootstrap_status_value(&json!({
        "Ok": {
            "ready": false,
            "phase": "failed",
            "last_error": "registry phase failed"
        }
    }))
    .expect("wrapped bootstrap status must parse");

    assert!(!status.ready);
    assert_eq!(status.phase, "failed");
    assert_eq!(status.last_error.as_deref(), Some("registry phase failed"));
}

#[test]
fn parse_bootstrap_status_rejects_icp_cli_response_candid() {
    assert_eq!(
        parse_bootstrap_status_value(&json!({
            "response_candid": r#"(
  record {
    89_620_959 = opt "registry phase failed";
    3_253_282_875 = "failed";
    3_870_990_435 = false;
  },
)"#
        })),
        None
    );
}

#[test]
fn parses_quiet_canister_create_output() {
    assert_eq!(
        parse_created_canister_id("Created canister:\nt63gs-up777-77776-aaaba-cai\n"),
        Some("t63gs-up777-77776-aaaba-cai".to_string())
    );
    assert_eq!(parse_created_canister_id("created root\n"), None);
}

#[test]
fn parses_json_canister_ids() {
    assert_eq!(
        parse_created_canister_id(r#"{"canister_id":"t63gs-up777-77776-aaaba-cai"}"#),
        Some("t63gs-up777-77776-aaaba-cai".to_string())
    );
    assert_eq!(
        parse_created_canister_id(r#"{"id":"t63gs-up777-77776-aaaba-cai","name":"root"}"#),
        Some("t63gs-up777-77776-aaaba-cai".to_string())
    );
    assert_eq!(
        parse_canister_id_json(&json!([{ "principal": "t63gs-up777-77776-aaaba-cai" }])),
        Some("t63gs-up777-77776-aaaba-cai".to_string())
    );
    assert_eq!(
        parse_created_canister_id(r#"{"canister_id":"not-a-principal"}"#),
        None
    );
}

#[test]
fn detects_missing_canister_id_errors() {
    assert!(is_missing_canister_id_error(
        "Error: failed to lookup canister ID for canister 'root' in environment 'local'"
    ));
    assert!(is_missing_canister_id_error(
        "could not find ID for canister 'root' in environment 'local'"
    ));
    assert!(!is_missing_canister_id_error(
        "Error: failed to connect to replica"
    ));
}
