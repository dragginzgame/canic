use super::*;

pub(in crate::deployment_truth::tests) fn sample_identity() -> DeploymentIdentityV1 {
    DeploymentIdentityV1 {
        deployment_name: "local-root".to_string(),
        environment: "local".to_string(),
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

pub(in crate::deployment_truth::tests) fn sample_role_artifact() -> RoleArtifactV1 {
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
pub(in crate::deployment_truth::tests) fn sample_sha256(seed: &str) -> String {
    seed.repeat(64)
}

pub(in crate::deployment_truth::tests) fn sample_plan() -> DeploymentPlanV1 {
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

pub(in crate::deployment_truth::tests) fn sample_matching_inventory() -> DeploymentInventoryV1 {
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
            role_assignment_source: Some(
                RoleAssignmentSourceV1::IcpCanisterStatus
                    .label()
                    .to_string(),
            ),
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

pub(in crate::deployment_truth::tests) fn sample_root_observation() -> DeploymentRootObservationV1 {
    DeploymentRootObservationV1 {
        deployment_name: "demo".to_string(),
        environment: "local".to_string(),
        fleet_template: "root".to_string(),
        root_principal: "aaaaa-aa".to_string(),
        observed_canister_id: "aaaaa-aa".to_string(),
        observation_source: DeploymentRootObservationSourceV1::IcpCanisterStatus,
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("running".to_string()),
        role_assignment_source: Some(
            RoleAssignmentSourceV1::IcpCanisterStatus
                .label()
                .to_string(),
        ),
    }
}

pub(in crate::deployment_truth::tests) fn sample_check(
    plan: DeploymentPlanV1,
    inventory: DeploymentInventoryV1,
) -> DeploymentCheckV1 {
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
pub(in crate::deployment_truth::tests) fn sample_unknown_unsafe_check() -> DeploymentCheckV1 {
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
        role_assignment_source: Some(
            RoleAssignmentSourceV1::IcpCanisterStatus
                .label()
                .to_string(),
        ),
    });

    sample_check(sample_plan(), inventory)
}
