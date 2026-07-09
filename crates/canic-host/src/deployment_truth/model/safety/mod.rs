use super::inventory::DeploymentInventoryV1;
use super::plan::{DeploymentIdentityV1, DeploymentPlanV1};
use serde::{Deserialize, Serialize};

///
/// DeploymentDiffV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentDiffV1 {
    pub schema_version: u32,
    pub plan_identity: DeploymentIdentityV1,
    pub observed_identity: Option<DeploymentIdentityV1>,
    pub artifact_diff: Vec<DiffItemV1>,
    pub controller_diff: Vec<DiffItemV1>,
    pub pool_diff: Vec<DiffItemV1>,
    pub embedded_config_diff: Vec<DiffItemV1>,
    pub module_hash_diff: Vec<DiffItemV1>,
    pub verifier_readiness_diff: Vec<DiffItemV1>,
    pub resume_safety: ResumeSafetyV1,
    pub hard_failures: Vec<SafetyFindingV1>,
    pub warnings: Vec<SafetyFindingV1>,
    pub resumable_phases: Vec<String>,
}

///
/// SafetyReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SafetyReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub diff_id: Option<String>,
    pub status: SafetyStatusV1,
    pub summary: String,
    pub hard_failures: Vec<SafetyFindingV1>,
    pub warnings: Vec<SafetyFindingV1>,
    pub next_actions: Vec<String>,
}

///
/// DeploymentCheckV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentCheckV1 {
    pub schema_version: u32,
    pub check_id: String,
    pub plan: DeploymentPlanV1,
    pub inventory: DeploymentInventoryV1,
    pub diff: DeploymentDiffV1,
    pub report: SafetyReportV1,
}

///
/// DiffItemV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffItemV1 {
    pub category: String,
    pub subject: String,
    pub expected: Option<String>,
    pub observed: Option<String>,
    pub severity: SafetySeverityV1,
}

///
/// ResumeSafetyV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResumeSafetyV1 {
    pub status: SafetyStatusV1,
    pub reasons: Vec<String>,
}

///
/// SafetyFindingV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SafetyFindingV1 {
    pub code: String,
    pub message: String,
    pub severity: SafetySeverityV1,
    pub subject: Option<String>,
}

///
/// SafetyStatusV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SafetyStatusV1 {
    NotEvaluated,
    Safe,
    Warning,
    Blocked,
}

impl SafetyStatusV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::NotEvaluated => "not_evaluated",
            Self::Safe => "safe",
            Self::Warning => "warning",
            Self::Blocked => "blocked",
        }
    }
}

///
/// SafetySeverityV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SafetySeverityV1 {
    Info,
    Warning,
    HardFailure,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safety_status_owns_text_labels() {
        assert_eq!(SafetyStatusV1::NotEvaluated.label(), "not_evaluated");
        assert_eq!(SafetyStatusV1::Safe.label(), "safe");
        assert_eq!(SafetyStatusV1::Warning.label(), "warning");
        assert_eq!(SafetyStatusV1::Blocked.label(), "blocked");
    }
}
