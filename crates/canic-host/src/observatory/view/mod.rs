//! Read-only observed facts and the smaller public presentation contract.

use serde::{Deserialize, Serialize};

/// Stable failure classes never carry private tool output or free-form runtime errors.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservationFailure {
    AuthorityUnavailable,
    BindingUnavailable,
    BudgetExceeded,
    InvalidResponse,
    ProfileMismatch,
    Rejected { code: u32 },
    TimedOut,
    TransportUnavailable,
    Stale,
    Unsupported,
}

/// Evidence source explicitly distinguishes retained authority and live replies.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationSource {
    LocalOperationJournal,
    PublicRoleOverview,
    ProtectedRoleStatus,
    RetainedTerminalReview,
}

/// Each field has its own observation time and failure; another role cannot fill it in.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Observation<T> {
    Observed {
        observed_at_unix_ms: u64,
        source: ObservationSource,
        value: T,
    },
    Unavailable {
        observed_at_unix_ms: u64,
        failure: ObservationFailure,
    },
}

/// Actual responsiveness and bootstrap readiness from the declared role's public contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoleOverviewView {
    pub bootstrap_ready: bool,
    pub canic_version: String,
    pub canister_version: u64,
}

/// Protected operating-funding facts, not a public balance or a conservation receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoleFundingView {
    pub native_cycles: String,
    pub automatic_funding_enabled: bool,
    pub policy_generation: u64,
    pub pending_operations: u64,
}

/// Root-owned physical estate totals; no individual asset identities are projected.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootEstateView {
    pub tracked: u32,
    pub stores: u32,
    pub workloads: u32,
    pub ready: u32,
    pub failed: u32,
    pub resetting: u32,
    pub pending_operations: u32,
}

/// Current Store occupancy and inventory, independent of historical metric counters.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StoreInventoryView {
    pub occupied_bytes: u64,
    pub maximum_bytes: u64,
    pub remaining_bytes: u64,
    pub retained_templates: u32,
    pub retained_releases: u32,
    pub approved_catalog_entries: u64,
    pub expected_template_chunks: u64,
    pub stored_template_chunks: u64,
    pub fixture_sources: u64,
    pub expected_fixture_chunks: u64,
    pub stored_fixture_chunks: u64,
    pub gc_state: String,
}

/// Exact private source and independent observations for one retained role instance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservatoryRoleView {
    pub role: String,
    pub canister_id: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub parent_canister_id: Option<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub subnet_id: Option<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub release_identity: Option<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub expected_module_sha256: Option<String>,
    pub overview: Observation<RoleOverviewView>,
    pub funding: Observation<RoleFundingView>,
    pub estate: Observation<RootEstateView>,
    pub store: Observation<StoreInventoryView>,
}

/// Private terminal provenance without controller lists, admission users or mutation proofs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservatoryAuthorityView {
    pub app: String,
    pub canonical_network_id: String,
    pub plan_sha256: String,
    pub registry_revision: u64,
    pub admission_principals: usize,
}

/// Local durable progress is evidence of intents and receipts, not a live runtime assertion.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalOperationView {
    pub operation_id: String,
    pub plan_sha256: String,
    pub completion: LocalOperationCompletion,
    pub applied_effects: usize,
    pub pending_effects: usize,
    pub funding_review_required: bool,
    pub stalled_observations: u32,
}

/// Closed local completion classes; the report grants no permission to resume an operation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalOperationCompletion {
    Converged,
    Prepared,
    InProgress,
    ReplanRequired,
}

/// One bounded collection, including unavailable authority instead of invented role success.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservatorySnapshotView {
    pub schema_version: u16,
    pub environment: String,
    pub fleet: String,
    pub collected_at_unix_ms: u64,
    pub freshness_secs: u32,
    pub collection_elapsed_ms: u64,
    pub remote_call_attempts: u64,
    pub authority: Observation<ObservatoryAuthorityView>,
    pub operation: Observation<LocalOperationView>,
    pub roles: Vec<ObservatoryRoleView>,
}

/// Public role projection deliberately omits Principals, hashes, placement and exact funding.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublicObservatoryRoleView {
    pub key: usize,
    pub label: String,
    pub role: String,
    pub overview: Observation<RoleOverviewView>,
    pub store: Observation<StoreInventoryView>,
}

/// Publishable JSON/HTML input; profiles cannot reintroduce private fields.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublicObservatoryView {
    pub schema_version: u16,
    pub title: String,
    pub collected_at_unix_ms: u64,
    pub freshness_secs: u32,
    pub authority_available: bool,
    pub roles: Vec<PublicObservatoryRoleView>,
}

/// Framework-neutral HTTP adapter output; the downstream application owns routing and serving.
pub struct ObservatoryHttpView {
    pub status: u16,
    pub content_type: &'static str,
    pub cache_control: &'static str,
    pub content_security_policy: &'static str,
    pub body: Vec<u8>,
}
