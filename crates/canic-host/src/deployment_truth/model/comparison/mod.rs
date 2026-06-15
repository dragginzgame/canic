use super::{DeploymentIdentityV1, SafetyFindingV1, SafetySeverityV1, SafetyStatusV1};
use serde::{Deserialize, Serialize};

///
/// DeploymentComparisonReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentComparisonReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub report_digest: String,
    pub compared_at: String,
    pub left: DeploymentComparisonTargetV1,
    pub right: DeploymentComparisonTargetV1,
    pub status: SafetyStatusV1,
    pub identity_diff: Vec<DeploymentComparisonDiffV1>,
    pub artifact_diff: Vec<DeploymentComparisonDiffV1>,
    pub module_hash_diff: Vec<DeploymentComparisonDiffV1>,
    pub embedded_config_diff: Vec<DeploymentComparisonDiffV1>,
    pub authority_diff: Vec<DeploymentComparisonDiffV1>,
    pub pool_diff: Vec<DeploymentComparisonDiffV1>,
    pub verifier_readiness_diff: Vec<DeploymentComparisonDiffV1>,
    pub external_lifecycle_diff: Vec<DeploymentComparisonDiffV1>,
    pub hard_failures: Vec<SafetyFindingV1>,
    pub warnings: Vec<SafetyFindingV1>,
    pub next_actions: Vec<String>,
}

///
/// DeploymentComparisonTargetV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentComparisonTargetV1 {
    pub label: String,
    pub check_id: String,
    pub check_digest: String,
    pub plan_id: String,
    pub plan_digest: String,
    pub inventory_id: String,
    pub inventory_digest: String,
    pub deployment_identity: DeploymentIdentityV1,
}

///
/// DeploymentComparisonDiffV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentComparisonDiffV1 {
    pub category: DeploymentComparisonCategoryV1,
    pub subject: String,
    pub left: Option<String>,
    pub right: Option<String>,
    pub severity: SafetySeverityV1,
    pub message: String,
}

///
/// DeploymentComparisonCategoryV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum DeploymentComparisonCategoryV1 {
    Identity,
    TrustDomain,
    Artifact,
    ModuleHash,
    EmbeddedConfig,
    Authority,
    Pool,
    VerifierReadiness,
    ExternalLifecycle,
}
