use super::*;
use crate::deployment_truth::observe::{
    apply_canister_control_to_observed_pool, apply_live_status_to_registry_observation,
    observed_root_from_status, registry_entries_to_observed_canisters,
    registry_entries_to_observed_pool,
};
use crate::deployment_truth::report::{RootSubnetEvidence, RootSubnetEvidenceSource};
use crate::icp::{IcpCanisterStatusReport, IcpCanisterStatusSettings};
use crate::install_root::{InstallState, RootVerificationStatus};
use crate::registry::RegistryEntry;
use crate::release_set::{ConfiguredPoolExpectation, ROOT_RELEASE_SET_MANIFEST_FILE};
use crate::test_support::temp_dir;
use serde::Serialize;
use std::fs;

mod authority;
mod comparison;
mod core;
mod diff;
mod execution_receipts;
mod lifecycle;
mod local_observation_plan;
mod promotion;
mod root_verification;

const SAMPLE_CONFIG: &str = r#"
controllers = []
app_index = []

[fleet]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[app]
init_mode = "enabled"
[app.whitelist]

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.user_hub]
kind = "service"
"#;

struct LimitedExecutor {
    context: DeploymentExecutionContextV1,
}

impl DeploymentExecutor for LimitedExecutor {
    fn execution_context(&self) -> DeploymentExecutionContextV1 {
        self.context.clone()
    }
}

struct FixtureRootSubnetEvidenceSource {
    result: Result<RootSubnetEvidence, String>,
}

impl RootSubnetEvidenceSource for FixtureRootSubnetEvidenceSource {
    fn root_subnet_evidence(
        &self,
        _network: &str,
        _icp_root: &std::path::Path,
        _canister_id: &str,
    ) -> Result<RootSubnetEvidence, String> {
        self.result.clone()
    }
}

fn sample_external_lifecycle_pending_artifacts()
-> (ExternalLifecyclePlanV1, ExternalLifecyclePendingReportV1) {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    let pending_report = external_lifecycle_pending_report_from_plan(
        "external-lifecycle-pending-1",
        &lifecycle_plan,
        &proposal_report,
    );
    (lifecycle_plan, pending_report)
}

fn sample_external_upgrade_proposal_and_receipt()
-> (ExternalUpgradeProposalV1, ExternalUpgradeReceiptV1) {
    let (proposal, receipt, _) = sample_external_upgrade_proposal_receipt_and_check();
    (proposal, receipt)
}

fn sample_external_upgrade_proposal_receipt_and_check() -> (
    ExternalUpgradeProposalV1,
    ExternalUpgradeReceiptV1,
    DeploymentCheckV1,
) {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    let proposal = proposal_report.proposals[0].clone();
    let receipt = external_upgrade_receipt_from_observation(
        "external-upgrade-receipt-1",
        &proposal,
        ExternalUpgradeConsentStateV1::ExecutedExternally,
        Some("user-principal".to_string()),
        Some(&check.inventory.observed_canisters[0]),
    );
    (proposal, receipt, check)
}

fn assert_plan_excludes_declared_only_store(plan: &DeploymentPlanV1) {
    assert!(
        plan.role_artifacts
            .iter()
            .all(|artifact| artifact.role != "store")
    );
    assert!(
        plan.unresolved_assumptions
            .iter()
            .all(|assumption| assumption.key != "local_artifacts.store")
    );
}

fn assert_plan_has_implicit_wasm_store_artifact(plan: &DeploymentPlanV1) {
    assert!(
        plan.role_artifacts
            .iter()
            .any(|artifact| artifact.role == "wasm_store"
                && artifact.source == ArtifactSourceV1::WasmStore
                && artifact.observed_wasm_gz_file_sha256_source
                    == Some(ArtifactDigestSourceV1::ObservedFileDigest))
    );
}

fn assert_plan_has_user_hub_release_artifact(plan: &DeploymentPlanV1) {
    assert!(
        plan.role_artifacts
            .iter()
            .any(|artifact| artifact.role == "user_hub"
                && artifact.wasm_gz_sha256.as_deref() == Some("user-hub-hash")
                && artifact.wasm_gz_sha256_source
                    == Some(ArtifactDigestSourceV1::ReleaseSetManifest)
                && artifact.observed_wasm_gz_file_sha256_source
                    == Some(ArtifactDigestSourceV1::ObservedFileDigest))
    );
}

fn assert_json_round_trip<T>(value: &T)
where
    T: Clone + std::fmt::Debug + Eq + serde::de::DeserializeOwned + Serialize,
{
    let encoded = serde_json::to_string(value).expect("value should encode");
    let decoded = serde_json::from_str::<T>(&encoded).expect("value should decode");
    assert_eq!(decoded, *value);
}

fn assert_object_keys(value: &serde_json::Value, expected: &[&str]) {
    let object = value.as_object().expect("value should be a JSON object");
    let mut actual = object.keys().map(String::as_str).collect::<Vec<_>>();
    actual.sort_unstable();
    let mut expected = expected.to_vec();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}

fn assert_required_policy_requirement(
    policy: &ExternalUpgradeVerificationPolicyV1,
    requirement: LifecycleVerificationRequirementV1,
    expected_value: Option<&str>,
) {
    let row = policy
        .verification_requirements
        .iter()
        .find(|row| row.requirement == requirement)
        .expect("verification requirement should be present");
    assert_eq!(
        row.status,
        ExternalUpgradeVerificationRequirementStatusV1::Required
    );
    assert_eq!(row.expected_value.as_deref(), expected_value);
}

fn matching_external_verification_observation(
    proposal: &ExternalUpgradeProposalV1,
) -> ExternalUpgradeVerificationObservationV1 {
    ExternalUpgradeVerificationObservationV1 {
        source: ExternalVerificationObservationSourceV1::SuppliedObservation,
        deployment_check_id: None,
        deployment_check_digest: None,
        inventory_id: Some("inventory-verified".to_string()),
        observed_at: Some("2026-05-26T00:00:00Z".to_string()),
        live_inventory_observed: true,
        controller_observation_present: true,
        observed_control_class: Some(proposal.control_class),
        observed_module_hash: proposal.target_installed_module_hash.clone(),
        observed_canonical_embedded_config_sha256: proposal
            .target_canonical_embedded_config_sha256
            .clone(),
        protected_call_ready: Some(true),
    }
}

fn sample_external_completion_sources() -> (
    ExternalUpgradeProposalV1,
    ExternalUpgradeConsentEvidenceV1,
    ExternalUpgradeVerificationCheckV1,
) {
    let (proposal, receipt, deployment_check) =
        sample_external_upgrade_proposal_receipt_and_check();
    let consent_evidence = external_upgrade_consent_evidence_from_receipt(
        "external-upgrade-consent-evidence-1",
        &proposal,
        &receipt,
    )
    .expect("consent evidence should build");
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let observation =
        external_upgrade_verification_observation_from_check(&policy, &deployment_check)
            .expect("sample deployment check should produce verification observation");
    let verification_check = external_upgrade_verification_check_from_policy(
        "external-upgrade-verification-check-1",
        &policy,
        observation,
    );
    (proposal, consent_evidence, verification_check)
}

fn assert_inventory_verification_mismatch(
    mutate: impl FnOnce(&mut DeploymentCheckV1),
    requirement: LifecycleVerificationRequirementV1,
) {
    let (proposal, _, mut deployment_check) = sample_external_upgrade_proposal_receipt_and_check();
    mutate(&mut deployment_check);
    let policy = external_upgrade_verification_policy_from_proposal(
        "external-upgrade-verification-policy-1",
        &proposal,
    );
    let observation =
        external_upgrade_verification_observation_from_check(&policy, &deployment_check)
            .expect("deployment check should still produce verification observation");
    let check = external_upgrade_verification_check_from_policy(
        "external-upgrade-verification-check-1",
        &policy,
        observation,
    );

    assert_eq!(
        check.observation.source,
        ExternalVerificationObservationSourceV1::DeploymentTruthInventory
    );
    assert_eq!(
        check.verification_result,
        ExternalUpgradeVerificationResultV1::Mismatch
    );
    assert!(check.requirement_results.iter().any(|row| {
        row.requirement == requirement
            && row.status == ExternalUpgradeVerificationRequirementStatusV1::Required
            && row.satisfied == Some(false)
    }));
    validate_external_upgrade_verification_check_for_deployment_check(
        &check,
        &policy,
        &deployment_check,
    )
    .expect("mismatch check should still validate against its source inventory");
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
        wasm_gz_size_bytes: Some(42),
        wasm_sha256: Some("wasm".to_string()),
        wasm_gz_sha256: Some("gzip".to_string()),
        wasm_gz_sha256_source: Some(ArtifactDigestSourceV1::ReleaseSetManifest),
        observed_wasm_gz_file_sha256: Some("file".to_string()),
        observed_wasm_gz_file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
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

fn sample_role_artifact_source(kind: RoleArtifactSourceKindV1) -> RoleArtifactSourceV1 {
    RoleArtifactSourceV1 {
        role: "root".to_string(),
        kind,
        locator: Some("artifacts/root.wasm.gz".to_string()),
        previous_receipt_kind: (kind == RoleArtifactSourceKindV1::PreviousReceiptArtifact)
            .then_some(PreviousArtifactReceiptKindV1::DeploymentReceipt),
        previous_receipt_lineage_digest: (kind
            == RoleArtifactSourceKindV1::PreviousReceiptArtifact)
            .then(|| sample_sha256("9")),
        expected_wasm_sha256: Some(sample_sha256("d")),
        expected_wasm_gz_sha256: Some(sample_sha256("a")),
        expected_candid_sha256: Some(sample_sha256("b")),
        expected_canonical_embedded_config_sha256: Some(sample_sha256("c")),
    }
}

fn sample_role_promotion_input(promotion_level: PromotionArtifactLevelV1) -> RolePromotionInputV1 {
    RolePromotionInputV1 {
        role: "root".to_string(),
        promotion_level,
        source: sample_role_artifact_source(RoleArtifactSourceKindV1::LocalWasmGz),
        require_byte_identical_wasm: promotion_level == PromotionArtifactLevelV1::SealedWasm,
        require_target_embedded_config: true,
        target_store_has_artifact: Some(true),
    }
}

fn sample_role_promotion_policy() -> RolePromotionPolicyV1 {
    RolePromotionPolicyV1 {
        role: "root".to_string(),
        allowed_promotion_levels: vec![PromotionArtifactLevelV1::SealedWasm],
        requirements: vec![
            PromotionPolicyRequirementV1::SameSourceRevision,
            PromotionPolicyRequirementV1::SameCargoFeatures,
            PromotionPolicyRequirementV1::TargetConfigDigest,
            PromotionPolicyRequirementV1::ByteIdenticalWasm,
            PromotionPolicyRequirementV1::SealedBytes,
        ],
    }
}

fn sample_build_recipe_identity() -> BuildRecipeIdentityV1 {
    BuildRecipeIdentityV1 {
        recipe_id: "recipe:root:debug".to_string(),
        source_kind: RoleArtifactSourceKindV1::WorkspacePackage,
        source_revision: "0123456789abcdef0123456789abcdef01234567".to_string(),
        source_tree_clean: true,
        package_or_role_selector: "root".to_string(),
        cargo_profile: "debug".to_string(),
        cargo_features_digest: sample_sha256("1"),
        cargo_lock_digest: sample_sha256("2"),
        rust_toolchain: "1.96.0".to_string(),
        builder_version: "canic-build-v1".to_string(),
        target_triple: "wasm32-unknown-unknown".to_string(),
        linker_identity: "rust-lld".to_string(),
        deterministic_build_mode: "locked".to_string(),
        wasm_opt_version: "not-used".to_string(),
        compression_identity: "gzip:default".to_string(),
    }
}

fn sample_build_materialization_input() -> BuildMaterializationInputV1 {
    BuildMaterializationInputV1 {
        materialization_input_id: "materialization-input:root:prod".to_string(),
        build_recipe_id: "recipe:root:debug".to_string(),
        canonical_embedded_config_sha256: sample_sha256("3"),
        network: "ic".to_string(),
        root_trust_anchor: "aaaaa-aa".to_string(),
        runtime_variant: "prod".to_string(),
    }
}

fn sample_build_materialization_result() -> BuildMaterializationResultV1 {
    BuildMaterializationResultV1 {
        materialization_result_id: "materialization-result:root:prod".to_string(),
        build_recipe_id: "recipe:root:debug".to_string(),
        materialization_input_digest: sample_sha256("4"),
        wasm_sha256: sample_sha256("5"),
        wasm_gz_sha256: sample_sha256("6"),
        installed_module_hash: sample_sha256("7"),
        candid_sha256: sample_sha256("8"),
    }
}

fn sample_build_materialization_evidence() -> BuildMaterializationEvidenceV1 {
    let input = sample_build_materialization_input();
    let mut result = sample_build_materialization_result();
    result.materialization_input_digest = build_materialization_input_digest(&input);
    build_materialization_evidence(BuildMaterializationEvidenceRequest {
        evidence_id: "materialization-evidence-1".to_string(),
        recipe: sample_build_recipe_identity(),
        materialization_input: input,
        materialization_result: result,
    })
    .expect("sample materialization evidence should validate")
}

fn sample_promotion_target_plan() -> DeploymentPlanV1 {
    let mut plan = sample_plan();
    plan.role_artifacts[0].wasm_sha256 = Some(sample_sha256("d"));
    plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("a"));
    plan.role_artifacts[0].canonical_embedded_config_sha256 = Some(sample_sha256("c"));
    plan
}

fn sample_promotion_transform() -> PromotionPlanTransformV1 {
    promoted_deployment_plan_transform_from_inputs(&PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    })
    .expect("sample promotion transform should validate")
}

fn sample_execution_preflight_for_plan(plan_id: &str) -> DeploymentExecutionPreflightV1 {
    DeploymentExecutionPreflightV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_id: plan_id.to_string(),
        safety_report_id: "report-1".to_string(),
        authority_plan_id: plan_id.to_string(),
        backend: DeploymentExecutorBackendV1::CurrentCli,
        status: DeploymentExecutionPreflightStatusV1::Ready,
        planned_phases: vec!["install_root".to_string(), "activate_root".to_string()],
        required_capabilities: vec![
            DeploymentExecutorCapabilityV1::StageArtifact,
            DeploymentExecutorCapabilityV1::InstallCode,
        ],
        missing_capabilities: Vec::new(),
        blockers: Vec::new(),
    }
}

fn sample_artifact_promotion_plan() -> ArtifactPromotionPlanV1 {
    let target_plan = sample_promotion_target_plan();
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let readiness = promotion_readiness_from_inputs(
        "promotion-ready-1",
        &target_plan,
        std::slice::from_ref(&input),
    );
    let artifact_identity_report =
        promotion_artifact_identity_report_from_inputs(PromotionArtifactIdentityReportRequest {
            report_id: "promotion-artifact-identity-1".to_string(),
            inputs: vec![input],
        })
        .expect("sample artifact identity report should validate");
    let transform = sample_promotion_transform();
    let target_execution_lineage =
        promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
            lineage_id: "target-execution-lineage-1".to_string(),
            generated_at: "2026-05-25T00:00:00Z".to_string(),
            transform: transform.clone(),
            execution_preflight: sample_execution_preflight_for_plan("promoted-plan-1"),
        })
        .expect("sample target execution lineage should validate");

    artifact_promotion_plan(ArtifactPromotionPlanRequest {
        plan_id: "artifact-promotion-plan-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        readiness,
        artifact_identity_report,
        transform,
        target_execution_lineage: Some(target_execution_lineage),
    })
    .expect("sample artifact promotion plan should validate")
}

fn sample_artifact_promotion_provenance_report() -> ArtifactPromotionProvenanceReportV1 {
    artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
        report_id: "promotion-provenance-1".to_string(),
        artifact_promotion_plan: sample_artifact_promotion_plan(),
        wasm_store_identity_report: Some(sample_wasm_store_identity_report()),
        wasm_store_catalog_verification: Some(sample_wasm_store_catalog_verification()),
        materialization_identity_report: Some(sample_materialization_identity_report()),
    })
    .expect("sample promotion provenance report should validate")
}

fn sample_artifact_promotion_execution_receipt() -> ArtifactPromotionExecutionReceiptV1 {
    artifact_promotion_execution_receipt(ArtifactPromotionExecutionReceiptRequest {
        receipt_id: "promotion-execution-receipt-1".to_string(),
        provenance_report: sample_artifact_promotion_provenance_report(),
        deployment_receipt: sample_promoted_deployment_receipt(),
    })
    .expect("sample promotion execution receipt should validate")
}

fn sample_promoted_deployment_receipt() -> DeploymentReceiptV1 {
    let mut receipt = sample_receipt_with_phase(
        "promoted-plan-1",
        Some("aaaaa-aa"),
        ObservationStatusV1::Observed,
        RolePhaseResultV1::Applied,
    );
    receipt.operation_id = "promoted-operation-1".to_string();
    receipt.phase_receipts[0].phase = "promote_artifacts".to_string();
    receipt.role_phase_receipts[0].phase = "install_root".to_string();
    receipt.role_phase_receipts[0].artifact_digest = Some(sample_sha256("5"));
    receipt.role_phase_receipts[0].observed_module_hash_after = Some(sample_sha256("7"));
    receipt.role_phase_receipts[0].canonical_embedded_config_sha256 = Some(sample_sha256("3"));
    receipt
}

fn sample_wasm_store_identity_report() -> PromotionWasmStoreIdentityReportV1 {
    promotion_wasm_store_identity_report_from_staging(PromotionWasmStoreIdentityReportRequest {
        report_id: "wasm-store-identity-1".to_string(),
        staging_receipts: vec![sample_wasm_store_staging_receipt()],
    })
    .expect("sample wasm-store identity report should validate")
}

fn sample_wasm_store_catalog_entry() -> PromotionWasmStoreCatalogEntryV1 {
    PromotionWasmStoreCatalogEntryV1 {
        locator: "root:aaaaa-aa:bootstrap".to_string(),
        artifact_identity: "embedded:root:0.44.0:abc123".to_string(),
        published_chunk_count: 2,
    }
}

fn sample_wasm_store_catalog_verification() -> PromotionWasmStoreCatalogVerificationV1 {
    promotion_wasm_store_catalog_verification(PromotionWasmStoreCatalogVerificationRequest {
        verification_id: "wasm-store-catalog-1".to_string(),
        wasm_store_identity_report: sample_wasm_store_identity_report(),
        catalog_entries: vec![sample_wasm_store_catalog_entry()],
    })
    .expect("sample wasm-store catalog verification should validate")
}

fn sample_materialization_identity_report() -> PromotionMaterializationIdentityReportV1 {
    promotion_materialization_identity_report_from_evidence(
        PromotionMaterializationIdentityReportRequest {
            report_id: "materialization-report-1".to_string(),
            evidence: vec![sample_build_materialization_evidence()],
        },
    )
    .expect("sample materialization identity report should validate")
}

fn sample_sha256(seed: &str) -> String {
    seed.repeat(64)
}

fn sample_plan() -> DeploymentPlanV1 {
    DeploymentPlanV1 {
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
    }
}

fn sample_matching_inventory() -> DeploymentInventoryV1 {
    DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-22T00:00:00Z".to_string(),
        observed_identity: Some(sample_identity()),
        observed_root: Some(sample_root_observation()),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("canic.toml".to_string()),
            raw_config_sha256: Some("raw".to_string()),
            canonical_embedded_config_sha256: Some("runtime".to_string()),
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
            role_assignment_source: Some("icp_canister_status".to_string()),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "root.wasm.gz".to_string(),
            file_sha256: Some("file".to_string()),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: Some("gzip".to_string()),
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
    }
}

fn sample_root_observation() -> DeploymentRootObservationV1 {
    DeploymentRootObservationV1 {
        deployment_name: "demo".to_string(),
        network: "local".to_string(),
        fleet_template: "root".to_string(),
        root_principal: "aaaaa-aa".to_string(),
        observed_canister_id: "aaaaa-aa".to_string(),
        observation_source: DeploymentRootObservationSourceV1::IcpCanisterStatus,
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("running".to_string()),
        role_assignment_source: Some("icp_canister_status".to_string()),
    }
}

fn sample_check(plan: DeploymentPlanV1, inventory: DeploymentInventoryV1) -> DeploymentCheckV1 {
    let diff = compare_plan_to_inventory(&plan, &inventory);
    let report = safety_report_from_diff("report-1", Some("diff-1".to_string()), &diff);
    DeploymentCheckV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        check_id: "check-1".to_string(),
        plan,
        inventory,
        diff,
        report,
    }
}

fn sample_root_verification_check() -> DeploymentCheckV1 {
    sample_check(
        sample_root_verification_plan(),
        sample_root_verification_inventory(),
    )
}

fn sample_root_verification_receipt() -> DeploymentRootVerificationReceiptV1 {
    let report = deployment_root_verification_report_from_check(sample_root_verification_request(
        sample_root_verification_check(),
    ));
    let mut receipt = DeploymentRootVerificationReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        receipt_id: "receipt-root-verification".to_string(),
        receipt_digest: String::new(),
        deployment_name: report.deployment_name,
        network: report.network,
        fleet_template: report.expected_fleet_template,
        root_principal: report.expected_root_principal,
        previous_root_verification: DeploymentRootVerificationStateV1::NotVerified,
        new_root_verification: DeploymentRootVerificationStateV1::Verified,
        state_transition:
            DeploymentRootVerificationStateTransitionV1::PromotedNotVerifiedToVerified,
        source_report_id: report.report_id,
        source_report_digest: report.report_digest,
        source_report_requested_at: report.requested_at,
        source_report_source: report.source,
        source_report_evidence_status: report.evidence_status,
        source_report_current_root_verification: report.current_root_verification,
        source_report_state_transition: report.state_transition,
        source_root_observation_source: report
            .observed_root_observation_source
            .expect("observed source"),
        source_observed_root_canister_id: report
            .observed_root_canister_id
            .expect("observed root canister id"),
        source_check_id: report.source_check_id,
        source_check_digest: report.source_check_digest,
        source_deployment_plan_id: report.source_deployment_plan_id,
        source_deployment_plan_digest: report.source_deployment_plan_digest,
        source_inventory_id: report.source_inventory_id,
        source_inventory_digest: report.source_inventory_digest,
        verified_at_unix_secs: 100,
        local_state_path: ".canic/local/deployments/demo.json".to_string(),
        local_state_digest_before: "a".repeat(64),
        local_state_digest_after: "b".repeat(64),
        warnings: Vec::new(),
    };
    receipt.receipt_digest = deployment_root_verification_receipt_digest(&receipt);
    receipt
}

fn sample_root_verification_plan() -> DeploymentPlanV1 {
    let mut plan = sample_plan();
    plan.deployment_identity.deployment_name = "demo".to_string();
    plan
}

fn sample_root_verification_inventory() -> DeploymentInventoryV1 {
    let mut inventory = sample_matching_inventory();
    if let Some(identity) = inventory.observed_identity.as_mut() {
        identity.deployment_name = "demo".to_string();
    }
    inventory
}

fn sample_root_verification_request(
    deployment_check: DeploymentCheckV1,
) -> DeploymentRootVerificationRequestV1 {
    DeploymentRootVerificationRequestV1 {
        report_id: "root-verification-report-1".to_string(),
        requested_at: "2026-05-27T00:00:00Z".to_string(),
        deployment_name: "demo".to_string(),
        network: "local".to_string(),
        expected_fleet_template: "root".to_string(),
        expected_root_principal: "aaaaa-aa".to_string(),
        current_root_verification: DeploymentRootVerificationStateV1::NotVerified,
        source: DeploymentRootVerificationSourceV1::DeploymentTruthCheck,
        deployment_check,
    }
}

fn sample_authority_evidence() -> AuthorityDryRunEvidenceV1 {
    sample_authority_evidence_from_check(sample_check(sample_plan(), sample_matching_inventory()))
}

fn sample_authority_evidence_from_check(check: DeploymentCheckV1) -> AuthorityDryRunEvidenceV1 {
    authority_dry_run_evidence_from_check(
        &check,
        "authority-evidence-1",
        "authority-report-1",
        "authority-dry-run-1",
        "2026-05-23T00:00:01Z",
    )
    .expect("build authority evidence")
}

fn sample_unknown_unsafe_check() -> DeploymentCheckV1 {
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "unsafe-canister".to_string(),
        role: Some("surprise".to_string()),
        control_class: CanisterControlClassV1::UnknownUnsafe,
        controllers: vec!["unknown-controller".to_string()],
        module_hash: None,
        status: None,
        root_trust_anchor: None,
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("icp_canister_status".to_string()),
    });

    sample_check(sample_plan(), inventory)
}

fn sample_receipt_with_phase(
    plan_id: &str,
    root_principal: Option<&str>,
    postcondition: ObservationStatusV1,
    role_result: RolePhaseResultV1,
) -> DeploymentReceiptV1 {
    DeploymentReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        operation_id: "operation-1".to_string(),
        plan_id: plan_id.to_string(),
        execution_context: None,
        operation_status: DeploymentExecutionStatusV1::Complete,
        started_at: "2026-05-22T00:00:00Z".to_string(),
        finished_at: Some("2026-05-22T00:00:01Z".to_string()),
        operator_principal: None,
        root_principal: root_principal.map(str::to_string),
        previous_observed_deployment_epoch: None,
        phase_receipts: vec![PhaseReceiptV1 {
            phase: "materialize_artifacts".to_string(),
            started_at: "2026-05-22T00:00:00Z".to_string(),
            finished_at: Some("2026-05-22T00:00:01Z".to_string()),
            attempted_action: "verify configured role artifacts are materialized".to_string(),
            verified_postcondition: VerifiedPostconditionV1 {
                status: postcondition,
                evidence: vec!["artifact:root:sha256:file".to_string()],
            },
        }],
        role_phase_receipts: vec![RolePhaseReceiptV1 {
            role: "root".to_string(),
            phase: "materialize_artifacts".to_string(),
            result: role_result,
            previous_module_hash: None,
            target_module_hash: Some("module".to_string()),
            observed_module_hash_after: None,
            artifact_digest: Some("file".to_string()),
            canonical_embedded_config_sha256: Some("canonical".to_string()),
            error: (role_result == RolePhaseResultV1::Failed)
                .then(|| "artifact_missing: missing observed artifact for role root".to_string()),
        }],
        final_inventory_id: Some("inventory-1".to_string()),
        command_result: DeploymentCommandResultV1::Succeeded,
    }
}

fn sample_wasm_store_staging_receipt() -> StagingReceiptV1 {
    StagingReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        role: "root".to_string(),
        artifact_identity: "embedded:root:0.44.0:abc123".to_string(),
        transport: ArtifactTransportV1::WasmStore,
        wasm_store_locator: Some("root:aaaaa-aa:bootstrap".to_string()),
        prepared_chunk_hashes: vec!["chunk-a".to_string(), "chunk-b".to_string()],
        published_chunk_count: 2,
        verified_postcondition: VerifiedPostconditionV1 {
            status: ObservationStatusV1::Observed,
            evidence: vec!["payload_sha256:abc123".to_string()],
        },
    }
}

fn sample_role_phase_receipt(result: RolePhaseResultV1) -> RolePhaseReceiptV1 {
    RolePhaseReceiptV1 {
        role: "root".to_string(),
        phase: "install_root".to_string(),
        result,
        previous_module_hash: None,
        target_module_hash: Some("module".to_string()),
        observed_module_hash_after: (result == RolePhaseResultV1::Applied)
            .then(|| "module".to_string()),
        artifact_digest: Some("file".to_string()),
        canonical_embedded_config_sha256: Some("canonical".to_string()),
        error: (result == RolePhaseResultV1::Failed).then(|| "install failed".to_string()),
    }
}

fn assert_sha256_len(value: Option<&String>) {
    assert_eq!(value.map(String::len), Some(64));
}

struct TempWorkspace {
    path: std::path::PathBuf,
}

impl TempWorkspace {
    fn new(name: &str) -> Self {
        let path = temp_dir(name);
        fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn write_artifact(icp_root: &Path, role: &str, bytes: &[u8]) {
    let path = icp_root
        .join(".icp")
        .join("local")
        .join("canisters")
        .join(role)
        .join(format!("{role}.wasm.gz"));
    fs::create_dir_all(path.parent().expect("artifact parent")).expect("create artifact dir");
    fs::write(path, bytes).expect("write artifact");
}

fn write_release_set_manifest(icp_root: &Path) {
    let path = icp_root
        .join(".icp")
        .join("local")
        .join("canisters")
        .join("root")
        .join(ROOT_RELEASE_SET_MANIFEST_FILE);
    let manifest = serde_json::json!({
        "release_version": "0.41.1",
        "entries": [{
            "role": "user_hub",
            "template_id": "embedded:user_hub",
            "artifact_relative_path": ".icp/local/canisters/user_hub/user_hub.wasm.gz",
            "payload_size_bytes": 17,
            "payload_sha256_hex": "user-hub-hash",
            "chunk_size_bytes": 1_048_576,
            "chunk_sha256_hex": ["user-hub-hash"]
        }]
    });
    fs::create_dir_all(path.parent().expect("manifest parent")).expect("create manifest dir");
    fs::write(
        path,
        serde_json::to_vec_pretty(&manifest).expect("encode manifest"),
    )
    .expect("write manifest");
}

fn write_deployment_state_json(icp_root: &Path, network: &str, state: InstallState) {
    let path = icp_root
        .join(".canic")
        .join(network)
        .join("deployments")
        .join(format!("{}.json", state.deployment_name));
    fs::create_dir_all(path.parent().expect("state parent")).expect("create state dir");
    fs::write(
        path,
        serde_json::to_vec_pretty(&state).expect("encode install state"),
    )
    .expect("write install state");
}

fn sample_install_state(deployment_name: &str, root_canister_id: &str) -> InstallState {
    InstallState {
        schema_version: 2,
        deployment_name: deployment_name.to_string(),
        fleet_template: "demo".to_string(),
        created_at_unix_secs: 1,
        updated_at_unix_secs: 1,
        network: "local".to_string(),
        root_target: "root".to_string(),
        root_canister_id: root_canister_id.to_string(),
        root_verification: RootVerificationStatus::Verified,
        root_build_target: "root".to_string(),
        workspace_root: "/workspace".to_string(),
        icp_root: "/workspace".to_string(),
        config_path: "fleets/canic.toml".to_string(),
        release_set_manifest_path: ".icp/local/canisters/root/release-set.json".to_string(),
    }
}
