use super::artifact::ObservedArtifactV1;
use super::plan::DeploymentIdentityV1;
use serde::{Deserialize, Serialize};

///
/// DeploymentInventoryV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentInventoryV1 {
    pub schema_version: u32,
    pub inventory_id: String,
    pub observed_at: String,
    pub observed_identity: Option<DeploymentIdentityV1>,
    pub observed_root: Option<DeploymentRootObservationV1>,
    pub local_config: LocalDeploymentConfigV1,
    pub observed_canisters: Vec<ObservedCanisterV1>,
    pub observed_pool: Vec<ObservedPoolCanisterV1>,
    pub observed_artifacts: Vec<ObservedArtifactV1>,
    pub observed_verifier_readiness: VerifierReadinessObservationV1,
    pub unresolved_observations: Vec<DeploymentObservationGapV1>,
}

///
/// DeploymentRootObservationV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentRootObservationV1 {
    pub deployment_name: String,
    pub network: String,
    pub fleet_template: String,
    pub root_principal: String,
    pub observed_canister_id: String,
    pub observation_source: DeploymentRootObservationSourceV1,
    pub control_class: CanisterControlClassV1,
    pub controllers: Vec<String>,
    pub module_hash: Option<String>,
    pub status: Option<String>,
    pub role_assignment_source: Option<String>,
}

///
/// DeploymentRootObservationSourceV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DeploymentRootObservationSourceV1 {
    IcpCanisterStatus,
    LocalDeploymentState,
}

///
/// ExpectedCanisterV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExpectedCanisterV1 {
    pub role: String,
    pub canister_id: Option<String>,
    pub control_class: CanisterControlClassV1,
}

///
/// ObservedCanisterV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObservedCanisterV1 {
    pub canister_id: String,
    pub role: Option<String>,
    pub control_class: CanisterControlClassV1,
    pub controllers: Vec<String>,
    pub module_hash: Option<String>,
    pub status: Option<String>,
    pub root_trust_anchor: Option<String>,
    pub canonical_embedded_config_digest: Option<String>,
    pub role_assignment_source: Option<String>,
}

///
/// CanisterControlClassV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterControlClassV1 {
    DeploymentControlled,
    CanicManagedPool,
    ExternallyImported,
    JointlyControlled,
    UserControlled,
    UnknownUnsafe,
}

///
/// ExpectedPoolCanisterV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExpectedPoolCanisterV1 {
    pub pool: String,
    pub canister_id: Option<String>,
    pub role: Option<String>,
}

///
/// ObservedPoolCanisterV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObservedPoolCanisterV1 {
    pub pool: String,
    pub canister_id: String,
    pub role: Option<String>,
    pub control_class: CanisterControlClassV1,
}

///
/// LocalDeploymentConfigV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LocalDeploymentConfigV1 {
    pub config_path: Option<String>,
    pub raw_config_sha256: Option<String>,
    pub canonical_embedded_config_sha256: Option<String>,
}

///
/// VerifierReadinessExpectationV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerifierReadinessExpectationV1 {
    pub required: bool,
    pub expected_role_epochs: Vec<RoleEpochExpectationV1>,
}

///
/// VerifierReadinessObservationV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerifierReadinessObservationV1 {
    pub status: ObservationStatusV1,
    pub role_epochs: Vec<RoleEpochObservationV1>,
}

///
/// RoleEpochExpectationV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleEpochExpectationV1 {
    pub role: String,
    pub minimum_epoch: u64,
}

///
/// RoleEpochObservationV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleEpochObservationV1 {
    pub role: String,
    pub observed_epoch: Option<u64>,
    pub status: ObservationStatusV1,
}

///
/// DeploymentObservationGapV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentObservationGapV1 {
    pub key: String,
    pub description: String,
}

///
/// ObservationStatusV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ObservationStatusV1 {
    NotObserved,
    Observed,
    Missing,
    Inconclusive,
}
