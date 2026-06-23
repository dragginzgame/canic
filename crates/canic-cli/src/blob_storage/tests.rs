use super::*;
use crate::{blob_storage::options::BlobStorageOptions, cli::globals, run};
use std::ffi::OsString;

#[test]
fn parses_status_options_with_required_target() {
    let command = BlobStorageOptions::parse([
        OsString::from("status"),
        OsString::from("local"),
        OsString::from("backend"),
        OsString::from("--json"),
        OsString::from(globals::INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
        OsString::from(globals::INTERNAL_ICP_OPTION),
        OsString::from("/bin/icp"),
    ])
    .expect("parse status options");

    let options = match command {
        options::BlobStorageCommand::Status(options) => options,
        other => panic!("expected status options, got {other:?}"),
    };

    assert_eq!(options.deployment, "local");
    assert_eq!(options.canister, "backend");
    assert_eq!(options.common.network, "local");
    assert_eq!(options.common.icp, "/bin/icp");
    assert!(options.json);
}

#[test]
fn rejects_missing_target() {
    let err = BlobStorageOptions::parse([OsString::from("status"), OsString::from("local")])
        .expect_err("target should be required");

    std::assert_matches!(err, BlobStorageCommandError::Usage(_));
}

#[test]
fn parses_fund_cycles_strictly() {
    let command = BlobStorageOptions::parse([
        OsString::from("fund"),
        OsString::from("local"),
        OsString::from("backend"),
        OsString::from("--cycles"),
        OsString::from("1000000000000"),
        OsString::from("--dry-run"),
    ])
    .expect("parse fund options");

    let options = match command {
        options::BlobStorageCommand::Fund(options) => options,
        other => panic!("expected fund options, got {other:?}"),
    };

    assert_eq!(options.cycles, 1_000_000_000_000);
    assert!(options.dry_run);
}

#[test]
fn rejects_non_decimal_cycle_syntax() {
    for value in ["0", "1_000", "1T", "1.5", "1e12", "-1"] {
        let err = BlobStorageOptions::parse([
            OsString::from("fund"),
            OsString::from("local"),
            OsString::from("backend"),
            OsString::from("--cycles"),
            OsString::from(value),
        ])
        .expect_err("invalid cycles should fail");

        std::assert_matches!(err, BlobStorageCommandError::Usage(_));
    }
}

#[test]
fn top_level_forwards_global_icp_and_network() {
    let err = run([
        OsString::from("--icp"),
        OsString::from("/bin/icp"),
        OsString::from("--network"),
        OsString::from("local"),
        OsString::from("blob-storage"),
        OsString::from("fund"),
        OsString::from("demo"),
        OsString::from("backend"),
        OsString::from("--cycles"),
        OsString::from("0"),
    ])
    .expect_err("invalid cycles should be parsed after global options");

    let message = err.to_string();
    assert!(message.contains("--cycles must be greater than zero"));
}

#[test]
fn json_reported_errors_use_structured_blob_storage_shape() {
    let err = BlobStorageCommandError::ResponseParse.with_json_report("local", "backend");
    let cli_error = crate::CliError::from(err);
    let output = crate::render_cli_error(&cli_error);
    let value = serde_json::from_str::<serde_json::Value>(&output).expect("error json");

    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["kind"], "blob_storage_error");
    assert_eq!(value["deployment"], "local");
    assert_eq!(value["target"]["input"], "backend");
    assert_eq!(value["target"]["role"], serde_json::Value::Null);
    assert_eq!(value["target"]["canister_id"], serde_json::Value::Null);
    assert_eq!(value["target"]["candid_source"], serde_json::Value::Null);
    assert_eq!(value["error"]["code"], "response_parse_failed");
    assert_eq!(value["error"]["exit_code"], 3);
    assert_eq!(crate::cli_error_exit_code(&cli_error), 3);
}

#[test]
fn non_json_blob_storage_errors_keep_top_level_prefix() {
    let cli_error = crate::CliError::from(BlobStorageCommandError::ResponseParse);

    assert_eq!(
        crate::render_cli_error(&cli_error),
        "blob-storage: failed to parse blob-storage canister response"
    );
    assert_eq!(crate::cli_error_exit_code(&cli_error), 3);
}

#[test]
fn json_error_codes_distinguish_candid_and_transport_failures() {
    let candid = BlobStorageCommandError::CandidUnavailable {
        deployment: "local".to_string(),
        target: "backend".to_string(),
    }
    .with_json_report("local", "backend");
    let transport = BlobStorageCommandError::IcpFailed {
        command: "icp canister call".to_string(),
        stderr: "network unavailable".to_string(),
    }
    .with_json_report("local", "backend");

    let candid = serde_json::from_str::<serde_json::Value>(
        &candid.json_error_report().expect("candid error json"),
    )
    .expect("decode candid error json");
    let transport = serde_json::from_str::<serde_json::Value>(
        &transport.json_error_report().expect("transport error json"),
    )
    .expect("decode transport error json");

    assert_eq!(candid["error"]["code"], "candid_unavailable");
    assert_eq!(candid["error"]["exit_code"], 1);
    assert_eq!(transport["error"]["code"], "transport_failed");
    assert_eq!(transport["error"]["exit_code"], 2);
}

#[test]
fn renders_sync_gateways_dry_run_json_shape() {
    let target = model::BlobStorageTarget::resolved(
        "backend",
        Some("backend".to_string()),
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
        "installed_deployment",
    );
    let result = model::BlobStorageActionResult::dry_run(
        "local",
        model::BlobStorageActionName::SyncGateways,
        target,
        canic_core::protocol::BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
        "update",
        "icp canister call backend _immutableObjectStorageUpdateGatewayPrincipals () --json"
            .to_string(),
        None,
    );
    let value = serde_json::to_value(&result).expect("serialize result");

    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["kind"], "blob_storage_sync_gateways_result");
    assert_eq!(value["deployment"], "local");
    assert_eq!(value["target"]["input"], "backend");
    assert_eq!(value["target"]["role"], "backend");
    assert_eq!(
        value["target"]["canister_id"],
        "rrkah-fqaaa-aaaaa-aaaaq-cai"
    );
    assert_eq!(value["target"]["candid_source"], "installed_deployment");
    assert_eq!(value["action"]["name"], "sync_gateways");
    assert_eq!(value["action"]["mode"], "update");
    assert_eq!(value["action"]["dry_run"], true);
}

#[test]
fn renders_sync_gateways_completed_json_shape() {
    let target = model::BlobStorageTarget::resolved(
        "backend",
        Some("backend".to_string()),
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
        "installed_deployment",
    );
    let result = model::BlobStorageActionResult::completed(
        "local",
        model::BlobStorageActionName::SyncGateways,
        target,
        canic_core::protocol::BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
        "update",
        "icp canister call backend _immutableObjectStorageUpdateGatewayPrincipals ()".to_string(),
        None,
    )
    .with_post_status(sample_status_result())
    .with_warning("post_status_unavailable");
    let value = serde_json::to_value(&result).expect("serialize result");

    assert_eq!(value["kind"], "blob_storage_sync_gateways_result");
    assert_eq!(value["action"]["name"], "sync_gateways");
    assert_eq!(value["action"]["dry_run"], false);
    assert_eq!(value["action"]["success"], true);
    assert_eq!(value["post_status"]["kind"], "blob_storage_status");
    assert_eq!(
        value["warnings"],
        serde_json::json!(["post_status_unavailable"])
    );
    let text = render::render_action_result(&result);
    assert_eq!(
        text.lines().next().expect("first line"),
        "Blob storage sync_gateways completed"
    );
    assert!(text.contains("Warnings:\n  - post_status_unavailable"));
    assert!(text.contains("Post status:\n  Blob storage status: backend"));
    assert!(text.contains("  Readiness: ready"));
}

#[test]
fn renders_fund_dry_run_plain_text() {
    let target = model::BlobStorageTarget::resolved(
        "backend",
        Some("backend".to_string()),
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
        "installed_deployment",
    );
    let result = model::BlobStorageActionResult::dry_run(
        "local",
        model::BlobStorageActionName::Fund,
        target,
        canic_core::protocol::BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
        "update",
        "icp canister call backend _immutableObjectStorageFundFromProjectCycles (100 : nat)"
            .to_string(),
        Some(100),
    );

    assert_eq!(
        render::render_action_result(&result),
        [
            "Blob storage fund dry run",
            "Deployment: local",
            "Target: backend",
            "Method: _immutableObjectStorageFundFromProjectCycles",
            "Mode: update",
            "Requested cycles: 100",
        ]
        .join("\n")
    );
    assert_eq!(
        render::render_dry_run_command(&result),
        "Command: icp canister call backend _immutableObjectStorageFundFromProjectCycles (100 : nat)"
    );
}

#[test]
fn parses_funding_report_json_into_stable_cli_shape() {
    let output = serde_json::json!({
        "Ok": {
            "requested_cycles": "1000",
            "attached_cycles": "750",
            "project_cycles_before": "5000",
            "project_cycles_after": "4250",
            "reserve_cycles": "2000",
            "cashier_total_after": "1750",
            "skipped_reason": null
        }
    })
    .to_string();

    let report = parse::parse_funding_report(&output).expect("parse funding report");

    assert_eq!(report.requested_cycles, "1000");
    assert_eq!(report.attached_cycles, "750");
    assert_eq!(report.project_cycles_before, "5000");
    assert_eq!(report.project_cycles_after, "4250");
    assert_eq!(report.reserve_cycles, "2000");
    assert_eq!(report.cashier_total_after, "1750");
    assert_eq!(report.skipped_reason, None);
}

#[test]
fn renders_fund_completed_report_json_and_plain_text() {
    let target = model::BlobStorageTarget::resolved(
        "backend",
        Some("backend".to_string()),
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
        "installed_deployment",
    );
    let result = model::BlobStorageActionResult::completed(
        "local",
        model::BlobStorageActionName::Fund,
        target,
        canic_core::protocol::BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
        "update",
        "icp canister call backend _immutableObjectStorageFundFromProjectCycles (100 : nat) --json"
            .to_string(),
        Some(100),
    )
    .with_funding_report(model::BlobStorageFundingReport {
        requested_cycles: "100".to_string(),
        attached_cycles: "100".to_string(),
        project_cycles_before: "1000".to_string(),
        project_cycles_after: "900".to_string(),
        reserve_cycles: "200".to_string(),
        cashier_total_after: "300".to_string(),
        skipped_reason: None,
    });
    let value = serde_json::to_value(&result).expect("serialize result");

    assert_eq!(value["kind"], "blob_storage_fund_result");
    assert_eq!(value["action"]["dry_run"], false);
    assert_eq!(value["funding_report"]["requested_cycles"], "100");
    assert_eq!(value["funding_report"]["attached_cycles"], "100");
    let text = render::render_action_result(&result);
    assert!(text.contains("Blob storage fund completed"));
    assert!(text.contains("Attached cycles: 100"));
    assert!(text.contains("Project cycles: 1000 -> 900"));
    assert!(text.contains("Cashier total after: 300"));
}

#[test]
fn parses_status_json_into_stable_cli_shape() {
    let target = model::BlobStorageTarget::resolved(
        "backend",
        Some("backend".to_string()),
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
        "installed_deployment",
    );
    let output = serde_json::json!({
        "Ok": {
            "payment_model": { "ProjectAsPaymentAccount": null },
            "cashier_canister_id": ["ryjl3-tyaaa-aaaaa-aaaba-cai"],
            "payment_account": ["rrkah-fqaaa-aaaaa-aaaaq-cai"],
            "cashier_balance": ["100"],
            "min_upload_balance": ["500"],
            "target_upload_balance": ["1000"],
            "project_cycles_reserve": ["2000"],
            "project_cycles_available": "3000",
            "gateway_principal_count": 0,
            "last_gateway_principal_sync_at_ns": null,
            "gateway_principal_sync_action": { "SkippedReadOnlyStatus": null },
            "funding_status": {
                "FundingRequired": {
                    "requested_cycles": "900"
                }
            },
            "ready": false,
            "blockers": [
                { "GatewayPrincipalsMissing": null },
                { "InsufficientCashierBalance": null }
            ],
            "warnings": [
                { "GatewayPrincipalSetEmpty": null }
            ]
        }
    })
    .to_string();

    let status = parse::parse_status_result("local", target, &output).expect("parse status");
    let value = serde_json::to_value(&status).expect("serialize status");

    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["kind"], "blob_storage_status");
    assert_eq!(value["configured"], true);
    assert_eq!(value["cashier"]["balance_cycles"], "100");
    assert_eq!(value["policy"]["project_cycles_available"], "3000");
    assert_eq!(value["gateways"]["principal_count"], 0);
    assert_eq!(value["funding"]["status"], "funding_needed");
    assert_eq!(value["funding"]["requested_cycles"], "900");
    assert_eq!(value["readiness"]["state"], "blocked");
    assert_eq!(value["readiness"]["ready_for_upload"], false);
    assert_eq!(
        value["readiness"]["blockers"],
        serde_json::json!(["gateway_principals_empty", "cashier_balance_below_min"])
    );
    assert_eq!(value["next"][0]["action"], "sync_gateways");
    assert_eq!(
        value["next"][0]["command"],
        "canic blob-storage sync-gateways local backend"
    );
    assert_eq!(
        value["next"][1]["command"],
        "canic blob-storage fund local backend --cycles 900"
    );
}

#[test]
fn renders_status_plain_text_with_blockers_and_next_actions() {
    let target = model::BlobStorageTarget::resolved(
        "backend",
        Some("backend".to_string()),
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
        "installed_deployment",
    );
    let output = serde_json::json!({
        "Ok": {
            "payment_model": { "NotConfigured": null },
            "cashier_canister_id": null,
            "payment_account": null,
            "cashier_balance": null,
            "min_upload_balance": null,
            "target_upload_balance": null,
            "project_cycles_reserve": null,
            "project_cycles_available": "3000",
            "gateway_principal_count": 0,
            "last_gateway_principal_sync_at_ns": null,
            "gateway_principal_sync_action": { "SkippedConfigMissing": null },
            "funding_status": { "NotConfigured": null },
            "ready": false,
            "blockers": [
                { "NotConfigured": null }
            ],
            "warnings": []
        }
    })
    .to_string();

    let status = parse::parse_status_result("local", target, &output).expect("parse status");

    assert_eq!(
        render::render_status_result(&status),
        [
            "Blob storage status: backend",
            "Deployment: local",
            "Target: rrkah-fqaaa-aaaaa-aaaaq-cai",
            "Configured: no",
            "Cashier: -",
            "Payment account: -",
            "Cashier balance: -",
            "Upload balance: min -, target -",
            "Project reserve: -",
            "Project cycles available: 3000",
            "Gateways: 0 synced",
            "Last gateway sync: never",
            "Readiness: blocked",
            "Blockers:",
            "  - not_configured",
        ]
        .join("\n")
    );
}

fn sample_status_result() -> model::BlobStorageStatusResult {
    let target = model::BlobStorageTarget::resolved(
        "backend",
        Some("backend".to_string()),
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
        "installed_deployment",
    );
    let output = serde_json::json!({
        "Ok": {
            "payment_model": { "ProjectAsPaymentAccount": null },
            "cashier_canister_id": ["ryjl3-tyaaa-aaaaa-aaaba-cai"],
            "payment_account": ["rrkah-fqaaa-aaaaa-aaaaq-cai"],
            "cashier_balance": ["1000"],
            "min_upload_balance": ["500"],
            "target_upload_balance": ["1000"],
            "project_cycles_reserve": ["2000"],
            "project_cycles_available": "3000",
            "gateway_principal_count": 1,
            "last_gateway_principal_sync_at_ns": ["123"],
            "gateway_principal_sync_action": { "SkippedReadOnlyStatus": null },
            "funding_status": { "NotNeeded": null },
            "ready": true,
            "blockers": [],
            "warnings": []
        }
    })
    .to_string();

    parse::parse_status_result("local", target, &output).expect("sample status")
}
