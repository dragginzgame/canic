//! Module: canic_cli::blob_storage::parse
//!
//! Responsibility: decode typed blob-storage responses into CLI views.
//! Does not own: runtime billing policy, Candid DTO definitions, or rendering.
//! Boundary: accepts the canonical ICP envelope and maps DTO enums to stable JSON codes.

use crate::blob_storage::model::{
    BLOB_STORAGE_CODE_CASHIER_BALANCE_BELOW_MIN, BLOB_STORAGE_CODE_CASHIER_BALANCE_UNAVAILABLE,
    BLOB_STORAGE_CODE_CASHIER_RESPONSE_MALFORMED, BLOB_STORAGE_CODE_FUNDING_NEEDED,
    BLOB_STORAGE_CODE_GATEWAY_PRINCIPALS_EMPTY, BLOB_STORAGE_CODE_NOT_CONFIGURED,
    BLOB_STORAGE_CODE_NOT_REQUESTED, BLOB_STORAGE_CODE_PROJECT_CYCLES_RESERVE_BLOCKS_FUNDING,
    BLOB_STORAGE_CODE_SKIPPED_CONFIG_MISSING, BLOB_STORAGE_CODE_SKIPPED_READ_ONLY_STATUS,
    BLOB_STORAGE_CODE_STATUS_SYNC_REQUEST_IGNORED, BLOB_STORAGE_JSON_SCHEMA_VERSION,
    BlobStorageActionName, BlobStorageCashierStatus, BlobStorageFundingReport,
    BlobStorageFundingStatus, BlobStorageFundingStatusCode, BlobStorageGatewayStatus,
    BlobStorageNextAction, BlobStoragePolicyStatus, BlobStorageReadinessState,
    BlobStorageReadinessStatus, BlobStorageReportKind, BlobStorageStatusResult, BlobStorageTarget,
};
use candid::Nat;
use canic_core::dto::blob_storage::{
    BlobProjectCyclesTopUpReport, BlobStorageBillingWarning,
    BlobStorageFundingStatus as BlobStorageFundingStatusDto, BlobStorageGatewayPrincipalSyncAction,
    BlobStoragePaymentModelStatus, BlobStorageReadinessBlocker, BlobStorageStatusResponse,
};
use canic_host::icp::{IcpJsonResponseError, decode_json_result_response};
use thiserror::Error as ThisError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BlobStorageResponseKind {
    Funding,
    Status,
}

impl BlobStorageResponseKind {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Funding => "funding",
            Self::Status => "status",
        }
    }
}

#[derive(Debug, ThisError)]
pub(super) enum BlobStorageParseError {
    #[error("{} response field `{field}` exceeds u128", kind.label())]
    NatOutOfRange {
        kind: BlobStorageResponseKind,
        field: &'static str,
    },

    #[error(transparent)]
    Response(#[from] IcpJsonResponseError),
}

pub(super) fn parse_status_result(
    deployment: &str,
    target: BlobStorageTarget,
    output: &str,
) -> Result<BlobStorageStatusResult, BlobStorageParseError> {
    let response = decode_json_result_response::<BlobStorageStatusResponse>(output)?;
    status_result(deployment, target, response)
}

fn status_result(
    deployment: &str,
    target: BlobStorageTarget,
    response: BlobStorageStatusResponse,
) -> Result<BlobStorageStatusResult, BlobStorageParseError> {
    let kind = BlobStorageResponseKind::Status;
    let configured = !matches!(
        response.payment_model,
        BlobStoragePaymentModelStatus::NotConfigured
    );
    let cashier_balance = optional_nat_string(response.cashier_balance, kind, "cashier_balance")?;
    let blockers = response
        .blockers
        .into_iter()
        .map(readiness_blocker_code)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let warnings = response
        .warnings
        .into_iter()
        .map(billing_warning_code)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let funding = funding_status(response.funding_status)?;
    let readiness = readiness_status(response.ready, blockers, warnings);

    Ok(BlobStorageStatusResult {
        schema_version: BLOB_STORAGE_JSON_SCHEMA_VERSION,
        kind: BlobStorageReportKind::Status,
        deployment: deployment.to_string(),
        next: next_actions(deployment, &target, &readiness, &funding),
        target,
        configured,
        cashier: BlobStorageCashierStatus {
            canister_id: response.cashier_canister_id.map(|pid| pid.to_text()),
            payment_account: response.payment_account.map(|pid| pid.to_text()),
            balance_available: cashier_balance.is_some(),
            balance_cycles: cashier_balance,
        },
        policy: BlobStoragePolicyStatus {
            min_upload_balance_cycles: optional_nat_string(
                response.min_upload_balance,
                kind,
                "min_upload_balance",
            )?,
            target_upload_balance_cycles: optional_nat_string(
                response.target_upload_balance,
                kind,
                "target_upload_balance",
            )?,
            project_cycles_reserve_cycles: optional_nat_string(
                response.project_cycles_reserve,
                kind,
                "project_cycles_reserve",
            )?,
            project_cycles_available: nat_string(
                response.project_cycles_available,
                kind,
                "project_cycles_available",
            )?,
        },
        gateways: BlobStorageGatewayStatus {
            principal_count: response.gateway_principal_count,
            last_sync_at_ns: response
                .last_gateway_principal_sync_at_ns
                .map(|timestamp| timestamp.to_string()),
            sync_action: gateway_sync_action_code(response.gateway_principal_sync_action)
                .to_string(),
        },
        funding,
        readiness,
    })
}

pub(super) fn parse_funding_report(
    output: &str,
) -> Result<BlobStorageFundingReport, BlobStorageParseError> {
    let response = decode_json_result_response::<BlobProjectCyclesTopUpReport>(output)?;
    let kind = BlobStorageResponseKind::Funding;
    Ok(BlobStorageFundingReport {
        requested_cycles: nat_string(response.requested_cycles, kind, "requested_cycles")?,
        attached_cycles: nat_string(response.attached_cycles, kind, "attached_cycles")?,
        project_cycles_before: nat_string(
            response.project_cycles_before,
            kind,
            "project_cycles_before",
        )?,
        project_cycles_after: nat_string(
            response.project_cycles_after,
            kind,
            "project_cycles_after",
        )?,
        reserve_cycles: nat_string(response.reserve_cycles, kind, "reserve_cycles")?,
        cashier_total_after: nat_string(response.cashier_total_after, kind, "cashier_total_after")?,
        skipped_reason: response.skipped_reason,
    })
}

fn funding_status(
    status: BlobStorageFundingStatusDto,
) -> Result<BlobStorageFundingStatus, BlobStorageParseError> {
    let kind = BlobStorageResponseKind::Status;
    let (status, requested_cycles, transferable_cycles) = match status {
        BlobStorageFundingStatusDto::NotConfigured => {
            (BlobStorageFundingStatusCode::NotConfigured, None, None)
        }
        BlobStorageFundingStatusDto::NotNeeded => {
            (BlobStorageFundingStatusCode::NotNeeded, None, None)
        }
        BlobStorageFundingStatusDto::FundingRequired { requested_cycles } => (
            BlobStorageFundingStatusCode::FundingNeeded,
            Some(nat_string(
                requested_cycles,
                kind,
                "funding_status.requested_cycles",
            )?),
            None,
        ),
        BlobStorageFundingStatusDto::BalanceUnavailable => (
            BlobStorageFundingStatusCode::CashierBalanceUnavailable,
            None,
            None,
        ),
        BlobStorageFundingStatusDto::BalanceMalformed => (
            BlobStorageFundingStatusCode::CashierResponseMalformed,
            None,
            None,
        ),
        BlobStorageFundingStatusDto::ReserveWouldBeViolated {
            requested_cycles,
            transferable_cycles,
        } => (
            BlobStorageFundingStatusCode::ProjectCyclesReserveBlocksFunding,
            Some(nat_string(
                requested_cycles,
                kind,
                "funding_status.requested_cycles",
            )?),
            Some(nat_string(
                transferable_cycles,
                kind,
                "funding_status.transferable_cycles",
            )?),
        ),
    };
    Ok(BlobStorageFundingStatus {
        status,
        requested_cycles,
        transferable_cycles,
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

fn optional_nat_string(
    value: Option<Nat>,
    kind: BlobStorageResponseKind,
    field: &'static str,
) -> Result<Option<String>, BlobStorageParseError> {
    value
        .map(|value| nat_string(value, kind, field))
        .transpose()
}

fn nat_string(
    value: Nat,
    kind: BlobStorageResponseKind,
    field: &'static str,
) -> Result<String, BlobStorageParseError> {
    u128::try_from(value.0)
        .map(|value| value.to_string())
        .map_err(|_| BlobStorageParseError::NatOutOfRange { kind, field })
}

const fn gateway_sync_action_code(action: BlobStorageGatewayPrincipalSyncAction) -> &'static str {
    match action {
        BlobStorageGatewayPrincipalSyncAction::NotRequested => BLOB_STORAGE_CODE_NOT_REQUESTED,
        BlobStorageGatewayPrincipalSyncAction::SkippedConfigMissing => {
            BLOB_STORAGE_CODE_SKIPPED_CONFIG_MISSING
        }
        BlobStorageGatewayPrincipalSyncAction::SkippedReadOnlyStatus => {
            BLOB_STORAGE_CODE_SKIPPED_READ_ONLY_STATUS
        }
    }
}

const fn readiness_blocker_code(blocker: BlobStorageReadinessBlocker) -> &'static str {
    match blocker {
        BlobStorageReadinessBlocker::NotConfigured => BLOB_STORAGE_CODE_NOT_CONFIGURED,
        BlobStorageReadinessBlocker::GatewayPrincipalsMissing => {
            BLOB_STORAGE_CODE_GATEWAY_PRINCIPALS_EMPTY
        }
        BlobStorageReadinessBlocker::CashierBalanceUnavailable => {
            BLOB_STORAGE_CODE_CASHIER_BALANCE_UNAVAILABLE
        }
        BlobStorageReadinessBlocker::CashierBalanceMalformed => {
            BLOB_STORAGE_CODE_CASHIER_RESPONSE_MALFORMED
        }
        BlobStorageReadinessBlocker::InsufficientCashierBalance => {
            BLOB_STORAGE_CODE_CASHIER_BALANCE_BELOW_MIN
        }
        BlobStorageReadinessBlocker::ReserveWouldBeViolated => {
            BLOB_STORAGE_CODE_PROJECT_CYCLES_RESERVE_BLOCKS_FUNDING
        }
    }
}

const fn billing_warning_code(warning: BlobStorageBillingWarning) -> &'static str {
    match warning {
        BlobStorageBillingWarning::GatewayPrincipalSetEmpty => {
            BLOB_STORAGE_CODE_GATEWAY_PRINCIPALS_EMPTY
        }
        BlobStorageBillingWarning::CashierBalanceUnavailable => {
            BLOB_STORAGE_CODE_CASHIER_BALANCE_UNAVAILABLE
        }
        BlobStorageBillingWarning::CashierBalanceMalformed => {
            BLOB_STORAGE_CODE_CASHIER_RESPONSE_MALFORMED
        }
        BlobStorageBillingWarning::SyncRequestedButStatusIsReadOnly => {
            BLOB_STORAGE_CODE_STATUS_SYNC_REQUEST_IGNORED
        }
    }
}
