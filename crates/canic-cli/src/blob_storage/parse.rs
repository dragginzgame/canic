//! Module: canic_cli::blob_storage::parse
//!
//! Responsibility: parse blob-storage canister-call responses into CLI views.
//! Does not own: runtime billing policy, Candid DTO definitions, or rendering.
//! Boundary: maps endpoint response fields into stable operator JSON codes.

use crate::blob_storage::model::{
    BLOB_STORAGE_CODE_CASHIER_BALANCE_BELOW_MIN, BLOB_STORAGE_CODE_CASHIER_BALANCE_UNAVAILABLE,
    BLOB_STORAGE_CODE_CASHIER_RESPONSE_MALFORMED, BLOB_STORAGE_CODE_FUNDING_NEEDED,
    BLOB_STORAGE_CODE_GATEWAY_PRINCIPALS_EMPTY, BLOB_STORAGE_CODE_NOT_CONFIGURED,
    BLOB_STORAGE_CODE_NOT_REQUESTED, BLOB_STORAGE_CODE_PROJECT_AS_PAYMENT_ACCOUNT,
    BLOB_STORAGE_CODE_PROJECT_CYCLES_RESERVE_BLOCKS_FUNDING,
    BLOB_STORAGE_CODE_SKIPPED_CONFIG_MISSING, BLOB_STORAGE_CODE_SKIPPED_READ_ONLY_STATUS,
    BLOB_STORAGE_CODE_STATUS_SYNC_REQUEST_IGNORED, BLOB_STORAGE_CODE_UNKNOWN,
    BLOB_STORAGE_JSON_SCHEMA_VERSION, BlobStorageActionName, BlobStorageCashierStatus,
    BlobStorageFundingReport, BlobStorageFundingStatus, BlobStorageFundingStatusCode,
    BlobStorageGatewayStatus, BlobStorageNextAction, BlobStoragePolicyStatus,
    BlobStorageReadinessState, BlobStorageReadinessStatus, BlobStorageReportKind,
    BlobStorageStatusResult, BlobStorageTarget,
};
use canic_host::response_parse::{find_field, parse_json_u64, parse_json_u128};
use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BlobStorageResponseKind {
    Status,
    Funding,
}

impl BlobStorageResponseKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::Funding => "funding",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum BlobStorageParseError {
    InvalidJson {
        kind: BlobStorageResponseKind,
        error: String,
    },
    MissingField {
        kind: BlobStorageResponseKind,
        field: &'static str,
    },
    InvalidField {
        kind: BlobStorageResponseKind,
        field: &'static str,
    },
}

impl fmt::Display for BlobStorageParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson { kind, error } => {
                write!(
                    formatter,
                    "{} response has invalid JSON: {error}",
                    kind.label()
                )
            }
            Self::MissingField { kind, field } => {
                write!(formatter, "{} response is missing `{field}`", kind.label())
            }
            Self::InvalidField { kind, field } => {
                write!(formatter, "{} response has invalid `{field}`", kind.label())
            }
        }
    }
}

impl Error for BlobStorageParseError {}

pub(super) fn parse_status_result(
    deployment: &str,
    target: BlobStorageTarget,
    output: &str,
) -> Result<BlobStorageStatusResult, BlobStorageParseError> {
    let kind = BlobStorageResponseKind::Status;
    let value = parse_json_response(output, kind)?;
    let payment_model = parse_required_field(&value, kind, "payment_model", parse_variant_code)?;
    let configured = payment_model != BLOB_STORAGE_CODE_NOT_CONFIGURED;
    let cashier_balance = parse_optional_field(
        find_field(&value, "cashier_balance"),
        kind,
        "cashier_balance",
        parse_optional_u128,
    )?;
    let blockers = parse_code_array(
        find_field(&value, "blockers"),
        kind,
        "blockers",
        readiness_blocker_code,
    )?;
    let warnings = parse_code_array(
        find_field(&value, "warnings"),
        kind,
        "warnings",
        billing_warning_code,
    )?;
    let ready = parse_required_field(&value, kind, "ready", serde_json::Value::as_bool)?;
    let funding_value = required_field(&value, kind, "funding_status")?;
    let funding = parse_funding_status(funding_value)?;
    let readiness = readiness_status(ready, blockers, warnings);

    Ok(BlobStorageStatusResult {
        schema_version: BLOB_STORAGE_JSON_SCHEMA_VERSION,
        kind: BlobStorageReportKind::Status,
        deployment: deployment.to_string(),
        next: next_actions(deployment, &target, &readiness, &funding),
        target,
        configured,
        cashier: parse_cashier_status(&value, cashier_balance)?,
        policy: parse_policy_status(&value)?,
        gateways: parse_gateway_status(&value)?,
        funding,
        readiness,
    })
}

fn parse_cashier_status(
    value: &serde_json::Value,
    balance: Option<u128>,
) -> Result<BlobStorageCashierStatus, BlobStorageParseError> {
    let kind = BlobStorageResponseKind::Status;
    Ok(BlobStorageCashierStatus {
        canister_id: parse_optional_field(
            find_field(value, "cashier_canister_id"),
            kind,
            "cashier_canister_id",
            parse_optional_text,
        )?,
        payment_account: parse_optional_field(
            find_field(value, "payment_account"),
            kind,
            "payment_account",
            parse_optional_text,
        )?,
        balance_cycles: balance.map(|cycles| cycles.to_string()),
        balance_available: balance.is_some(),
    })
}

fn parse_policy_status(
    value: &serde_json::Value,
) -> Result<BlobStoragePolicyStatus, BlobStorageParseError> {
    let kind = BlobStorageResponseKind::Status;
    Ok(BlobStoragePolicyStatus {
        min_upload_balance_cycles: parse_optional_field(
            find_field(value, "min_upload_balance"),
            kind,
            "min_upload_balance",
            parse_optional_u128,
        )?
        .map(|cycles| cycles.to_string()),
        target_upload_balance_cycles: parse_optional_field(
            find_field(value, "target_upload_balance"),
            kind,
            "target_upload_balance",
            parse_optional_u128,
        )?
        .map(|cycles| cycles.to_string()),
        project_cycles_reserve_cycles: parse_optional_field(
            find_field(value, "project_cycles_reserve"),
            kind,
            "project_cycles_reserve",
            parse_optional_u128,
        )?
        .map(|cycles| cycles.to_string()),
        project_cycles_available: parse_required_field(
            value,
            kind,
            "project_cycles_available",
            parse_u128_deep,
        )?
        .to_string(),
    })
}

fn parse_gateway_status(
    value: &serde_json::Value,
) -> Result<BlobStorageGatewayStatus, BlobStorageParseError> {
    let kind = BlobStorageResponseKind::Status;
    Ok(BlobStorageGatewayStatus {
        principal_count: parse_required_field(
            value,
            kind,
            "gateway_principal_count",
            parse_json_u64,
        )?,
        last_sync_at_ns: parse_optional_field(
            find_field(value, "last_gateway_principal_sync_at_ns"),
            kind,
            "last_gateway_principal_sync_at_ns",
            parse_optional_u64,
        )?
        .map(|timestamp| timestamp.to_string()),
        sync_action: parse_required_field(
            value,
            kind,
            "gateway_principal_sync_action",
            parse_variant_code,
        )?,
    })
}

pub(super) fn parse_funding_report(
    output: &str,
) -> Result<BlobStorageFundingReport, BlobStorageParseError> {
    let kind = BlobStorageResponseKind::Funding;
    let value = parse_json_response(output, kind)?;
    Ok(BlobStorageFundingReport {
        requested_cycles: required_cycles(&value, kind, "requested_cycles")?,
        attached_cycles: required_cycles(&value, kind, "attached_cycles")?,
        project_cycles_before: required_cycles(&value, kind, "project_cycles_before")?,
        project_cycles_after: required_cycles(&value, kind, "project_cycles_after")?,
        reserve_cycles: required_cycles(&value, kind, "reserve_cycles")?,
        cashier_total_after: required_cycles(&value, kind, "cashier_total_after")?,
        skipped_reason: parse_optional_field(
            find_field(&value, "skipped_reason"),
            kind,
            "skipped_reason",
            parse_optional_text,
        )?,
    })
}

const fn readiness_status(
    ready: bool,
    blockers: Vec<String>,
    warnings: Vec<String>,
) -> BlobStorageReadinessStatus {
    let state = if !ready || !blockers.is_empty() {
        BlobStorageReadinessState::Blocked
    } else if warnings.is_empty() {
        BlobStorageReadinessState::Ready
    } else {
        BlobStorageReadinessState::Warning
    };
    BlobStorageReadinessStatus {
        state,
        ready_for_upload: ready,
        blockers,
        warnings,
    }
}

fn parse_funding_status(
    value: &serde_json::Value,
) -> Result<BlobStorageFundingStatus, BlobStorageParseError> {
    let kind = BlobStorageResponseKind::Status;
    let variant = parse_variant_name(value).ok_or(BlobStorageParseError::InvalidField {
        kind,
        field: "funding_status",
    })?;
    let status = funding_status_code(&variant);
    let payload = variant_payload(value, &variant);
    Ok(BlobStorageFundingStatus {
        status,
        requested_cycles: parse_optional_field(
            payload.and_then(|payload| find_field(payload, "requested_cycles")),
            kind,
            "funding_status.requested_cycles",
            parse_u128_deep,
        )?
        .map(|cycles| cycles.to_string()),
        transferable_cycles: parse_optional_field(
            payload.and_then(|payload| find_field(payload, "transferable_cycles")),
            kind,
            "funding_status.transferable_cycles",
            parse_u128_deep,
        )?
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
        .any(|blocker| blocker == BLOB_STORAGE_CODE_GATEWAY_PRINCIPALS_EMPTY)
    {
        next.push(BlobStorageNextAction {
            action: BlobStorageActionName::SyncGateways.label().to_string(),
            reason: BLOB_STORAGE_CODE_GATEWAY_PRINCIPALS_EMPTY.to_string(),
            command: Some(format!(
                "canic blob-storage sync-gateways {deployment} {}",
                target.input
            )),
        });
    }
    if funding.status == BlobStorageFundingStatusCode::FundingNeeded
        && let Some(requested_cycles) = &funding.requested_cycles
    {
        next.push(BlobStorageNextAction {
            action: BlobStorageActionName::Fund.label().to_string(),
            reason: BLOB_STORAGE_CODE_FUNDING_NEEDED.to_string(),
            command: Some(format!(
                "canic blob-storage fund {deployment} {} --cycles {requested_cycles}",
                target.input
            )),
        });
    }
    next
}

fn parse_json_response(
    output: &str,
    kind: BlobStorageResponseKind,
) -> Result<serde_json::Value, BlobStorageParseError> {
    serde_json::from_str(output).map_err(|error| BlobStorageParseError::InvalidJson {
        kind,
        error: error.to_string(),
    })
}

fn required_field<'a>(
    value: &'a serde_json::Value,
    kind: BlobStorageResponseKind,
    field: &'static str,
) -> Result<&'a serde_json::Value, BlobStorageParseError> {
    find_field(value, field).ok_or(BlobStorageParseError::MissingField { kind, field })
}

fn parse_required_field<T>(
    value: &serde_json::Value,
    kind: BlobStorageResponseKind,
    field: &'static str,
    parse: impl FnOnce(&serde_json::Value) -> Option<T>,
) -> Result<T, BlobStorageParseError> {
    let value = required_field(value, kind, field)?;
    parse(value).ok_or(BlobStorageParseError::InvalidField { kind, field })
}

fn parse_optional_field<T>(
    value: Option<&serde_json::Value>,
    kind: BlobStorageResponseKind,
    field: &'static str,
    parse: impl FnOnce(&serde_json::Value) -> Option<T>,
) -> Result<Option<T>, BlobStorageParseError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || matches!(value, serde_json::Value::Array(values) if values.is_empty()) {
        return Ok(None);
    }
    parse(value)
        .map(Some)
        .ok_or(BlobStorageParseError::InvalidField { kind, field })
}

fn required_cycles(
    value: &serde_json::Value,
    kind: BlobStorageResponseKind,
    field: &'static str,
) -> Result<String, BlobStorageParseError> {
    parse_required_field(value, kind, field, parse_u128_deep).map(|cycles| cycles.to_string())
}

fn parse_code_array(
    value: Option<&serde_json::Value>,
    kind: BlobStorageResponseKind,
    field: &'static str,
    code: fn(&str) -> &'static str,
) -> Result<Vec<String>, BlobStorageParseError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    value
        .as_array()
        .ok_or(BlobStorageParseError::InvalidField { kind, field })?
        .iter()
        .map(|value| parse_variant_name(value).map(|variant| code(&variant).to_string()))
        .collect::<Option<Vec<_>>>()
        .ok_or(BlobStorageParseError::InvalidField { kind, field })
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
        "NotConfigured" => BLOB_STORAGE_CODE_NOT_CONFIGURED,
        "ProjectAsPaymentAccount" => BLOB_STORAGE_CODE_PROJECT_AS_PAYMENT_ACCOUNT,
        "NotRequested" => BLOB_STORAGE_CODE_NOT_REQUESTED,
        "SkippedConfigMissing" => BLOB_STORAGE_CODE_SKIPPED_CONFIG_MISSING,
        "SkippedReadOnlyStatus" => BLOB_STORAGE_CODE_SKIPPED_READ_ONLY_STATUS,
        _ => BLOB_STORAGE_CODE_UNKNOWN,
    }
}

fn funding_status_code(variant: &str) -> BlobStorageFundingStatusCode {
    match variant {
        "NotConfigured" => BlobStorageFundingStatusCode::NotConfigured,
        "NotNeeded" => BlobStorageFundingStatusCode::NotNeeded,
        "FundingRequired" => BlobStorageFundingStatusCode::FundingNeeded,
        "BalanceUnavailable" => BlobStorageFundingStatusCode::CashierBalanceUnavailable,
        "BalanceMalformed" => BlobStorageFundingStatusCode::CashierResponseMalformed,
        "ReserveWouldBeViolated" => BlobStorageFundingStatusCode::ProjectCyclesReserveBlocksFunding,
        _ => BlobStorageFundingStatusCode::Unknown,
    }
}

fn readiness_blocker_code(variant: &str) -> &'static str {
    match variant {
        "NotConfigured" => BLOB_STORAGE_CODE_NOT_CONFIGURED,
        "GatewayPrincipalsMissing" => BLOB_STORAGE_CODE_GATEWAY_PRINCIPALS_EMPTY,
        "CashierBalanceUnavailable" => BLOB_STORAGE_CODE_CASHIER_BALANCE_UNAVAILABLE,
        "CashierBalanceMalformed" => BLOB_STORAGE_CODE_CASHIER_RESPONSE_MALFORMED,
        "InsufficientCashierBalance" => BLOB_STORAGE_CODE_CASHIER_BALANCE_BELOW_MIN,
        "ReserveWouldBeViolated" => BLOB_STORAGE_CODE_PROJECT_CYCLES_RESERVE_BLOCKS_FUNDING,
        _ => BLOB_STORAGE_CODE_UNKNOWN,
    }
}

fn billing_warning_code(variant: &str) -> &'static str {
    match variant {
        "GatewayPrincipalSetEmpty" => BLOB_STORAGE_CODE_GATEWAY_PRINCIPALS_EMPTY,
        "CashierBalanceUnavailable" => BLOB_STORAGE_CODE_CASHIER_BALANCE_UNAVAILABLE,
        "CashierBalanceMalformed" => BLOB_STORAGE_CODE_CASHIER_RESPONSE_MALFORMED,
        "SyncRequestedButStatusIsReadOnly" => BLOB_STORAGE_CODE_STATUS_SYNC_REQUEST_IGNORED,
        _ => BLOB_STORAGE_CODE_UNKNOWN,
    }
}
