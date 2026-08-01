use canic_core::ids::{CanonicalNetworkId, FleetId};
use canic_host::deployment_truth::{
    ArtifactDigestSourceV1, ArtifactSourceV1, AuthorityProfileV1, CanisterControlClassV1,
    DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentCheckV1, DeploymentDiffV1, DeploymentIdentityV1,
    DeploymentInventoryV1, DeploymentPlanV1, ExpectedCanisterV1, LocalDeploymentConfigV1,
    ObservationStatusV1, ObservedCanisterV1, ResumeSafetyV1, RoleArtifactV1, SafetyReportV1,
    SafetyStatusV1, TrustDomainV1, VerifierReadinessExpectationV1, VerifierReadinessObservationV1,
};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) fn sample_authority_check() -> DeploymentCheckV1 {
    let identity = sample_deployment_identity();
    let plan = sample_deployment_plan(identity.clone());
    let inventory = sample_deployment_inventory(identity);
    let diff = sample_deployment_diff(&plan, &inventory);
    let report = sample_safety_report();

    DeploymentCheckV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        check_id: "check-1".to_string(),
        plan,
        inventory,
        diff,
        report,
    }
}

fn sample_deployment_identity() -> DeploymentIdentityV1 {
    DeploymentIdentityV1 {
        canonical_network_id: Some(CanonicalNetworkId::ic_mainnet()),
        fleet_id: Some(FleetId::from_generated_bytes([2; 32])),
        fleet_name: "demo".to_string(),
        app: "demo".to_string(),
        environment: "local".to_string(),
        root_principal: Some("aaaaa-aa".to_string()),
        authority_profile_hash: Some("authority".to_string()),
        role_topology_hash: None,
        deployment_manifest_digest: None,
        canonical_runtime_config_digest: None,
        role_embedded_config_set_digest: None,
        artifact_set_digest: None,
        pool_identity_set_digest: None,
        canic_version: None,
        ic_memory_version: None,
    }
}

fn sample_deployment_plan(identity: DeploymentIdentityV1) -> DeploymentPlanV1 {
    DeploymentPlanV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_id: "plan-1".to_string(),
        deployment_identity: identity,
        trust_domain: TrustDomainV1 {
            root_trust_anchor: Some("aaaaa-aa".to_string()),
        },
        runtime_variant: "local".to_string(),
        authority_profile: AuthorityProfileV1 {
            profile_id: "authority-profile-1".to_string(),
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
            required: false,
            expected_role_epochs: Vec::new(),
        },
        unresolved_assumptions: Vec::new(),
    }
}

pub(super) fn sample_role_artifact() -> RoleArtifactV1 {
    RoleArtifactV1 {
        role: "root".to_string(),
        source: ArtifactSourceV1::LocalBuild,
        build_profile: "fast".to_string(),
        wasm_path: Some("artifacts/root.wasm".to_string()),
        wasm_gz_path: Some("artifacts/root.wasm.gz".to_string()),
        wasm_gz_size_bytes: Some(123),
        wasm_sha256: Some(sample_sha256("d")),
        wasm_gz_sha256: Some(sample_sha256("a")),
        wasm_gz_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
        observed_wasm_gz_file_sha256: Some(sample_sha256("a")),
        observed_wasm_gz_file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
        installed_module_hash: Some("module".to_string()),
        candid_path: Some("root.did".to_string()),
        candid_sha256: Some(sample_sha256("b")),
        raw_config_sha256: Some("raw".to_string()),
        canonical_embedded_config_sha256: Some(sample_sha256("c")),
        embedded_topology_sha256: Some("topology".to_string()),
        builder_version: Some("0.44.0".to_string()),
        rust_toolchain: Some("stable".to_string()),
        package_version: Some("0.44.0".to_string()),
    }
}

pub(super) fn sample_sha256(seed: &str) -> String {
    seed.repeat(64)
}

pub(super) fn sample_catalog_report() -> canic_host::fleet_catalog::FleetCatalogReportV1 {
    use canic_core::ids::{AppId, CanonicalNetworkId, FleetId};
    use canic_host::fleet_catalog::FleetCatalogEntryV1;

    let canonical_network_id = "01"
        .repeat(32)
        .parse::<CanonicalNetworkId>()
        .expect("canonical network ID");
    canic_host::fleet_catalog::FleetCatalogReportV1 {
        schema_version: 1,
        generated_at: "unix:54".to_string(),
        project_root: Some(".".to_string()),
        canonical_network_id,
        environment: "local".to_string(),
        entries: vec![FleetCatalogEntryV1 {
            canonical_network_id,
            fleet_id: FleetId::from_generated_bytes([2; 32]),
            fleet_name: "demo-local".parse().expect("Fleet name"),
            app: AppId::from("demo"),
            environment: "local".to_string(),
            deployed_at_unix_secs: 54,
            coordinator_principal: "aaaaa-aa".to_string(),
        }],
    }
}

pub(super) fn temp_json_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "canic-cli-{name}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ))
}

fn sample_deployment_inventory(identity: DeploymentIdentityV1) -> DeploymentInventoryV1 {
    DeploymentInventoryV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-23T00:00:00Z".to_string(),
        observed_identity: Some(identity),
        observed_root: None,
        local_config: LocalDeploymentConfigV1 {
            config_path: None,
            raw_config_sha256: None,
            canonical_embedded_config_sha256: None,
        },
        observed_canisters: vec![ObservedCanisterV1 {
            canister_id: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string()],
            module_hash: None,
            status: Some("running".to_string()),
            root_trust_anchor: Some("aaaaa-aa".to_string()),
            canonical_embedded_config_digest: None,
            role_assignment_source: Some("test".to_string()),
        }],
        observed_pool: Vec::new(),
        observed_artifacts: Vec::new(),
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    }
}

fn sample_deployment_diff(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
) -> DeploymentDiffV1 {
    DeploymentDiffV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_identity: plan.deployment_identity.clone(),
        observed_identity: inventory.observed_identity.clone(),
        artifact_diff: Vec::new(),
        controller_diff: Vec::new(),
        pool_diff: Vec::new(),
        embedded_config_diff: Vec::new(),
        module_hash_diff: Vec::new(),
        verifier_readiness_diff: Vec::new(),
        resume_safety: ResumeSafetyV1 {
            status: SafetyStatusV1::Safe,
            reasons: vec!["safe".to_string()],
        },
        hard_failures: Vec::new(),
        warnings: Vec::new(),
        resumable_phases: Vec::new(),
    }
}

fn sample_safety_report() -> SafetyReportV1 {
    SafetyReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: "safety-report-1".to_string(),
        diff_id: None,
        status: SafetyStatusV1::Safe,
        summary: "safe".to_string(),
        hard_failures: Vec::new(),
        warnings: Vec::new(),
        next_actions: Vec::new(),
    }
}
