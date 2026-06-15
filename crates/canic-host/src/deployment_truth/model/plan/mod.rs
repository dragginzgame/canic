use super::artifact::RoleArtifactV1;
use super::inventory::{
    ExpectedCanisterV1, ExpectedPoolCanisterV1, VerifierReadinessExpectationV1,
};
use serde::{Deserialize, Serialize};

///
/// DeploymentPlanV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentPlanV1 {
    pub schema_version: u32,
    pub plan_id: String,
    pub deployment_identity: DeploymentIdentityV1,
    pub trust_domain: TrustDomainV1,
    pub fleet_template: String,
    pub runtime_variant: String,
    pub authority_profile: AuthorityProfileV1,
    pub role_artifacts: Vec<RoleArtifactV1>,
    pub expected_canisters: Vec<ExpectedCanisterV1>,
    pub expected_pool: Vec<ExpectedPoolCanisterV1>,
    pub expected_verifier_readiness: VerifierReadinessExpectationV1,
    pub unresolved_assumptions: Vec<DeploymentAssumptionV1>,
}

///
/// DeploymentIdentityV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentIdentityV1 {
    pub deployment_name: String,
    pub network: String,
    pub root_principal: Option<String>,
    pub authority_profile_hash: Option<String>,
    pub role_topology_hash: Option<String>,
    pub deployment_manifest_digest: Option<String>,
    pub canonical_runtime_config_digest: Option<String>,
    pub role_embedded_config_set_digest: Option<String>,
    pub artifact_set_digest: Option<String>,
    pub pool_identity_set_digest: Option<String>,
    pub canic_version: Option<String>,
    pub ic_memory_version: Option<String>,
}

///
/// TrustDomainV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TrustDomainV1 {
    pub root_trust_anchor: Option<String>,
    pub migration_from: Option<String>,
}

///
/// AuthorityProfileV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityProfileV1 {
    pub profile_id: String,
    pub expected_controllers: Vec<String>,
    pub staging_controllers: Vec<String>,
    pub emergency_controllers: Vec<String>,
}

///
/// DeploymentAssumptionV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentAssumptionV1 {
    pub key: String,
    pub description: String,
}
