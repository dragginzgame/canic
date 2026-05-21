//! Passive deployment-truth model types for host-side planning and safety checks.

use serde::{Deserialize, Serialize};

pub const DEPLOYMENT_TRUTH_SCHEMA_VERSION: u32 = 1;

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
/// DeploymentInventoryV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentInventoryV1 {
    pub schema_version: u32,
    pub inventory_id: String,
    pub observed_at: String,
    pub observed_identity: Option<DeploymentIdentityV1>,
    pub local_config: LocalDeploymentConfigV1,
    pub observed_canisters: Vec<ObservedCanisterV1>,
    pub observed_pool: Vec<ObservedPoolCanisterV1>,
    pub observed_artifacts: Vec<ObservedArtifactV1>,
    pub observed_verifier_readiness: VerifierReadinessObservationV1,
    pub unresolved_observations: Vec<DeploymentObservationGapV1>,
}

///
/// DeploymentReceiptV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentReceiptV1 {
    pub schema_version: u32,
    pub operation_id: String,
    pub plan_id: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub operator_principal: Option<String>,
    pub root_principal: Option<String>,
    pub previous_observed_deployment_epoch: Option<u64>,
    pub phase_receipts: Vec<PhaseReceiptV1>,
    pub final_inventory_id: Option<String>,
    pub command_result: DeploymentCommandResultV1,
}

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
/// RoleArtifactV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleArtifactV1 {
    pub role: String,
    pub source: ArtifactSourceV1,
    pub build_profile: String,
    pub wasm_path: Option<String>,
    pub wasm_gz_path: Option<String>,
    pub wasm_sha256: Option<String>,
    pub wasm_gz_sha256: Option<String>,
    pub installed_module_hash: Option<String>,
    pub candid_path: Option<String>,
    pub candid_sha256: Option<String>,
    pub raw_config_sha256: Option<String>,
    pub canonical_embedded_config_sha256: Option<String>,
    pub embedded_topology_sha256: Option<String>,
    pub builder_version: Option<String>,
    pub rust_toolchain: Option<String>,
    pub package_version: Option<String>,
}

///
/// ArtifactSourceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ArtifactSourceV1 {
    LocalBuild,
    ReleaseSet,
    WasmStore,
    External,
    Unknown,
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
/// ObservedArtifactV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObservedArtifactV1 {
    pub role: String,
    pub artifact_path: String,
    pub payload_sha256: Option<String>,
    pub payload_size_bytes: Option<u64>,
    pub source: ArtifactSourceV1,
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
/// DeploymentAssumptionV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentAssumptionV1 {
    pub key: String,
    pub description: String,
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
/// PhaseReceiptV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PhaseReceiptV1 {
    pub phase: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub attempted_action: String,
    pub verified_postcondition: VerifiedPostconditionV1,
}

///
/// VerifiedPostconditionV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerifiedPostconditionV1 {
    pub status: ObservationStatusV1,
    pub evidence: Vec<String>,
}

///
/// DeploymentCommandResultV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DeploymentCommandResultV1 {
    NotFinished,
    Succeeded,
    Failed { code: String, message: String },
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SafetyStatusV1 {
    NotEvaluated,
    Safe,
    Warning,
    Blocked,
}

///
/// SafetySeverityV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SafetySeverityV1 {
    Info,
    Warning,
    HardFailure,
}

///
/// ObservationStatusV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ObservationStatusV1 {
    NotObserved,
    Observed,
    Missing,
    Inconclusive,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_round_trips_through_json() {
        let plan = DeploymentPlanV1 {
            schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            plan_id: "plan-local-root".to_string(),
            deployment_identity: sample_identity(),
            trust_domain: TrustDomainV1 {
                root_trust_anchor: Some("aaaaa-aa".to_string()),
                migration_from: None,
            },
            fleet_template: "root".to_string(),
            runtime_variant: "local".to_string(),
            authority_profile: AuthorityProfileV1 {
                profile_id: "local-default".to_string(),
                expected_controllers: vec!["aaaaa-aa".to_string()],
                staging_controllers: Vec::new(),
                emergency_controllers: Vec::new(),
            },
            role_artifacts: vec![sample_role_artifact()],
            expected_canisters: vec![ExpectedCanisterV1 {
                role: "root".to_string(),
                canister_id: Some("aaaaa-aa".to_string()),
                control_class: CanisterControlClassV1::DeploymentControlled,
            }],
            expected_pool: Vec::new(),
            expected_verifier_readiness: VerifierReadinessExpectationV1 {
                required: true,
                expected_role_epochs: vec![RoleEpochExpectationV1 {
                    role: "root".to_string(),
                    minimum_epoch: 1,
                }],
            },
            unresolved_assumptions: Vec::new(),
        };

        let encoded = serde_json::to_string(&plan).expect("plan should encode");
        let decoded =
            serde_json::from_str::<DeploymentPlanV1>(&encoded).expect("plan should decode");

        assert_eq!(decoded, plan);
    }

    #[test]
    fn inventory_round_trips_through_json() {
        let inventory = DeploymentInventoryV1 {
            schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            inventory_id: "inventory-1".to_string(),
            observed_at: "2026-05-21T00:00:00Z".to_string(),
            observed_identity: Some(sample_identity()),
            local_config: LocalDeploymentConfigV1 {
                config_path: Some("icp.yml".to_string()),
                raw_config_sha256: Some("raw".to_string()),
                canonical_embedded_config_sha256: Some("canonical".to_string()),
            },
            observed_canisters: vec![ObservedCanisterV1 {
                canister_id: "aaaaa-aa".to_string(),
                role: Some("root".to_string()),
                control_class: CanisterControlClassV1::DeploymentControlled,
                controllers: vec!["aaaaa-aa".to_string()],
                module_hash: Some("module".to_string()),
                status: Some("running".to_string()),
                root_trust_anchor: Some("aaaaa-aa".to_string()),
                canonical_embedded_config_digest: Some("canonical".to_string()),
                role_assignment_source: Some("registry".to_string()),
            }],
            observed_pool: Vec::new(),
            observed_artifacts: vec![ObservedArtifactV1 {
                role: "root".to_string(),
                artifact_path: ".icp/local/canisters/root/root.wasm.gz".to_string(),
                payload_sha256: Some("artifact".to_string()),
                payload_size_bytes: Some(42),
                source: ArtifactSourceV1::LocalBuild,
            }],
            observed_verifier_readiness: VerifierReadinessObservationV1 {
                status: ObservationStatusV1::Observed,
                role_epochs: vec![RoleEpochObservationV1 {
                    role: "root".to_string(),
                    observed_epoch: Some(1),
                    status: ObservationStatusV1::Observed,
                }],
            },
            unresolved_observations: Vec::new(),
        };

        let encoded = serde_json::to_string_pretty(&inventory).expect("inventory should encode");
        let decoded = serde_json::from_str::<DeploymentInventoryV1>(&encoded)
            .expect("inventory should decode");

        assert_eq!(decoded, inventory);
    }

    #[test]
    fn receipt_diff_and_safety_report_support_not_evaluated_state() {
        let receipt = DeploymentReceiptV1 {
            schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            operation_id: "operation-1".to_string(),
            plan_id: "plan-local-root".to_string(),
            started_at: "2026-05-21T00:00:00Z".to_string(),
            finished_at: None,
            operator_principal: None,
            root_principal: Some("aaaaa-aa".to_string()),
            previous_observed_deployment_epoch: None,
            phase_receipts: vec![PhaseReceiptV1 {
                phase: "build_artifacts".to_string(),
                started_at: "2026-05-21T00:00:00Z".to_string(),
                finished_at: None,
                attempted_action: "build root artifact".to_string(),
                verified_postcondition: VerifiedPostconditionV1 {
                    status: ObservationStatusV1::NotObserved,
                    evidence: Vec::new(),
                },
            }],
            final_inventory_id: None,
            command_result: DeploymentCommandResultV1::NotFinished,
        };
        let diff = DeploymentDiffV1 {
            schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            plan_identity: sample_identity(),
            observed_identity: None,
            artifact_diff: Vec::new(),
            controller_diff: Vec::new(),
            pool_diff: Vec::new(),
            embedded_config_diff: Vec::new(),
            module_hash_diff: Vec::new(),
            verifier_readiness_diff: Vec::new(),
            resume_safety: ResumeSafetyV1 {
                status: SafetyStatusV1::NotEvaluated,
                reasons: vec!["inventory not collected".to_string()],
            },
            hard_failures: Vec::new(),
            warnings: Vec::new(),
            resumable_phases: Vec::new(),
        };
        let report = SafetyReportV1 {
            schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            report_id: "report-1".to_string(),
            diff_id: None,
            status: SafetyStatusV1::NotEvaluated,
            summary: "deployment safety has not been evaluated".to_string(),
            hard_failures: Vec::new(),
            warnings: Vec::new(),
            next_actions: vec!["collect deployment inventory".to_string()],
        };

        assert_json_round_trip(&receipt);
        assert_json_round_trip(&diff);
        assert_json_round_trip(&report);
    }

    fn assert_json_round_trip<T>(value: &T)
    where
        T: Clone + std::fmt::Debug + Eq + serde::de::DeserializeOwned + Serialize,
    {
        let encoded = serde_json::to_string(value).expect("value should encode");
        let decoded = serde_json::from_str::<T>(&encoded).expect("value should decode");
        assert_eq!(decoded, *value);
    }

    fn sample_identity() -> DeploymentIdentityV1 {
        DeploymentIdentityV1 {
            deployment_name: "local-root".to_string(),
            network: "local".to_string(),
            root_principal: Some("aaaaa-aa".to_string()),
            authority_profile_hash: Some("authority".to_string()),
            role_topology_hash: Some("topology".to_string()),
            deployment_manifest_digest: Some("manifest".to_string()),
            canonical_runtime_config_digest: Some("runtime".to_string()),
            role_embedded_config_set_digest: Some("embedded".to_string()),
            artifact_set_digest: Some("artifacts".to_string()),
            pool_identity_set_digest: None,
            canic_version: Some("0.41.0".to_string()),
            ic_memory_version: Some("0.6.1".to_string()),
        }
    }

    fn sample_role_artifact() -> RoleArtifactV1 {
        RoleArtifactV1 {
            role: "root".to_string(),
            source: ArtifactSourceV1::LocalBuild,
            build_profile: "fast".to_string(),
            wasm_path: Some("root.wasm".to_string()),
            wasm_gz_path: Some("root.wasm.gz".to_string()),
            wasm_sha256: Some("wasm".to_string()),
            wasm_gz_sha256: Some("gzip".to_string()),
            installed_module_hash: Some("module".to_string()),
            candid_path: Some("root.did".to_string()),
            candid_sha256: Some("did".to_string()),
            raw_config_sha256: Some("raw".to_string()),
            canonical_embedded_config_sha256: Some("canonical".to_string()),
            embedded_topology_sha256: Some("topology".to_string()),
            builder_version: Some("0.41.0".to_string()),
            rust_toolchain: Some("stable".to_string()),
            package_version: Some("0.41.0".to_string()),
        }
    }
}
