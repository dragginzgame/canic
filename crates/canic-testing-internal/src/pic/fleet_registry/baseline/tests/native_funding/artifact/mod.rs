//! Bind the existing local balance-control audit Root into a disposable test release.
//! Production Root generation and release artifacts are unaffected.

use super::super::*;
use crate::pic::artifacts::build_internal_test_wasm_canisters_with_features;

pub fn uses_audit_root(config: &Path) -> bool {
    config.ends_with("canisters/audit/root_probe/native-child-recovery.toml")
        || config.ends_with("canisters/audit/root_probe/retained-estate.toml")
}

pub fn bind_audit_root(context: &WorkspaceBuildContext, output: &mut CanisterArtifactBuildOutput) {
    assert_eq!(context.build_network, BuildNetwork::Local);
    let target = context
        .workspace_root
        .join("target/pic-wasm/native-child-recovery");
    let config = context.config_path.to_str().unwrap();
    let release = context.release_build_id.unwrap().to_string();
    let snapshot = AppConfigSnapshot::load(&context.config_path).unwrap();
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
        &output.protocol_release_identity,
        &output.protocol_role,
        &output.protocol_capabilities,
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
    std::fs::write(&output.did_path, &candid.stdout).unwrap();
    let shrink = Command::new("ic-wasm")
        .arg(runtime_wasms.path("root_probe"))
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
    output.package_name = "root_probe".into();
    output.candid_sha256 = profile.candid_sha256;
    output.protocol_profile_digest = profile.protocol_profile_digest;
    // Canonical-Root transform measurements do not describe this audit artifact.
    output.transforms.clear();
}
