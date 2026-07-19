use crate::deployment_truth::{DeploymentInventoryV1, RoleArtifactManifestV1};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

pub const ADOPTION_REPORT_SCHEMA_VERSION: u32 = 1;

///
/// AdoptionReportRequest
///
#[derive(Clone, Debug)]
pub struct AdoptionReportRequest<'a> {
    pub report_id: &'a str,
    pub generated_at: &'a str,
    pub profile: AdoptionProfileV1,
    pub config_source: &'a str,
    pub inventory: Option<&'a DeploymentInventoryV1>,
    pub artifact_manifest: Option<&'a RoleArtifactManifestV1>,
    pub package_metadata: Vec<AdoptionPackageMetadataV1>,
}

///
/// AdoptionReportError
///
#[derive(Debug, Eq, Error, PartialEq)]
pub enum AdoptionReportError {
    #[error("invalid config: {0}")]
    InvalidConfig(String),

    #[error("missing required [fleet].name in canic.toml")]
    MissingFleetName,
}

///
/// AdoptionProfileV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionProfileV1 {
    Brownfield,
    Partial,
    Standalone,
    LeafOnly,
    HybridExternalWasm,
    Minimal,
}

impl FromStr for AdoptionProfileV1 {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "brownfield" => Ok(Self::Brownfield),
            "partial" => Ok(Self::Partial),
            "standalone" => Ok(Self::Standalone),
            "leaf-only" => Ok(Self::LeafOnly),
            "hybrid-external-wasm" => Ok(Self::HybridExternalWasm),
            "minimal" => Ok(Self::Minimal),
            other => Err(format!("invalid adoption profile: {other}")),
        }
    }
}

///
/// AdoptionReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdoptionReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub generated_at: String,
    pub fleet: String,
    pub profile: AdoptionProfileV1,
    pub inputs: AdoptionReportInputsV1,
    pub summary: AdoptionReportSummaryV1,
    pub role_findings: Vec<AdoptionRoleFindingV1>,
    pub observed_canisters: Vec<AdoptionObservedCanisterFindingV1>,
    pub recommendations: Vec<AdoptionRecommendationV1>,
    pub blocked_actions: Vec<String>,
    pub warnings: Vec<String>,
}

///
/// AdoptionReportInputsV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdoptionReportInputsV1 {
    pub config_present: bool,
    pub inventory_id: Option<String>,
    pub artifact_manifest_id: Option<String>,
    pub package_metadata_count: usize,
    pub missing_or_stale_evidence: Vec<String>,
}

///
/// AdoptionReportSummaryV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdoptionReportSummaryV1 {
    pub managed_configured_roles: usize,
    pub declared_only_roles: usize,
    pub attached_unobserved_roles: usize,
    pub observed_only_canisters: usize,
    pub user_controlled_canisters: usize,
    pub external_controller_required: usize,
    pub evidence_conflicts: usize,
    pub mutating_actions_performed: usize,
}

///
/// AdoptionRoleFindingV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdoptionRoleFindingV1 {
    pub fleet: String,
    pub role: String,
    pub classifications: Vec<AdoptionClassificationV1>,
    pub declaration_state: AdoptionDeclarationStateV1,
    pub topology_state: AdoptionTopologyStateV1,
    pub package_state: AdoptionPackageStateV1,
    pub observation_state: AdoptionObservationStateV1,
    pub authority_state: AdoptionAuthorityStateV1,
    pub artifact_state: AdoptionArtifactStateV1,
    pub evidence: Vec<String>,
    pub recommendations: Vec<AdoptionRecommendationV1>,
    pub warnings: Vec<String>,
}

///
/// AdoptionObservedCanisterFindingV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdoptionObservedCanisterFindingV1 {
    pub canister_id: String,
    pub matched_fleet: Option<String>,
    pub matched_role: Option<String>,
    pub confidence: AdoptionMatchConfidenceV1,
    pub classifications: Vec<AdoptionClassificationV1>,
    pub controllers: Vec<String>,
    pub wasm_evidence: Option<String>,
    pub deployment_target_evidence: Option<String>,
    pub recommendations: Vec<AdoptionRecommendationV1>,
    pub warnings: Vec<String>,
}

///
/// AdoptionRecommendationV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdoptionRecommendationV1 {
    pub kind: String,
    pub severity: AdoptionRecommendationSeverityV1,
    pub description: String,
    pub suggested_action: Option<String>,
    pub suggested_action_effect: AdoptionSuggestedActionEffectV1,
    pub suggested_action_support: AdoptionSuggestedActionSupportV1,
    pub operator_action_requirement: AdoptionOperatorActionRequirementV1,
}

///
/// AdoptionPackageMetadataV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdoptionPackageMetadataV1 {
    pub package: String,
    pub fleet: Option<String>,
    pub role: Option<String>,
}

///
/// AdoptionClassificationV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum AdoptionClassificationV1 {
    Managed,
    DeclaredOnly,
    ObservedOnly,
    AttachedUnobserved,
    UserControlled,
    ExternalControllerRequired,
    ImportedPoolCandidate,
    EvidenceConflict,
}

///
/// AdoptionDeclarationStateV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionDeclarationStateV1 {
    Undeclared,
    Declared,
}

///
/// AdoptionTopologyStateV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionTopologyStateV1 {
    Unattached,
    Attached,
}

///
/// AdoptionObservationStateV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionObservationStateV1 {
    Unobserved,
    Observed,
    CandidateMatch,
    ConflictingMatch,
}

///
/// AdoptionAuthorityStateV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum AdoptionAuthorityStateV1 {
    CanicAuthorized,
    UserControlled,
    External,
    Unknown,
}

///
/// AdoptionArtifactStateV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionArtifactStateV1 {
    CanicBuilt,
    ExternalWasm,
    Unknown,
}

///
/// AdoptionPackageStateV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionPackageStateV1 {
    UndeclaredRole,
    NotChecked,
    Matches,
    MissingFleet,
    MissingRole,
    Mismatch,
}

///
/// AdoptionMatchConfidenceV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionMatchConfidenceV1 {
    None,
    Candidate,
    ExplicitEvidence,
    Conflict,
}

///
/// AdoptionRecommendationSeverityV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionRecommendationSeverityV1 {
    Info,
    Warning,
    Blocked,
}

///
/// AdoptionSuggestedActionEffectV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionSuggestedActionEffectV1 {
    ReadOnly,
    MutatesState,
}

///
/// AdoptionSuggestedActionSupportV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionSuggestedActionSupportV1 {
    SupportedByAdoption,
    UnsupportedByAdoption,
}

///
/// AdoptionOperatorActionRequirementV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdoptionOperatorActionRequirementV1 {
    Required,
    NotRequired,
}
