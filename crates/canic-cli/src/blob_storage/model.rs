//! Module: canic_cli::blob_storage::model
//!
//! Responsibility: define render-ready blob-storage CLI output models.
//! Does not own: canister DTOs, Cashier protocol shapes, or readiness policy.
//! Boundary: serializes stable CLI output for operator automation.

use serde::{Serialize, Serializer};

pub(super) const BLOB_STORAGE_JSON_SCHEMA_VERSION: u16 = 1;
pub(super) const BLOB_STORAGE_STATUS_KIND: &str = "blob_storage_status";
pub(super) const BLOB_STORAGE_ERROR_KIND: &str = "blob_storage_error";
pub(super) const BLOB_STORAGE_CANDID_SOURCE_INSTALLED_DEPLOYMENT: &str = "installed_deployment";
pub(super) const BLOB_STORAGE_READINESS_BLOCKED: &str = "blocked";
pub(super) const BLOB_STORAGE_READINESS_READY: &str = "ready";
pub(super) const BLOB_STORAGE_READINESS_WARNING: &str = "warning";
pub(super) const BLOB_STORAGE_CODE_NOT_CONFIGURED: &str = "not_configured";
pub(super) const BLOB_STORAGE_CODE_FUNDING_NEEDED: &str = "funding_needed";
pub(super) const BLOB_STORAGE_CODE_GATEWAY_PRINCIPALS_EMPTY: &str = "gateway_principals_empty";
pub(super) const BLOB_STORAGE_CODE_CASHIER_BALANCE_BELOW_MIN: &str = "cashier_balance_below_min";
pub(super) const BLOB_STORAGE_CODE_CASHIER_BALANCE_UNAVAILABLE: &str =
    "cashier_balance_unavailable";
pub(super) const BLOB_STORAGE_CODE_CASHIER_RESPONSE_MALFORMED: &str = "cashier_response_malformed";
pub(super) const BLOB_STORAGE_CODE_PROJECT_CYCLES_RESERVE_BLOCKS_FUNDING: &str =
    "project_cycles_reserve_blocks_funding";
pub(super) const BLOB_STORAGE_CODE_NOT_NEEDED: &str = "not_needed";
pub(super) const BLOB_STORAGE_CODE_NOT_REQUESTED: &str = "not_requested";
pub(super) const BLOB_STORAGE_CODE_SKIPPED_CONFIG_MISSING: &str = "skipped_config_missing";
pub(super) const BLOB_STORAGE_CODE_SKIPPED_READ_ONLY_STATUS: &str = "skipped_read_only_status";
pub(super) const BLOB_STORAGE_CODE_STATUS_SYNC_REQUEST_IGNORED: &str =
    "status_sync_request_ignored";
pub(super) const BLOB_STORAGE_WARNING_POST_STATUS_UNAVAILABLE: &str = "post_status_unavailable";
pub(super) const BLOB_STORAGE_ERROR_CODE_INVALID_CYCLES: &str = "invalid_cycles";
pub(super) const BLOB_STORAGE_ERROR_CODE_TARGET_RESOLUTION_FAILED: &str =
    "target_resolution_failed";
pub(super) const BLOB_STORAGE_ERROR_CODE_CANDID_UNAVAILABLE: &str = "candid_unavailable";
pub(super) const BLOB_STORAGE_ERROR_CODE_METHOD_UNAVAILABLE: &str = "method_unavailable";
pub(super) const BLOB_STORAGE_ERROR_CODE_TRANSPORT_FAILED: &str = "transport_failed";
pub(super) const BLOB_STORAGE_ERROR_CODE_RESPONSE_PARSE_FAILED: &str = "response_parse_failed";
pub(super) const BLOB_STORAGE_ERROR_CODE_CANDID_DECODE_FAILED: &str = "candid_decode_failed";
pub(super) const BLOB_STORAGE_ERROR_CODE_READINESS_CHECK_FAILED: &str = "readiness_check_failed";

///
/// BlobStorageTarget
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BlobStorageTarget {
    pub(super) input: String,
    pub(super) role: Option<String>,
    pub(super) canister_id: String,
    pub(super) candid_source: Option<BlobStorageCandidSource>,
}

impl BlobStorageTarget {
    pub(super) fn from_installed_deployment(
        input: &str,
        role: Option<String>,
        canister_id: &str,
    ) -> Self {
        Self {
            input: input.to_string(),
            role,
            canister_id: canister_id.to_string(),
            candid_source: Some(BlobStorageCandidSource::InstalledDeployment),
        }
    }
}

///
/// BlobStorageMethodMode
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum BlobStorageMethodMode {
    Query,
    Update,
}

impl BlobStorageMethodMode {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Query => "query",
            Self::Update => "update",
        }
    }
}

///
/// BlobStorageCandidSource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BlobStorageCandidSource {
    InstalledDeployment,
}

impl BlobStorageCandidSource {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::InstalledDeployment => BLOB_STORAGE_CANDID_SOURCE_INSTALLED_DEPLOYMENT,
        }
    }
}

impl Serialize for BlobStorageCandidSource {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.label())
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
    pub(super) candid_source: Option<BlobStorageCandidSource>,
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
    pub(super) kind: BlobStorageReportKind,
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
            kind: BlobStorageReportKind::Error,
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
/// BlobStorageReportKind
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BlobStorageReportKind {
    Error,
    Status,
}

impl BlobStorageReportKind {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Error => BLOB_STORAGE_ERROR_KIND,
            Self::Status => BLOB_STORAGE_STATUS_KIND,
        }
    }
}

impl Serialize for BlobStorageReportKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.label())
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
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

    pub(super) const fn result_kind(self) -> BlobStorageActionResultKind {
        match self {
            Self::SyncGateways => BlobStorageActionResultKind::SyncGateways,
            Self::Fund => BlobStorageActionResultKind::Fund,
        }
    }
}

///
/// BlobStorageActionResultKind
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BlobStorageActionResultKind {
    Fund,
    SyncGateways,
}

impl BlobStorageActionResultKind {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Fund => "blob_storage_fund_result",
            Self::SyncGateways => "blob_storage_sync_gateways_result",
        }
    }
}

impl Serialize for BlobStorageActionResultKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.label())
    }
}

///
/// BlobStorageAction
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BlobStorageAction {
    pub(super) name: BlobStorageActionName,
    pub(super) method: String,
    pub(super) mode: BlobStorageMethodMode,
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
    pub(super) kind: BlobStorageActionResultKind,
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
        mode: BlobStorageMethodMode,
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
        mode: BlobStorageMethodMode,
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
        method_mode: (&str, BlobStorageMethodMode),
        dry_run: bool,
        command: String,
        requested_cycles: Option<u128>,
    ) -> Self {
        let (method, mode) = method_mode;
        Self {
            schema_version: BLOB_STORAGE_JSON_SCHEMA_VERSION,
            kind: action_name.result_kind(),
            deployment: deployment.to_string(),
            target,
            action: BlobStorageAction {
                name: action_name,
                method: method.to_string(),
                mode,
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
    pub(super) kind: BlobStorageReportKind,
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
    pub(super) status: BlobStorageFundingStatusCode,
    pub(super) requested_cycles: Option<String>,
    pub(super) transferable_cycles: Option<String>,
}

///
/// BlobStorageFundingStatusCode
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum BlobStorageFundingStatusCode {
    CashierBalanceUnavailable,
    CashierResponseMalformed,
    FundingNeeded,
    NotConfigured,
    NotNeeded,
    ProjectCyclesReserveBlocksFunding,
}

impl BlobStorageFundingStatusCode {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::CashierBalanceUnavailable => BLOB_STORAGE_CODE_CASHIER_BALANCE_UNAVAILABLE,
            Self::CashierResponseMalformed => BLOB_STORAGE_CODE_CASHIER_RESPONSE_MALFORMED,
            Self::FundingNeeded => BLOB_STORAGE_CODE_FUNDING_NEEDED,
            Self::NotConfigured => BLOB_STORAGE_CODE_NOT_CONFIGURED,
            Self::NotNeeded => BLOB_STORAGE_CODE_NOT_NEEDED,
            Self::ProjectCyclesReserveBlocksFunding => {
                BLOB_STORAGE_CODE_PROJECT_CYCLES_RESERVE_BLOCKS_FUNDING
            }
        }
    }
}

///
/// BlobStorageReadinessStatus
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BlobStorageReadinessStatus {
    pub(super) state: BlobStorageReadinessState,
    pub(super) ready_for_upload: bool,
    pub(super) blockers: Vec<String>,
    pub(super) warnings: Vec<String>,
}

///
/// BlobStorageReadinessState
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum BlobStorageReadinessState {
    Blocked,
    Ready,
    Warning,
}

impl BlobStorageReadinessState {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Blocked => BLOB_STORAGE_READINESS_BLOCKED,
            Self::Ready => BLOB_STORAGE_READINESS_READY,
            Self::Warning => BLOB_STORAGE_READINESS_WARNING,
        }
    }
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
