//! Build the local balance-control audit Root directly for a disposable test release.
//! Production Root generation and release artifacts are unaffected.

use super::literal_zero_role_artifact_path;
use crate::pic::CanicWasmBuildProfile;
use crate::pic::artifacts::build_internal_test_wasm_canisters_with_features;
use canic_core::ids::{BuildNetwork, CanisterRole};
use canic_host::{
    canister_build::{CanisterArtifactBuildOutput, CanisterBuildProfile, WorkspaceBuildContext},
    release_set::AppConfigSnapshot,
    role_contract::{PackageValidationMode, RolePackageValidation, validate_declared_role_package},
};
use flate2::{Compression, write::GzEncoder};
use std::{io::Write, path::Path, process::Command};

pub fn uses_audit_root(config: &Path) -> bool {
    config.ends_with("canisters/audit/root_probe/native-child-recovery.toml")
        || config.ends_with("canisters/audit/root_probe/retained-estate.toml")
}

pub fn build_audit_root(context: &WorkspaceBuildContext) -> CanisterArtifactBuildOutput {
    assert!(uses_audit_root(&context.config_path));
    assert_eq!(context.profile, CanisterBuildProfile::Fast);
    assert_eq!(context.build_network, BuildNetwork::Local);
    let target = context
        .workspace_root
        .join("target/pic-wasm/native-child-recovery");
    let config = context.config_path.to_str().unwrap();
    let release = context.release_build_id.unwrap().to_string();
    let snapshot = AppConfigSnapshot::load(&context.config_path).unwrap();
    let evidence = admitted_root_package(context, &snapshot);
    let canic_core::role_contract::RoleContractResolution::Resolved { contract } =
        canic_host::role_contract::resolve_declared_role_package_contract(
            snapshot.model(),
            &evidence,
        )
    else {
        panic!("audit Root must retain the resolved role contract");
    };
    let features = canic_core::role_contract::required_features_for_role(
        snapshot.model(),
        &CanisterRole::ROOT,
    )
    .unwrap()
    .into_iter()
    .map(|required| format!("canic/{}", required.feature.cargo_name()))
    .collect::<Vec<_>>();
    let base_env = [
        (
            canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
            config,
        ),
        (canic_core::ids::RELEASE_BUILD_ID_ENV, release.as_str()),
    ];
    let mut declaration = base_env.to_vec();
    declaration.push((canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV, "1"));
    let declaration_wasms = build_internal_test_wasm_canisters_with_features(
        &context.workspace_root,
        &target,
        &["root_probe"],
        CanicWasmBuildProfile::Fast,
        &declaration,
        &features,
    );
    let raw = declaration_wasms.path("root_probe");
    let candid = Command::new("candid-extractor").arg(raw).output().unwrap();
    assert!(
        candid.status.success(),
        "{}",
        String::from_utf8_lossy(&candid.stderr)
    );
    let profile = canic_core::role_contract::derive_protocol_profile_hashes(
        &evidence.canic_version,
        &CanisterRole::ROOT,
        &contract.capabilities,
        &candid.stdout,
    );
    let digest = profile.protocol_profile_digest.to_string();
    let mut runtime = base_env.to_vec();
    runtime.push((
        canic_core::role_contract::PROTOCOL_PROFILE_DIGEST_ENV,
        digest.as_str(),
    ));
    let runtime_wasms = build_internal_test_wasm_canisters_with_features(
        &context.workspace_root,
        &target,
        &["root_probe"],
        CanicWasmBuildProfile::Fast,
        &runtime,
        &features,
    );
    let role_path = |extension| {
        literal_zero_role_artifact_path(
            &context.icp_root,
            context.release_build_id.unwrap(),
            "root",
            extension,
        )
    };
    let wasm_path = role_path("wasm");
    let output = CanisterArtifactBuildOutput {
        package_name: "root_probe".into(),
        package_version: evidence.role_package_version,
        protocol_release_identity: evidence.canic_version,
        protocol_role: CanisterRole::ROOT,
        protocol_capabilities: contract.capabilities,
        artifact_root: wasm_path.parent().unwrap().to_path_buf(),
        wasm_path,
        wasm_gz_path: role_path("wasm.gz"),
        did_path: role_path("did"),
        candid_sha256: profile.candid_sha256,
        protocol_profile_digest: profile.protocol_profile_digest,
        transforms: Vec::new(),
    };
    finalize_audit_root(&output, runtime_wasms.path("root_probe"), &candid.stdout);
    output
}

fn admitted_root_package(
    context: &WorkspaceBuildContext,
    snapshot: &AppConfigSnapshot,
) -> canic_host::role_contract::RoleCargoGraphEvidence {
    let RolePackageValidation::Supported(evidence) = validate_declared_role_package(
        &context.config_path,
        snapshot.model(),
        &CanisterRole::ROOT,
        PackageValidationMode::Build,
        &canic_host::role_contract::CargoFeatureSelection::default(),
    ) else {
        panic!("audit Root must retain admitted canonical role authority");
    };
    evidence
}

fn finalize_audit_root(output: &CanisterArtifactBuildOutput, runtime: &Path, candid: &[u8]) {
    std::fs::create_dir_all(&output.artifact_root).unwrap();
    std::fs::write(&output.did_path, candid).unwrap();
    let shrink = Command::new("ic-wasm")
        .arg(runtime)
        .arg("-o")
        .arg(&output.wasm_path)
        .arg("shrink")
        .output()
        .unwrap();
    assert!(
        shrink.status.success(),
        "{}",
        String::from_utf8_lossy(&shrink.stderr)
    );
    let metadata = Command::new("ic-wasm")
        .arg(&output.wasm_path)
        .arg("-o")
        .arg(&output.wasm_path)
        .args(["metadata", "candid:service", "-f"])
        .arg(&output.did_path)
        .args(["-v", "public"])
        .output()
        .unwrap();
    assert!(
        metadata.status.success(),
        "{}",
        String::from_utf8_lossy(&metadata.stderr)
    );
    canic_host::canister_build::validate_wasm_candid_endpoints(&output.wasm_path, &output.did_path)
        .unwrap();
    std::fs::write(
        &output.wasm_gz_path,
        gzip(&std::fs::read(&output.wasm_path).unwrap()),
    )
    .unwrap();
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).expect("gzip fixture Wasm");
    encoder.finish().expect("finish fixture Wasm gzip")
}
