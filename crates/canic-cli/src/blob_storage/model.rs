//! Module: canic_cli::blob_storage::model
//!
//! Responsibility: define render-ready blob-storage CLI output models.
//! Does not own: canister DTOs, Cashier protocol shapes, or readiness policy.
//! Boundary: serializes stable CLI output for operator automation.

use serde::Serialize;

pub(super) const BLOB_STORAGE_JSON_SCHEMA_VERSION: u16 = 1;
pub(super) const BLOB_STORAGE_STATUS_KIND: &str = "blob_storage_status";
pub(super) const BLOB_STORAGE_ERROR_KIND: &str = "blob_storage_error";

///
/// BlobStorageTarget
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BlobStorageTarget {
    pub(super) input: String,
    pub(super) role: Option<String>,
    pub(super) canister_id: String,
    pub(super) candid_source: Option<String>,
}

impl BlobStorageTarget {
    pub(super) fn resolved(
        input: &str,
        role: Option<String>,
        canister_id: &str,
        candid_source: &str,
    ) -> Self {
        Self {
            input: input.to_string(),
            role,
            canister_id: canister_id.to_string(),
            candid_source: Some(candid_source.to_string()),
        }
    }
}

///
/// BlobStorageErrorTarget
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BlobStorageErrorTarget {
    pub(super) input: String,
    pub(super) role: Option<String>,
    pub(super) canister_id: Option<String>,
    pub(super) candid_source: Option<String>,
}

impl BlobStorageErrorTarget {
    pub(super) fn unresolved(input: &str) -> Self {
        Self {
            input: input.to_string(),
            role: None,
            canister_id: None,
            candid_source: None,
        }
    }
}

///
/// BlobStorageErrorResult
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BlobStorageErrorResult {
    pub(super) schema_version: u16,
    pub(super) kind: String,
    pub(super) deployment: String,
    pub(super) target: BlobStorageErrorTarget,
    pub(super) error: BlobStorageErrorBody,
}

impl BlobStorageErrorResult {
    pub(super) fn new(
        deployment: &str,
        target: &str,
        code: &str,
        message: String,
        exit_code: u8,
    ) -> Self {
        Self {
            schema_version: BLOB_STORAGE_JSON_SCHEMA_VERSION,
            kind: BLOB_STORAGE_ERROR_KIND.to_string(),
            deployment: deployment.to_string(),
            target: BlobStorageErrorTarget::unresolved(target),
            error: BlobStorageErrorBody {
                code: code.to_string(),
                message,
                exit_code,
            },
        }
    }
}

///
/// BlobStorageErrorBody
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BlobStorageErrorBody {
    pub(super) code: String,
    pub(super) message: String,
    pub(super) exit_code: u8,
}

///
/// BlobStorageActionName
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BlobStorageActionName {
    SyncGateways,
    Fund,
}

impl BlobStorageActionName {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::SyncGateways => "sync_gateways",
            Self::Fund => "fund",
        }
    }

    pub(super) const fn kind(self) -> &'static str {
        match self {
            Self::SyncGateways => "blob_storage_sync_gateways_result",
            Self::Fund => "blob_storage_fund_result",
        }
    }
}

///
/// BlobStorageAction
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BlobStorageAction {
    pub(super) name: String,
    pub(super) method: String,
    pub(super) mode: String,
    pub(super) dry_run: bool,
    pub(super) success: bool,
    pub(super) command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) requested_cycles: Option<String>,
}

///
/// BlobStorageActionResult
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BlobStorageActionResult {
    pub(super) schema_version: u16,
    pub(super) kind: String,
    pub(super) deployment: String,
    pub(super) target: BlobStorageTarget,
    pub(super) action: BlobStorageAction,
    pub(super) funding_report: Option<BlobStorageFundingReport>,
    pub(super) post_status: Option<BlobStorageStatusResult>,
    pub(super) warnings: Vec<String>,
}

impl BlobStorageActionResult {
    pub(super) fn dry_run(
        deployment: &str,
        action_name: BlobStorageActionName,
        target: BlobStorageTarget,
        method: &str,
        mode: &str,
        command: String,
        requested_cycles: Option<u128>,
    ) -> Self {
        Self::new(
            deployment,
            action_name,
            target,
            (method, mode),
            true,
            command,
            requested_cycles,
        )
    }

    pub(super) fn completed(
        deployment: &str,
        action_name: BlobStorageActionName,
        target: BlobStorageTarget,
        method: &str,
        mode: &str,
        command: String,
        requested_cycles: Option<u128>,
    ) -> Self {
        Self::new(
            deployment,
            action_name,
            target,
            (method, mode),
            false,
            command,
            requested_cycles,
        )
    }

    fn new(
        deployment: &str,
        action_name: BlobStorageActionName,
        target: BlobStorageTarget,
        method_mode: (&str, &str),
        dry_run: bool,
        command: String,
        requested_cycles: Option<u128>,
    ) -> Self {
        let (method, mode) = method_mode;
        Self {
            schema_version: BLOB_STORAGE_JSON_SCHEMA_VERSION,
            kind: action_name.kind().to_string(),
            deployment: deployment.to_string(),
            target,
            action: BlobStorageAction {
                name: action_name.label().to_string(),
                method: method.to_string(),
                mode: mode.to_string(),
                dry_run,
                success: true,
                command,
                requested_cycles: requested_cycles.map(|value| value.to_string()),
            },
            funding_report: None,
            post_status: None,
            warnings: Vec::new(),
        }
    }

    pub(super) fn with_funding_report(mut self, report: BlobStorageFundingReport) -> Self {
        self.funding_report = Some(report);
        self
    }

    pub(super) fn with_post_status(mut self, status: BlobStorageStatusResult) -> Self {
        self.post_status = Some(status);
        self
    }

    pub(super) fn with_warning(mut self, warning: &str) -> Self {
        self.warnings.push(warning.to_string());
        self
    }
}

///
/// BlobStorageFundingReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BlobStorageFundingReport {
    pub(super) requested_cycles: String,
    pub(super) attached_cycles: String,
    pub(super) project_cycles_before: String,
    pub(super) project_cycles_after: String,
    pub(super) reserve_cycles: String,
    pub(super) cashier_total_after: String,
    pub(super) skipped_reason: Option<String>,
}

///
/// BlobStorageStatusResult
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BlobStorageStatusResult {
    pub(super) schema_version: u16,
    pub(super) kind: String,
    pub(super) deployment: String,
    pub(super) target: BlobStorageTarget,
    pub(super) configured: bool,
    pub(super) cashier: BlobStorageCashierStatus,
    pub(super) policy: BlobStoragePolicyStatus,
    pub(super) gateways: BlobStorageGatewayStatus,
    pub(super) funding: BlobStorageFundingStatus,
    pub(super) readiness: BlobStorageReadinessStatus,
    pub(super) next: Vec<BlobStorageNextAction>,
}

///
/// BlobStorageCashierStatus
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BlobStorageCashierStatus {
    pub(super) canister_id: Option<String>,
    pub(super) payment_account: Option<String>,
    pub(super) balance_cycles: Option<String>,
    pub(super) balance_available: bool,
}

///
/// BlobStoragePolicyStatus
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BlobStoragePolicyStatus {
    pub(super) min_upload_balance_cycles: Option<String>,
    pub(super) target_upload_balance_cycles: Option<String>,
    pub(super) project_cycles_reserve_cycles: Option<String>,
    pub(super) project_cycles_available: String,
}

///
/// BlobStorageGatewayStatus
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BlobStorageGatewayStatus {
    pub(super) principal_count: u64,
    pub(super) last_sync_at_ns: Option<String>,
    pub(super) sync_action: String,
}

///
/// BlobStorageFundingStatus
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BlobStorageFundingStatus {
    pub(super) status: String,
    pub(super) requested_cycles: Option<String>,
    pub(super) transferable_cycles: Option<String>,
}

///
/// BlobStorageReadinessStatus
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BlobStorageReadinessStatus {
    pub(super) state: String,
    pub(super) ready_for_upload: bool,
    pub(super) blockers: Vec<String>,
    pub(super) warnings: Vec<String>,
}

///
/// BlobStorageNextAction
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BlobStorageNextAction {
    pub(super) action: String,
    pub(super) reason: String,
    pub(super) command: Option<String>,
}
