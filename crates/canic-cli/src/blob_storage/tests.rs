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
        OsString::from("sync-gateways"),
        OsString::from("demo"),
        OsString::from("backend"),
    ])
    .expect_err("non-dry-run is not implemented yet");

    let message = err.to_string();
    assert!(message.contains("blob-storage sync-gateways requires --dry-run"));
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
        value["next"][1]["command"],
        "canic blob-storage fund local backend --cycles 900 --dry-run"
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
