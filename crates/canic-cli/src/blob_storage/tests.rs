use super::*;
use crate::{blob_storage::options::BlobStorageOptions, cli::globals, run};
use candid::{CandidType, Encode, Nat, Principal};
use canic_core::{
    cdk::utils::hash::hex_bytes,
    dto::{
        blob_storage::{
            BlobProjectCyclesTopUpReport, BlobStorageBillingWarning,
            BlobStorageFundingStatus as BlobStorageFundingStatusDto,
            BlobStorageGatewayPrincipalSyncAction, BlobStoragePaymentModelStatus,
            BlobStorageReadinessBlocker, BlobStorageStatusResponse,
        },
        error::Error as CanicError,
    },
};
use std::{cell::RefCell, collections::VecDeque, ffi::OsString, path::PathBuf};

#[test]
fn parses_status_options_with_required_target() {
    let command = BlobStorageOptions::parse([
        OsString::from("status"),
        OsString::from("local"),
        OsString::from("backend"),
        OsString::from("--json"),
        OsString::from(globals::INTERNAL_ENVIRONMENT_OPTION),
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
    assert_eq!(options.common.environment, "local");
    assert_eq!(options.common.icp, "/bin/icp");
    assert!(options.json);
    assert!(!options.check_ready);
}

#[test]
fn parses_status_check_ready_option() {
    let command = BlobStorageOptions::parse([
        OsString::from("status"),
        OsString::from("local"),
        OsString::from("backend"),
        OsString::from("--check-ready"),
    ])
    .expect("parse status check-ready options");

    let options = match command {
        options::BlobStorageCommand::Status(options) => options,
        other => panic!("expected status options, got {other:?}"),
    };

    assert!(options.check_ready);
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
fn funding_response_reports_the_malformed_field() {
    let too_large = "340282366920938463463374607431768211456"
        .parse::<Nat>()
        .expect("parse over-u128 Nat");
    let output = response_json(&Ok::<_, CanicError>(BlobProjectCyclesTopUpReport {
        requested_cycles: too_large,
        attached_cycles: Nat::from(0_u8),
        project_cycles_before: Nat::from(0_u8),
        project_cycles_after: Nat::from(0_u8),
        reserve_cycles: Nat::from(0_u8),
        cashier_total_after: Nat::from(0_u8),
        skipped_reason: None,
    }));

    assert!(matches!(
        parse::parse_funding_report(&output),
        Err(parse::BlobStorageParseError::NatOutOfRange {
            kind: parse::BlobStorageResponseKind::Funding,
            field: "requested_cycles"
        })
    ));
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

        std::assert_matches!(err, BlobStorageCommandError::InvalidCycles(_));
    }
}

#[test]
fn top_level_forwards_global_icp_and_environment() {
    let err = run([
        OsString::from("--icp"),
        OsString::from("/bin/icp"),
        OsString::from("--environment"),
        OsString::from("local"),
        OsString::from("blob-storage"),
        OsString::from("fund"),
        OsString::from("demo"),
        OsString::from("backend"),
        OsString::from("--cycles"),
        OsString::from("0"),
    ])
    .expect_err("invalid cycles should be parsed after global options");

    std::assert_matches!(
        err,
        crate::CliError::BlobStorage(BlobStorageCommandError::InvalidCycles(_))
    );
}

#[test]
fn json_reported_errors_use_structured_blob_storage_shape() {
    let err = BlobStorageCommandError::ResponseValueOutOfRange {
        response_kind: "status",
        field: "sample",
    }
    .with_json_report("local", "backend");
    let cli_error = crate::CliError::from(err);
    let output = crate::render_cli_error(&cli_error);
    let value = serde_json::from_str::<serde_json::Value>(&output).expect("error json");

    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["kind"], model::BLOB_STORAGE_ERROR_KIND);
    assert_eq!(value["deployment"], "local");
    assert_eq!(value["target"]["input"], "backend");
    assert_eq!(value["target"]["role"], serde_json::Value::Null);
    assert_eq!(value["target"]["canister_id"], serde_json::Value::Null);
    assert_eq!(value["target"]["candid_source"], serde_json::Value::Null);
    assert_eq!(
        value["error"]["code"],
        model::BLOB_STORAGE_ERROR_CODE_RESPONSE_PARSE_FAILED
    );
    assert_eq!(value["error"]["exit_code"], 3);
    assert_eq!(crate::cli_error_exit_code(&cli_error), 3);
}

#[test]
fn non_json_blob_storage_errors_keep_top_level_prefix() {
    let cli_error = crate::CliError::from(BlobStorageCommandError::ResponseValueOutOfRange {
        response_kind: "status",
        field: "sample",
    });

    assert_eq!(
        crate::render_cli_error(&cli_error),
        "blob-storage: blob-storage status response field `sample` exceeds u128"
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
    let transport = BlobStorageCommandError::Icp(IcpCommandError::Failed {
        command: "icp canister call".to_string(),
        stderr: "environment unavailable".to_string(),
    })
    .with_json_report("local", "backend");

    let candid = serde_json::from_str::<serde_json::Value>(
        &candid.json_error_report().expect("candid error json"),
    )
    .expect("decode candid error json");
    let transport = serde_json::from_str::<serde_json::Value>(
        &transport.json_error_report().expect("transport error json"),
    )
    .expect("decode transport error json");

    assert_eq!(
        candid["error"]["code"],
        model::BLOB_STORAGE_ERROR_CODE_CANDID_UNAVAILABLE
    );
    assert_eq!(candid["error"]["exit_code"], 1);
    assert_eq!(
        transport["error"]["code"],
        model::BLOB_STORAGE_ERROR_CODE_TRANSPORT_FAILED
    );
    assert_eq!(transport["error"]["exit_code"], 2);

    let io = BlobStorageCommandError::Icp(IcpCommandError::Io(std::io::Error::other("sample")))
        .with_json_report("local", "backend");
    let io =
        serde_json::from_str::<serde_json::Value>(&io.json_error_report().expect("I/O error json"))
            .expect("decode I/O error json");

    assert_eq!(
        io["error"]["code"],
        model::BLOB_STORAGE_ERROR_CODE_TARGET_RESOLUTION_FAILED
    );
    assert_eq!(io["error"]["exit_code"], 1);
}

#[test]
fn renders_sync_gateways_dry_run_json_shape() {
    let target = model::BlobStorageTarget::from_installed_deployment(
        "backend",
        Some("backend".to_string()),
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
    );
    let result = model::BlobStorageActionResult::dry_run(
        "local",
        model::BlobStorageActionName::SyncGateways,
        target,
        canic_core::protocol::BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
        model::BlobStorageMethodMode::Update,
        "icp canister call backend _immutableObjectStorageUpdateGatewayPrincipals () --json"
            .to_string(),
        None,
    );
    let value = serde_json::to_value(&result).expect("serialize result");

    assert_eq!(value["schema_version"], 1);
    assert_eq!(
        value["kind"],
        model::BlobStorageActionName::SyncGateways
            .result_kind()
            .label()
    );
    assert_eq!(value["deployment"], "local");
    assert_eq!(value["target"]["input"], "backend");
    assert_eq!(value["target"]["role"], "backend");
    assert_eq!(
        value["target"]["canister_id"],
        "rrkah-fqaaa-aaaaa-aaaaq-cai"
    );
    assert_eq!(
        value["target"]["candid_source"],
        model::BLOB_STORAGE_CANDID_SOURCE_INSTALLED_DEPLOYMENT
    );
    assert_eq!(
        value["action"]["name"],
        model::BlobStorageActionName::SyncGateways.label()
    );
    assert_eq!(value["action"]["mode"], "update");
    assert_eq!(value["action"]["dry_run"], true);
}

#[test]
fn renders_sync_gateways_completed_json_shape() {
    let target = model::BlobStorageTarget::from_installed_deployment(
        "backend",
        Some("backend".to_string()),
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
    );
    let result = model::BlobStorageActionResult::completed(
        "local",
        model::BlobStorageActionName::SyncGateways,
        target,
        canic_core::protocol::BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
        model::BlobStorageMethodMode::Update,
        "icp canister call backend _immutableObjectStorageUpdateGatewayPrincipals ()".to_string(),
        None,
    )
    .with_post_status(sample_status_result())
    .with_warning(model::BLOB_STORAGE_WARNING_POST_STATUS_UNAVAILABLE);
    let value = serde_json::to_value(&result).expect("serialize result");

    assert_eq!(
        value["kind"],
        model::BlobStorageActionName::SyncGateways
            .result_kind()
            .label()
    );
    assert_eq!(
        value["action"]["name"],
        model::BlobStorageActionName::SyncGateways.label()
    );
    assert_eq!(value["action"]["dry_run"], false);
    assert_eq!(value["action"]["success"], true);
    assert_eq!(
        value["post_status"]["kind"],
        model::BLOB_STORAGE_STATUS_KIND
    );
    assert_eq!(
        value["warnings"],
        serde_json::json!([model::BLOB_STORAGE_WARNING_POST_STATUS_UNAVAILABLE])
    );
    let text = render::render_action_result(&result);
    assert_eq!(
        text.lines().next().expect("first line"),
        "Blob storage sync_gateways completed"
    );
    assert!(text.contains(&format!(
        "Warnings:\n  - {}",
        model::BLOB_STORAGE_WARNING_POST_STATUS_UNAVAILABLE
    )));
    assert!(text.contains("Post status:\n  Blob storage status: backend"));
    assert!(text.contains(&format!(
        "  Readiness: {}",
        model::BLOB_STORAGE_READINESS_READY
    )));
}

#[test]
fn renders_fund_dry_run_plain_text() {
    let target = model::BlobStorageTarget::from_installed_deployment(
        "backend",
        Some("backend".to_string()),
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
    );
    let result = model::BlobStorageActionResult::dry_run(
        "local",
        model::BlobStorageActionName::Fund,
        target,
        canic_core::protocol::BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
        model::BlobStorageMethodMode::Update,
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
    let output = response_json(&Ok::<_, CanicError>(BlobProjectCyclesTopUpReport {
        requested_cycles: Nat::from(1_000_u64),
        attached_cycles: Nat::from(750_u64),
        project_cycles_before: Nat::from(5_000_u64),
        project_cycles_after: Nat::from(4_250_u64),
        reserve_cycles: Nat::from(2_000_u64),
        cashier_total_after: Nat::from(1_750_u64),
        skipped_reason: None,
    }));

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
    let target = model::BlobStorageTarget::from_installed_deployment(
        "backend",
        Some("backend".to_string()),
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
    );
    let result = model::BlobStorageActionResult::completed(
        "local",
        model::BlobStorageActionName::Fund,
        target,
        canic_core::protocol::BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
        model::BlobStorageMethodMode::Update,
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

    assert_eq!(
        value["kind"],
        model::BlobStorageActionName::Fund.result_kind().label()
    );
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
    let target = model::BlobStorageTarget::from_installed_deployment(
        "backend",
        Some("backend".to_string()),
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
    );
    let output = status_response_from(StatusResponseFixture {
        cashier_balance: Some(Nat::from(100_u64)),
        gateway_principal_count: 0,
        last_gateway_principal_sync_at_ns: None,
        funding_status: BlobStorageFundingStatusDto::FundingRequired {
            requested_cycles: Nat::from(900_u64),
        },
        ready: false,
        blockers: vec![
            BlobStorageReadinessBlocker::GatewayPrincipalsMissing,
            BlobStorageReadinessBlocker::InsufficientCashierBalance,
        ],
        warnings: vec![BlobStorageBillingWarning::GatewayPrincipalSetEmpty],
        ..StatusResponseFixture::default()
    });

    let status = parse::parse_status_result("local", target, &output).expect("parse status");
    let value = serde_json::to_value(&status).expect("serialize status");

    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["kind"], model::BLOB_STORAGE_STATUS_KIND);
    assert_eq!(value["configured"], true);
    assert_eq!(value["cashier"]["balance_cycles"], "100");
    assert_eq!(value["policy"]["project_cycles_available"], "3000");
    assert_eq!(value["gateways"]["principal_count"], 0);
    assert_eq!(
        value["funding"]["status"],
        model::BLOB_STORAGE_CODE_FUNDING_NEEDED
    );
    assert_eq!(
        status.funding.status,
        model::BlobStorageFundingStatusCode::FundingNeeded
    );
    assert_eq!(value["funding"]["requested_cycles"], "900");
    assert_eq!(
        value["readiness"]["state"],
        model::BLOB_STORAGE_READINESS_BLOCKED
    );
    assert_eq!(
        status.readiness.state,
        model::BlobStorageReadinessState::Blocked
    );
    assert_eq!(value["readiness"]["ready_for_upload"], false);
    assert_eq!(
        value["readiness"]["blockers"],
        serde_json::json!([
            model::BLOB_STORAGE_CODE_GATEWAY_PRINCIPALS_EMPTY,
            model::BLOB_STORAGE_CODE_CASHIER_BALANCE_BELOW_MIN
        ])
    );
    assert_eq!(
        value["next"][0]["action"],
        model::BlobStorageActionName::SyncGateways.label()
    );
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
fn parses_ready_status_with_warnings_as_warning_state() {
    let target = model::BlobStorageTarget::from_installed_deployment(
        "backend",
        Some("backend".to_string()),
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
    );
    let output = status_response_from(StatusResponseFixture {
        gateway_principal_count: 0,
        last_gateway_principal_sync_at_ns: None,
        warnings: vec![BlobStorageBillingWarning::GatewayPrincipalSetEmpty],
        ..StatusResponseFixture::default()
    });

    let status = parse::parse_status_result("local", target, &output).expect("parse status");

    assert_eq!(
        status.readiness.state,
        model::BlobStorageReadinessState::Warning
    );
    assert!(status.readiness.ready_for_upload);
    assert_eq!(
        status.readiness.warnings,
        vec![model::BLOB_STORAGE_CODE_GATEWAY_PRINCIPALS_EMPTY.to_string()]
    );
    assert!(status.next.is_empty());
    check_status_ready_for_upload(&status).expect("warning state is still ready for upload");
}

#[test]
fn status_check_ready_fails_with_exit_4_when_upload_not_ready() {
    let target = model::BlobStorageTarget::from_installed_deployment(
        "backend",
        Some("backend".to_string()),
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
    );
    let status = parse::parse_status_result("local", target, &status_response(0, false, "900"))
        .expect("parse blocked status");

    let err = check_status_ready_for_upload(&status).expect_err("status should be blocked");

    assert_eq!(err.exit_code(), 4);
    assert_eq!(
        err.command_error_code(),
        model::BLOB_STORAGE_ERROR_CODE_READINESS_CHECK_FAILED
    );
    assert_eq!(
        err.to_string(),
        "readiness check failed: state=blocked; blockers=gateway_principals_empty"
    );
    std::assert_matches!(
        err,
        BlobStorageCommandError::ReadinessCheckFailed {
            message: _,
            state,
            blockers,
            warnings
        } if state == model::BLOB_STORAGE_READINESS_BLOCKED
            && blockers == vec![model::BLOB_STORAGE_CODE_GATEWAY_PRINCIPALS_EMPTY.to_string()]
            && warnings.is_empty()
    );
}

#[test]
fn status_check_ready_failure_message_includes_warnings_without_blockers() {
    let target = model::BlobStorageTarget::from_installed_deployment(
        "backend",
        Some("backend".to_string()),
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
    );
    let output = status_response_from(StatusResponseFixture {
        ready: false,
        warnings: vec![BlobStorageBillingWarning::CashierBalanceUnavailable],
        ..StatusResponseFixture::default()
    });
    let status = parse::parse_status_result("local", target, &output).expect("parse status");

    let err = check_status_ready_for_upload(&status).expect_err("status should not be ready");

    assert_eq!(
        err.to_string(),
        "readiness check failed: state=blocked; warnings=cashier_balance_unavailable"
    );
    std::assert_matches!(
        err,
        BlobStorageCommandError::ReadinessCheckFailed {
            message: _,
            state,
            blockers,
            warnings
        } if state == model::BLOB_STORAGE_READINESS_BLOCKED
            && blockers.is_empty()
            && warnings == vec![model::BLOB_STORAGE_CODE_CASHIER_BALANCE_UNAVAILABLE.to_string()]
    );
}

#[test]
fn scripted_operator_loop_proves_status_sync_fund_and_recheck_sequence() {
    let runtime = ScriptedBlobStorageRuntime::new([
        scripted_response(BLOB_STORAGE_STATUS, status_response(0, false, "900")),
        scripted_response(BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS, "{}".to_string()),
        scripted_response(BLOB_STORAGE_STATUS, status_response(1, false, "900")),
        scripted_response(
            BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
            funding_report_response(900, 900),
        ),
        scripted_response(BLOB_STORAGE_STATUS, status_response(1, true, "0")),
    ]);
    let common = common_options();

    let initial =
        status_result_with_runtime(&runtime, &common, "local", "backend").expect("initial status");
    let sync =
        sync_gateways_result_with_runtime(&runtime, &sync_options(common.clone())).expect("sync");
    let fund = fund_result_with_runtime(&runtime, &fund_options(common, 900)).expect("fund result");

    assert_eq!(initial.gateways.principal_count, 0);
    assert_eq!(
        initial.readiness.state,
        model::BlobStorageReadinessState::Blocked
    );
    assert_eq!(
        sync.post_status
            .as_ref()
            .expect("sync post status")
            .gateways
            .principal_count,
        1
    );
    assert_eq!(
        fund.funding_report
            .as_ref()
            .expect("funding report")
            .attached_cycles,
        "900"
    );
    assert!(
        fund.post_status
            .as_ref()
            .expect("fund post status")
            .readiness
            .ready_for_upload
    );
    assert_eq!(
        runtime.called_methods(),
        vec![
            BLOB_STORAGE_STATUS,
            BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
            BLOB_STORAGE_STATUS,
            BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
            BLOB_STORAGE_STATUS,
        ]
    );
}

#[test]
fn mutating_commands_warn_when_post_status_diagnostic_fails() {
    let sync_runtime = ScriptedBlobStorageRuntime::new([
        scripted_response(BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS, "{}".to_string()),
        scripted_response(BLOB_STORAGE_STATUS, "not-json".to_string()),
    ]);
    let fund_runtime = ScriptedBlobStorageRuntime::new([
        scripted_response(
            BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
            funding_report_response(900, 900),
        ),
        scripted_response(BLOB_STORAGE_STATUS, "not-json".to_string()),
    ]);
    let common = common_options();

    let sync = sync_gateways_result_with_runtime(&sync_runtime, &sync_options(common.clone()))
        .expect("sync should not fail on post-status diagnostic");
    let fund = fund_result_with_runtime(&fund_runtime, &fund_options(common, 900))
        .expect("fund should not fail on post-status diagnostic");

    assert_eq!(
        sync.warnings,
        vec![model::BLOB_STORAGE_WARNING_POST_STATUS_UNAVAILABLE]
    );
    assert_eq!(sync.post_status, None);
    assert_eq!(
        fund.warnings,
        vec![model::BLOB_STORAGE_WARNING_POST_STATUS_UNAVAILABLE]
    );
    assert_eq!(fund.post_status, None);
    assert_eq!(
        fund.funding_report
            .as_ref()
            .expect("funding report")
            .attached_cycles,
        "900"
    );
}

#[test]
fn renders_status_plain_text_with_blockers_and_next_actions() {
    let target = model::BlobStorageTarget::from_installed_deployment(
        "backend",
        Some("backend".to_string()),
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
    );
    let output = status_response_from(StatusResponseFixture {
        payment_model: BlobStoragePaymentModelStatus::NotConfigured,
        cashier_canister_id: None,
        payment_account: None,
        cashier_balance: None,
        min_upload_balance: None,
        target_upload_balance: None,
        project_cycles_reserve: None,
        gateway_principal_count: 0,
        last_gateway_principal_sync_at_ns: None,
        gateway_principal_sync_action: BlobStorageGatewayPrincipalSyncAction::SkippedConfigMissing,
        funding_status: BlobStorageFundingStatusDto::NotConfigured,
        ready: false,
        blockers: vec![BlobStorageReadinessBlocker::NotConfigured],
        ..StatusResponseFixture::default()
    });

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
    let target = model::BlobStorageTarget::from_installed_deployment(
        "backend",
        Some("backend".to_string()),
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
    );
    let output = status_response_from(StatusResponseFixture::default());

    parse::parse_status_result("local", target, &output).expect("sample status")
}

fn common_options() -> options::CommonOptions {
    options::CommonOptions {
        environment: "local".to_string(),
        icp: "icp".to_string(),
    }
}

fn sync_options(common: options::CommonOptions) -> options::SyncGatewaysOptions {
    options::SyncGatewaysOptions {
        common,
        deployment: "local".to_string(),
        canister: "backend".to_string(),
        json: true,
        dry_run: false,
    }
}

fn fund_options(common: options::CommonOptions, cycles: u128) -> options::FundOptions {
    options::FundOptions {
        common,
        deployment: "local".to_string(),
        canister: "backend".to_string(),
        json: true,
        dry_run: false,
        cycles,
    }
}

struct ScriptedBlobStorageRuntime {
    responses: RefCell<VecDeque<ScriptedBlobStorageResponse>>,
    calls: RefCell<Vec<String>>,
}

impl ScriptedBlobStorageRuntime {
    fn new<const N: usize>(responses: [ScriptedBlobStorageResponse; N]) -> Self {
        Self {
            responses: RefCell::new(VecDeque::from(responses)),
            calls: RefCell::new(Vec::new()),
        }
    }

    fn called_methods(&self) -> Vec<&'static str> {
        self.calls
            .borrow()
            .iter()
            .map(String::as_str)
            .map(|method| match method {
                BLOB_STORAGE_STATUS => BLOB_STORAGE_STATUS,
                BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS => BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
                BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES => BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
                _ => panic!("unexpected method {method}"),
            })
            .collect()
    }
}

impl BlobStorageRuntime for ScriptedBlobStorageRuntime {
    fn resolve_call_target(
        &self,
        _options: &options::CommonOptions,
        _deployment: &str,
        canister: &str,
        method: &str,
    ) -> Result<target::BlobStorageCallTarget, BlobStorageCommandError> {
        Ok(target::BlobStorageCallTarget {
            target: model::BlobStorageTarget::from_installed_deployment(
                canister,
                Some(canister.to_string()),
                "rrkah-fqaaa-aaaaa-aaaaq-cai",
            ),
            method_mode: if method == BLOB_STORAGE_STATUS {
                model::BlobStorageMethodMode::Query
            } else {
                model::BlobStorageMethodMode::Update
            },
            candid_path: PathBuf::from(".icp/local/canisters/backend/backend.did"),
            icp_root: PathBuf::from("."),
        })
    }

    fn call_output(
        &self,
        _options: &options::CommonOptions,
        _target: &target::BlobStorageCallTarget,
        method: &str,
        _arg: &str,
        _output: Option<&str>,
    ) -> Result<String, BlobStorageCommandError> {
        self.calls.borrow_mut().push(method.to_string());
        let response = self
            .responses
            .borrow_mut()
            .pop_front()
            .expect("scripted response");

        assert_eq!(response.method, method);
        Ok(response.output)
    }
}

struct ScriptedBlobStorageResponse {
    method: &'static str,
    output: String,
}

fn scripted_response(method: &'static str, output: String) -> ScriptedBlobStorageResponse {
    ScriptedBlobStorageResponse { method, output }
}

struct StatusResponseFixture {
    payment_model: BlobStoragePaymentModelStatus,
    cashier_canister_id: Option<Principal>,
    payment_account: Option<Principal>,
    cashier_balance: Option<Nat>,
    min_upload_balance: Option<Nat>,
    target_upload_balance: Option<Nat>,
    project_cycles_reserve: Option<Nat>,
    project_cycles_available: Nat,
    gateway_principal_count: u64,
    last_gateway_principal_sync_at_ns: Option<u64>,
    gateway_principal_sync_action: BlobStorageGatewayPrincipalSyncAction,
    funding_status: BlobStorageFundingStatusDto,
    ready: bool,
    blockers: Vec<BlobStorageReadinessBlocker>,
    warnings: Vec<BlobStorageBillingWarning>,
}

impl Default for StatusResponseFixture {
    fn default() -> Self {
        Self {
            payment_model: BlobStoragePaymentModelStatus::ProjectAsPaymentAccount,
            cashier_canister_id: Some(
                Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai").expect("cashier principal"),
            ),
            payment_account: Some(
                Principal::from_text("rrkah-fqaaa-aaaaa-aaaaq-cai").expect("payment principal"),
            ),
            cashier_balance: Some(Nat::from(1_000_u64)),
            min_upload_balance: Some(Nat::from(500_u64)),
            target_upload_balance: Some(Nat::from(1_000_u64)),
            project_cycles_reserve: Some(Nat::from(2_000_u64)),
            project_cycles_available: Nat::from(3_000_u64),
            gateway_principal_count: 1,
            last_gateway_principal_sync_at_ns: Some(123),
            gateway_principal_sync_action:
                BlobStorageGatewayPrincipalSyncAction::SkippedReadOnlyStatus,
            funding_status: BlobStorageFundingStatusDto::NotNeeded,
            ready: true,
            blockers: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

fn status_response(gateway_count: u64, ready: bool, requested_cycles: &str) -> String {
    let blockers = if ready {
        Vec::new()
    } else {
        vec![BlobStorageReadinessBlocker::GatewayPrincipalsMissing]
    };
    let funding_status = if requested_cycles == "0" {
        BlobStorageFundingStatusDto::NotNeeded
    } else {
        BlobStorageFundingStatusDto::FundingRequired {
            requested_cycles: requested_cycles
                .parse::<Nat>()
                .expect("requested cycles Nat"),
        }
    };

    status_response_from(StatusResponseFixture {
        gateway_principal_count: gateway_count,
        last_gateway_principal_sync_at_ns: None,
        funding_status,
        ready,
        blockers,
        ..StatusResponseFixture::default()
    })
}

fn status_response_from(fixture: StatusResponseFixture) -> String {
    response_json(&Ok::<_, CanicError>(BlobStorageStatusResponse {
        payment_model: fixture.payment_model,
        cashier_canister_id: fixture.cashier_canister_id,
        payment_account: fixture.payment_account,
        cashier_balance: fixture.cashier_balance,
        min_upload_balance: fixture.min_upload_balance,
        target_upload_balance: fixture.target_upload_balance,
        project_cycles_reserve: fixture.project_cycles_reserve,
        project_cycles_available: fixture.project_cycles_available,
        gateway_principal_count: fixture.gateway_principal_count,
        last_gateway_principal_sync_at_ns: fixture.last_gateway_principal_sync_at_ns,
        gateway_principal_sync_action: fixture.gateway_principal_sync_action,
        funding_status: fixture.funding_status,
        ready: fixture.ready,
        blockers: fixture.blockers,
        warnings: fixture.warnings,
    }))
}

fn funding_report_response(requested_cycles: u128, attached_cycles: u128) -> String {
    response_json(&Ok::<_, CanicError>(BlobProjectCyclesTopUpReport {
        requested_cycles: Nat::from(requested_cycles),
        attached_cycles: Nat::from(attached_cycles),
        project_cycles_before: Nat::from(3_000_u64),
        project_cycles_after: Nat::from(2_100_u64),
        reserve_cycles: Nat::from(2_000_u64),
        cashier_total_after: Nat::from(1_900_u64),
        skipped_reason: None,
    }))
}

fn response_json<T: CandidType>(response: &T) -> String {
    let bytes = Encode!(response).expect("encode scripted response");
    serde_json::json!({
        "response_bytes": hex_bytes(bytes),
        "response_text": null,
        "response_candid": "scripted",
    })
    .to_string()
}
