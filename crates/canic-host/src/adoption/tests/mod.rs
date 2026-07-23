use super::*;
use crate::deployment_truth::{
    ArtifactDigestSourceV1, ArtifactSourceV1, CanisterControlClassV1, DeploymentInventoryV1,
    DeploymentObservationGapV1, DeploymentRootObservationSourceV1, DeploymentRootObservationV1,
    LocalDeploymentConfigV1, ObservationStatusV1, ObservedArtifactV1, ObservedCanisterV1,
    ObservedPoolCanisterV1, RoleArtifactManifestV1, RoleArtifactV1, VerifierReadinessObservationV1,
};

mod authority;
mod configured_roles;
mod evidence;
mod profiles;
mod serialization;

const CONFIG: &str = r#"
controllers = []
[services.fleet]
roles = []

[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.api]
kind = "canister"
package = "api"

[roles.store]
kind = "canister"
package = "store"

[subnets.default.canisters.root]
kind = "root"

[subnets.default.canisters.api]
kind = "service"
"#;

const BROWNFIELD_CONFIG: &str = r#"
controllers = []
[services.fleet]
roles = []

[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[subnets.default.canisters.root]
kind = "root"
"#;

const STANDALONE_CONFIG: &str = r#"
controllers = []
[services.fleet]
roles = []

[app]
name = "demo"

[roles.worker]
kind = "canister"
package = "worker"
"#;

const LEAF_ONLY_CONFIG: &str = r#"
controllers = []
[services.fleet]
roles = []

[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[subnets.default.canisters.app]
kind = "service"

[subnets.default.canisters.root]
kind = "root"
"#;

fn report(
    config_source: &str,
    inventory: Option<&DeploymentInventoryV1>,
    package_metadata: Vec<AdoptionPackageMetadataV1>,
) -> AdoptionReportV1 {
    report_with_profile(
        AdoptionProfileV1::Brownfield,
        config_source,
        inventory,
        package_metadata,
    )
}

fn report_with_profile(
    profile: AdoptionProfileV1,
    config_source: &str,
    inventory: Option<&DeploymentInventoryV1>,
    package_metadata: Vec<AdoptionPackageMetadataV1>,
) -> AdoptionReportV1 {
    adoption_report_from_config_source(AdoptionReportRequest {
        report_id: "adoption-1",
        generated_at: "2026-05-30T00:00:00Z",
        profile,
        config_source,
        inventory,
        artifact_manifest: None,
        package_metadata,
    })
    .expect("adoption report")
}

fn matching_metadata() -> Vec<AdoptionPackageMetadataV1> {
    ["root", "api", "store"]
        .into_iter()
        .map(|package| AdoptionPackageMetadataV1 {
            package: package.to_string(),
            app: Some("demo".to_string()),
            role: Some(package.to_string()),
        })
        .collect()
}

fn external_api_artifact_manifest() -> RoleArtifactManifestV1 {
    RoleArtifactManifestV1 {
        schema_version: 1,
        manifest_id: "external-manifest-1".to_string(),
        environment: "local".to_string(),
        artifact_root: None,
        role_artifacts: vec![external_api_role_artifact()],
        unresolved_artifacts: Vec::new(),
    }
}

fn external_api_role_artifact() -> RoleArtifactV1 {
    RoleArtifactV1 {
        role: "api".to_string(),
        source: ArtifactSourceV1::External,
        build_profile: "external".to_string(),
        wasm_path: Some("external/api.wasm".to_string()),
        wasm_gz_path: Some("external/api.wasm.gz".to_string()),
        wasm_gz_size_bytes: Some(42),
        wasm_sha256: Some("api-wasm-sha".to_string()),
        wasm_gz_sha256: Some("api-wasm-gz-sha".to_string()),
        wasm_gz_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
        observed_wasm_gz_file_sha256: Some("api-file-sha".to_string()),
        observed_wasm_gz_file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
        installed_module_hash: Some("api-installed-module".to_string()),
        candid_path: None,
        candid_sha256: None,
        raw_config_sha256: None,
        canonical_embedded_config_sha256: None,
        embedded_topology_sha256: None,
        builder_version: None,
        rust_toolchain: None,
        package_version: None,
    }
}

fn observed_external_api_artifact() -> ObservedArtifactV1 {
    ObservedArtifactV1 {
        role: "api".to_string(),
        artifact_path: "external/api.wasm.gz".to_string(),
        file_sha256: Some("api-file-sha".to_string()),
        file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
        payload_sha256: Some("api-payload-sha".to_string()),
        payload_size_bytes: Some(42),
        source: ArtifactSourceV1::External,
    }
}

fn role<'a>(report: &'a AdoptionReportV1, role: &str) -> &'a AdoptionRoleFindingV1 {
    report
        .role_findings
        .iter()
        .find(|finding| finding.role == role)
        .expect("role finding")
}

fn inventory(observed_canisters: Vec<ObservedCanisterV1>) -> DeploymentInventoryV1 {
    DeploymentInventoryV1 {
        schema_version: 1,
        inventory_id: "inventory-1".to_string(),
        observed_at: "2026-05-30T00:00:00Z".to_string(),
        observed_identity: None,
        observed_root: Some(DeploymentRootObservationV1 {
            deployment_name: "demo-dev".to_string(),
            environment: "local".to_string(),
            fleet_template: "demo".to_string(),
            root_principal: "aaaaa-aa".to_string(),
            observed_canister_id: "aaaaa-aa".to_string(),
            observation_source: DeploymentRootObservationSourceV1::LocalDeploymentState,
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string()],
            module_hash: None,
            status: Some("running".to_string()),
            role_assignment_source: Some("local-state".to_string()),
        }),
        local_config: LocalDeploymentConfigV1 {
            config_path: Some("apps/demo/canic.toml".to_string()),
            raw_config_sha256: None,
            canonical_embedded_config_sha256: None,
        },
        observed_canisters,
        observed_pool: Vec::new(),
        observed_artifacts: vec![ObservedArtifactV1 {
            role: "external_app".to_string(),
            artifact_path: "observed:external_app".to_string(),
            file_sha256: None,
            file_sha256_source: None,
            payload_sha256: None,
            payload_size_bytes: None,
            source: ArtifactSourceV1::External,
        }],
        observed_verifier_readiness: VerifierReadinessObservationV1 {
            status: ObservationStatusV1::NotObserved,
            role_epochs: Vec::new(),
        },
        unresolved_observations: Vec::new(),
    }
}

fn observed_canister(
    canister_id: &str,
    role: Option<&str>,
    control_class: CanisterControlClassV1,
    module_hash: Option<&str>,
) -> ObservedCanisterV1 {
    ObservedCanisterV1 {
        canister_id: canister_id.to_string(),
        role: role.map(str::to_string),
        control_class,
        controllers: vec!["controller".to_string()],
        module_hash: module_hash.map(str::to_string),
        status: Some("running".to_string()),
        root_trust_anchor: Some("root".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: role.map(|_| "explicit-test-evidence".to_string()),
    }
}
