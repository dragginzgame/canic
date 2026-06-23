//! Module: canic_cli::blob_storage::parse
//!
//! Responsibility: parse blob-storage canister-call responses into CLI views.
//! Does not own: runtime billing policy, Candid DTO definitions, or rendering.
//! Boundary: maps endpoint response fields into stable operator JSON codes.

use crate::blob_storage::model::{
    BLOB_STORAGE_JSON_SCHEMA_VERSION, BLOB_STORAGE_STATUS_KIND, BlobStorageCashierStatus,
    BlobStorageFundingStatus, BlobStorageGatewayStatus, BlobStorageNextAction,
    BlobStoragePolicyStatus, BlobStorageReadinessStatus, BlobStorageStatusResult,
    BlobStorageTarget,
};
use canic_host::response_parse::{find_field, parse_json_u64, parse_json_u128};

const PAYMENT_MODEL_NOT_CONFIGURED: &str = "not_configured";
const PAYMENT_MODEL_PROJECT_AS_PAYMENT_ACCOUNT: &str = "project_as_payment_account";
const FUNDING_NEEDED: &str = "funding_needed";
const GATEWAY_PRINCIPALS_EMPTY: &str = "gateway_principals_empty";
const CASHIER_BALANCE_BELOW_MIN: &str = "cashier_balance_below_min";
const CASHIER_BALANCE_UNAVAILABLE: &str = "cashier_balance_unavailable";
const CASHIER_RESPONSE_MALFORMED: &str = "cashier_response_malformed";
const PROJECT_CYCLES_RESERVE_BLOCKS_FUNDING: &str = "project_cycles_reserve_blocks_funding";

pub(super) fn parse_status_result(
    deployment: &str,
    target: BlobStorageTarget,
    output: &str,
) -> Option<BlobStorageStatusResult> {
    let value = serde_json::from_str::<serde_json::Value>(output).ok()?;
    let payment_model = find_field(&value, "payment_model").and_then(parse_variant_code)?;
    let configured = payment_model != PAYMENT_MODEL_NOT_CONFIGURED;
    let cashier_balance = find_field(&value, "cashier_balance").and_then(parse_optional_u128);
    let blockers = parse_code_array(find_field(&value, "blockers"), readiness_blocker_code)?;
    let warnings = parse_code_array(find_field(&value, "warnings"), billing_warning_code)?;
    let ready = find_field(&value, "ready")?.as_bool()?;
    let funding = parse_funding_status(find_field(&value, "funding_status")?)?;
    let readiness = readiness_status(ready, blockers, warnings);

    Some(BlobStorageStatusResult {
        schema_version: BLOB_STORAGE_JSON_SCHEMA_VERSION,
        kind: BLOB_STORAGE_STATUS_KIND.to_string(),
        deployment: deployment.to_string(),
        next: next_actions(deployment, &target, &readiness, &funding),
        target,
        configured,
        cashier: BlobStorageCashierStatus {
            canister_id: find_field(&value, "cashier_canister_id").and_then(parse_optional_text),
            payment_account: find_field(&value, "payment_account").and_then(parse_optional_text),
            balance_cycles: cashier_balance.map(|cycles| cycles.to_string()),
            balance_available: cashier_balance.is_some(),
        },
        policy: BlobStoragePolicyStatus {
            min_upload_balance_cycles: find_field(&value, "min_upload_balance")
                .and_then(parse_optional_u128)
                .map(|cycles| cycles.to_string()),
            target_upload_balance_cycles: find_field(&value, "target_upload_balance")
                .and_then(parse_optional_u128)
                .map(|cycles| cycles.to_string()),
            project_cycles_reserve_cycles: find_field(&value, "project_cycles_reserve")
                .and_then(parse_optional_u128)
                .map(|cycles| cycles.to_string()),
            project_cycles_available: find_field(&value, "project_cycles_available")
                .and_then(parse_u128_deep)?
                .to_string(),
        },
        gateways: BlobStorageGatewayStatus {
            principal_count: find_field(&value, "gateway_principal_count")
                .and_then(parse_json_u64)?,
            last_sync_at_ns: find_field(&value, "last_gateway_principal_sync_at_ns")
                .and_then(parse_optional_u64)
                .map(|timestamp| timestamp.to_string()),
            sync_action: find_field(&value, "gateway_principal_sync_action")
                .and_then(parse_variant_code)?,
        },
        funding,
        readiness,
    })
}

fn readiness_status(
    ready: bool,
    blockers: Vec<String>,
    warnings: Vec<String>,
) -> BlobStorageReadinessStatus {
    let state = if !ready || !blockers.is_empty() {
        "blocked"
    } else if warnings.is_empty() {
        "ready"
    } else {
        "warning"
    };
    BlobStorageReadinessStatus {
        state: state.to_string(),
        ready_for_upload: ready,
        blockers,
        warnings,
    }
}

fn parse_funding_status(value: &serde_json::Value) -> Option<BlobStorageFundingStatus> {
    let variant = parse_variant_name(value)?;
    let status = funding_status_code(&variant);
    let payload = variant_payload(value, &variant);
    Some(BlobStorageFundingStatus {
        status: status.to_string(),
        requested_cycles: payload
            .and_then(|payload| find_field(payload, "requested_cycles"))
            .and_then(parse_u128_deep)
            .map(|cycles| cycles.to_string()),
        transferable_cycles: payload
            .and_then(|payload| find_field(payload, "transferable_cycles"))
            .and_then(parse_u128_deep)
            .map(|cycles| cycles.to_string()),
    })
}

fn next_actions(
    deployment: &str,
    target: &BlobStorageTarget,
    readiness: &BlobStorageReadinessStatus,
    funding: &BlobStorageFundingStatus,
) -> Vec<BlobStorageNextAction> {
    let mut next = Vec::new();
    if readiness
        .blockers
        .iter()
        .any(|blocker| blocker == GATEWAY_PRINCIPALS_EMPTY)
    {
        next.push(BlobStorageNextAction {
            action: "sync_gateways".to_string(),
            reason: GATEWAY_PRINCIPALS_EMPTY.to_string(),
            command: Some(format!(
                "canic blob-storage sync-gateways {deployment} {}",
                target.input
            )),
        });
    }
    if funding.status == FUNDING_NEEDED
        && let Some(requested_cycles) = &funding.requested_cycles
    {
        next.push(BlobStorageNextAction {
            action: "fund".to_string(),
            reason: FUNDING_NEEDED.to_string(),
            command: Some(format!(
                "canic blob-storage fund {deployment} {} --cycles {requested_cycles} --dry-run",
                target.input
            )),
        });
    }
    next
}

fn parse_code_array(
    value: Option<&serde_json::Value>,
    code: fn(&str) -> &'static str,
) -> Option<Vec<String>> {
    let Some(value) = value else {
        return Some(Vec::new());
    };
    value
        .as_array()?
        .iter()
        .map(|value| parse_variant_name(value).map(|variant| code(&variant).to_string()))
        .collect()
}

fn parse_optional_text(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Array(values) => values.first().and_then(parse_optional_text),
        serde_json::Value::Object(map) => map.values().find_map(parse_optional_text),
        _ => None,
    }
}

fn parse_optional_u64(value: &serde_json::Value) -> Option<u64> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::Array(values) => values.first().and_then(parse_optional_u64),
        serde_json::Value::Object(map) => map.values().find_map(parse_optional_u64),
        _ => parse_json_u64(value),
    }
}

fn parse_optional_u128(value: &serde_json::Value) -> Option<u128> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::Array(values) => values.first().and_then(parse_optional_u128),
        serde_json::Value::Object(map) => map.values().find_map(parse_optional_u128),
        _ => parse_u128_deep(value),
    }
}

fn parse_u128_deep(value: &serde_json::Value) -> Option<u128> {
    parse_json_u128(value).or_else(|| match value {
        serde_json::Value::Array(values) => values.iter().find_map(parse_u128_deep),
        serde_json::Value::Object(map) => map.values().find_map(parse_u128_deep),
        _ => None,
    })
}

fn parse_variant_code(value: &serde_json::Value) -> Option<String> {
    parse_variant_name(value).map(|variant| common_variant_code(&variant).to_string())
}

fn parse_variant_name(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Object(map) => map.keys().next().cloned(),
        serde_json::Value::Array(values) => values.iter().find_map(parse_variant_name),
        _ => None,
    }
}

fn variant_payload<'a>(
    value: &'a serde_json::Value,
    variant: &str,
) -> Option<&'a serde_json::Value> {
    match value {
        serde_json::Value::Object(map) => map.get(variant),
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|value| variant_payload(value, variant)),
        _ => None,
    }
}

fn common_variant_code(variant: &str) -> &'static str {
    match variant {
        "NotConfigured" => PAYMENT_MODEL_NOT_CONFIGURED,
        "ProjectAsPaymentAccount" => PAYMENT_MODEL_PROJECT_AS_PAYMENT_ACCOUNT,
        "NotRequested" => "not_requested",
        "SkippedConfigMissing" => "skipped_config_missing",
        "SkippedReadOnlyStatus" => "skipped_read_only_status",
        _ => "unknown",
    }
}

fn funding_status_code(variant: &str) -> &'static str {
    match variant {
        "NotConfigured" => PAYMENT_MODEL_NOT_CONFIGURED,
        "NotNeeded" => "not_needed",
        "FundingRequired" => FUNDING_NEEDED,
        "BalanceUnavailable" => CASHIER_BALANCE_UNAVAILABLE,
        "BalanceMalformed" => CASHIER_RESPONSE_MALFORMED,
        "ReserveWouldBeViolated" => PROJECT_CYCLES_RESERVE_BLOCKS_FUNDING,
        _ => "unknown",
    }
}

fn readiness_blocker_code(variant: &str) -> &'static str {
    match variant {
        "NotConfigured" => PAYMENT_MODEL_NOT_CONFIGURED,
        "GatewayPrincipalsMissing" => GATEWAY_PRINCIPALS_EMPTY,
        "CashierBalanceUnavailable" => CASHIER_BALANCE_UNAVAILABLE,
        "CashierBalanceMalformed" => CASHIER_RESPONSE_MALFORMED,
        "InsufficientCashierBalance" => CASHIER_BALANCE_BELOW_MIN,
        "ReserveWouldBeViolated" => PROJECT_CYCLES_RESERVE_BLOCKS_FUNDING,
        _ => "unknown",
    }
}

fn billing_warning_code(variant: &str) -> &'static str {
    match variant {
        "GatewayPrincipalSetEmpty" => GATEWAY_PRINCIPALS_EMPTY,
        "CashierBalanceUnavailable" => CASHIER_BALANCE_UNAVAILABLE,
        "CashierBalanceMalformed" => CASHIER_RESPONSE_MALFORMED,
        "SyncRequestedButStatusIsReadOnly" => "status_sync_request_ignored",
        _ => "unknown",
    }
}
