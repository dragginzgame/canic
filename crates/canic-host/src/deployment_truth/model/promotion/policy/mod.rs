use super::super::SafetyFindingV1;
use super::source::{PromotionArtifactLevelV1, PromotionReadinessStatusV1};
use serde::{Deserialize, Serialize};

///
/// RolePromotionPolicyV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionPolicyV1 {
    pub role: String,
    pub allowed_promotion_levels: Vec<PromotionArtifactLevelV1>,
    pub requirements: Vec<PromotionPolicyRequirementV1>,
}

///
/// PromotionPolicyRequirementV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PromotionPolicyRequirementV1 {
    SameSourceRevision,
    SameCargoFeatures,
    TargetConfigDigest,
    ByteIdenticalWasm,
    SealedBytes,
}

///
/// PromotionPolicyClaimV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PromotionPolicyClaimV1 {
    ByteIdenticalWasm,
    TargetConfigDigest,
}

///
/// PromotionPolicyCheckV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionPolicyCheckV1 {
    pub schema_version: u32,
    pub check_id: String,
    pub promotion_policy_check_digest: String,
    pub status: PromotionReadinessStatusV1,
    pub roles: Vec<RolePromotionPolicyDecisionV1>,
    pub blockers: Vec<SafetyFindingV1>,
}

///
/// RolePromotionPolicyDecisionV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionPolicyDecisionV1 {
    pub role: String,
    pub requested_promotion_level: PromotionArtifactLevelV1,
    pub allowed_promotion_levels: Vec<PromotionArtifactLevelV1>,
    pub requirements: Vec<PromotionPolicyRequirementV1>,
    pub claims: Vec<PromotionPolicyClaimV1>,
    pub level_allowed: bool,
    pub policy_satisfied: bool,
}
