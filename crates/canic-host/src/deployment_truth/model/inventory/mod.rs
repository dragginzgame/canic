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
    pub environment: String,
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

impl DeploymentRootObservationSourceV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::IcpCanisterStatus => "IcpCanisterStatus",
            Self::LocalDeploymentState => "LocalDeploymentState",
        }
    }
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
/// RoleAssignmentSourceV1
///
/// Machine-readable provenance for one observed role assignment.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoleAssignmentSourceV1 {
    IcpCanisterStatus,
    LocalInstallState,
    SubnetRegistry,
    SubnetRegistryAndIcpCanisterStatus,
}

impl RoleAssignmentSourceV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::IcpCanisterStatus => "icp_canister_status",
            Self::LocalInstallState => "local_install_state",
            Self::SubnetRegistry => "subnet_registry",
            Self::SubnetRegistryAndIcpCanisterStatus => "subnet_registry+icp_canister_status",
        }
    }

    #[must_use]
    pub fn label_includes_live_status(label: &str) -> bool {
        label == Self::IcpCanisterStatus.label()
            || label == Self::SubnetRegistryAndIcpCanisterStatus.label()
    }
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

impl CanisterControlClassV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::DeploymentControlled => "DeploymentControlled",
            Self::CanicManagedPool => "CanicManagedPool",
            Self::ExternallyImported => "ExternallyImported",
            Self::JointlyControlled => "JointlyControlled",
            Self::UserControlled => "UserControlled",
            Self::UnknownUnsafe => "UnknownUnsafe",
        }
    }
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

impl ObservationStatusV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::NotObserved => "NotObserved",
            Self::Observed => "Observed",
            Self::Missing => "Missing",
            Self::Inconclusive => "Inconclusive",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deployment_root_observation_source_owns_text_labels() {
        assert_eq!(
            DeploymentRootObservationSourceV1::IcpCanisterStatus.label(),
            "IcpCanisterStatus"
        );
        assert_eq!(
            DeploymentRootObservationSourceV1::LocalDeploymentState.label(),
            "LocalDeploymentState"
        );
    }

    #[test]
    fn role_assignment_sources_own_wire_labels() {
        let cases = [
            (
                RoleAssignmentSourceV1::IcpCanisterStatus,
                "icp_canister_status",
            ),
            (
                RoleAssignmentSourceV1::LocalInstallState,
                "local_install_state",
            ),
            (RoleAssignmentSourceV1::SubnetRegistry, "subnet_registry"),
            (
                RoleAssignmentSourceV1::SubnetRegistryAndIcpCanisterStatus,
                "subnet_registry+icp_canister_status",
            ),
        ];

        for (source, expected) in cases {
            assert_eq!(source.label(), expected);
        }
        assert!(RoleAssignmentSourceV1::label_includes_live_status(
            RoleAssignmentSourceV1::IcpCanisterStatus.label()
        ));
        assert!(RoleAssignmentSourceV1::label_includes_live_status(
            RoleAssignmentSourceV1::SubnetRegistryAndIcpCanisterStatus.label()
        ));
        assert!(!RoleAssignmentSourceV1::label_includes_live_status(
            "custom_status_source"
        ));
    }

    #[test]
    fn canister_control_class_owns_text_labels() {
        assert_eq!(
            CanisterControlClassV1::DeploymentControlled.label(),
            "DeploymentControlled"
        );
        assert_eq!(
            CanisterControlClassV1::CanicManagedPool.label(),
            "CanicManagedPool"
        );
        assert_eq!(
            CanisterControlClassV1::ExternallyImported.label(),
            "ExternallyImported"
        );
        assert_eq!(
            CanisterControlClassV1::JointlyControlled.label(),
            "JointlyControlled"
        );
        assert_eq!(
            CanisterControlClassV1::UserControlled.label(),
            "UserControlled"
        );
        assert_eq!(
            CanisterControlClassV1::UnknownUnsafe.label(),
            "UnknownUnsafe"
        );
    }

    #[test]
    fn observation_status_owns_text_labels() {
        assert_eq!(ObservationStatusV1::NotObserved.label(), "NotObserved");
        assert_eq!(ObservationStatusV1::Observed.label(), "Observed");
        assert_eq!(ObservationStatusV1::Missing.label(), "Missing");
        assert_eq!(ObservationStatusV1::Inconclusive.label(), "Inconclusive");
    }
}
