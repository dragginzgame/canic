//! Build the local balance-control audit Root directly for a disposable test release.
//! Production Root generation and release artifacts are unaffected.

use crate::pic::CanicWasmBuildProfile;
mod package;
use canic_contracts::ids::{BuildNetwork, CanisterRole};
use canic_host::{
    canister_build::{CanisterArtifactBuildOutput, CanisterBuildProfile, WorkspaceBuildContext},
    release_set::AppConfigSnapshot,
    role_contract::{PackageValidationMode, RolePackageValidation, validate_declared_role_package},
};
use std::{path::Path, process::Command};

pub fn uses_audit_root(config: &Path) -> bool {
    config.ends_with("canisters/audit/root_probe/native-child-recovery.toml")
        || config.ends_with("canisters/audit/root_probe/retained-estate.toml")
}

pub fn build_audit_root(context: &WorkspaceBuildContext) -> CanisterArtifactBuildOutput {
    assert_eq!(context.profile, CanisterBuildProfile::Fast);
    assert_eq!(context.build_network, BuildNetwork::Local);
    let target = context
        .workspace_root
        .join("target/pic-wasm/native-child-recovery");
    let config = context.config_path.to_str().unwrap();
    let release = context.release_build_id.unwrap().to_string();
    canic_host::canister_build::prepare_workspace_infrastructure_packages(context).unwrap();
    let snapshot = AppConfigSnapshot::load(&context.config_path).unwrap();
    let evidence = admitted_root_package(context, &snapshot);
    let package = package::prepare(context, &evidence);
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
        (canic_contracts::ids::RELEASE_BUILD_ID_ENV, release.as_str()),
    ];
    let mut declaration = base_env.to_vec();
    declaration.push((canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV, "1"));
    let declaration_wasms = build_audit_wasm(context, &package, &target, &declaration, &features);
    let [raw] = declaration_wasms.artifacts() else {
        panic!("one audit Root artifact")
    };
    let candid = extract_declaration(raw);
    let wire_schema = canic_host::canister_build::build_native_wire_contract(
        context,
        audit_schema_recipe(&package, &base_env, &features),
    )
    .expect("produce audit Root native contract");
    let profile = canic_core::role_contract::derive_protocol_profile_hashes(
        &evidence.canic_version,
        &CanisterRole::ROOT,
        &contract.capabilities,
        &wire_schema,
    );
    let digest = profile.protocol_profile_digest.to_string();
    let mut runtime = base_env.to_vec();
    runtime.push((
        canic_core::role_contract::PROTOCOL_PROFILE_DIGEST_ENV,
        digest.as_str(),
    ));
    let runtime_wasms = build_audit_wasm(context, &package, &target, &runtime, &features);
    let role_path = |extension| {
        context
            .icp_root
            .join(".canic/release-builds")
            .join(context.release_build_id.unwrap().to_string())
            .join("artifacts/root")
            .join(format!("root.{extension}"))
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
        candid_sha256: canic_core::cdk::utils::hash::sha256_bytes(&candid)
            .try_into()
            .unwrap(),
        wire_schema_sha256: profile.wire_schema_sha256,
        protocol_profile_digest: profile.protocol_profile_digest,
        transforms: Vec::new(),
    };
    let [runtime_wasm] = runtime_wasms.artifacts() else {
        panic!("one audit Root artifact")
    };
    finalize_audit_root(&output, runtime_wasm, &candid);
    build_audit_schema(context, &package, &output, &runtime, &features);
    output
}

fn extract_declaration(wasm: &Path) -> Vec<u8> {
    let output = Command::new("candid-extractor").arg(wasm).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
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
    canic_host::canister_build::validate_wasm_endpoints(&output.wasm_path, &output.did_path)
        .unwrap();
    canic_host::canister_build::write_gzip_artifact(&output.wasm_path, &output.wasm_gz_path)
        .unwrap();
}

/// Reuse the exact audit runtime selection, not the canonical Root package recipe.
fn build_audit_schema(
    context: &WorkspaceBuildContext,
    package: &Path,
    output: &CanisterArtifactBuildOutput,
    runtime: &[(&str, &str)],
    features: &[String],
) {
    let retained = canic_host::canister_build::build_native_wire_schema(
        context,
        audit_schema_recipe(package, runtime, features),
        &output.wasm_path,
        &output.wasm_path.with_extension("wire.json"),
    )
    .expect("build and admit actual audit Root native schema");
    assert_eq!(retained.bindings.schema_sha256, output.wire_schema_sha256);
}

fn audit_schema_recipe(
    package: &Path,
    runtime: &[(&str, &str)],
    features: &[String],
) -> canic_host::canister_build::NativeSchemaBuildRecipe {
    use canic_host::canister_build::NativeSchemaBuildRecipe;
    use ic_build::cargo::execution::{CargoInvocation, CargoRecipe};
    let manifest = package.join("Cargo.toml");
    let selection = crate::pic::artifacts::fixture_cargo_selection(
        CanicWasmBuildProfile::Fast,
        runtime,
        features,
    );
    let mut environment =
        crate::workspace::wasm_build_environment().collect::<std::collections::BTreeMap<_, _>>();
    for (key, value) in [
        ("CARGO_INCREMENTAL", "0"),
        ("ICP_ENVIRONMENT", "local"),
        (
            canic_core::role_contract::CANONICAL_BUILD_MARKER_ENV,
            canic_core::role_contract::CANONICAL_BUILD_MARKER_VALUE,
        ),
    ]
    .into_iter()
    .chain(runtime.iter().copied())
    {
        environment.insert(key.into(), value.into());
    }
    environment
        .entry("RUSTC".into())
        .or_insert_with(|| "rustc".into());
    let invocation = CargoInvocation {
        cargo_program: std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()),
        working_directory: package.to_path_buf(),
        environment,
        target: "wasm32-unknown-unknown".into(),
        selection,
        packages: vec!["root_probe".to_owned()],
        manifest_path: Some(manifest.clone()),
        recipe: CargoRecipe::Library,
        configurations: Vec::new(),
        render_diagnostics: true,
        keep_going: true,
    };
    NativeSchemaBuildRecipe {
        manifest,
        workspace: package.to_path_buf(),
        package: "root_probe".to_owned(),
        invocation,
    }
}

fn build_audit_wasm(
    context: &WorkspaceBuildContext,
    package: &Path,
    target: &Path,
    environment: &[(&str, &str)],
    features: &[String],
) -> ic_build::artifacts::WasmBuildRecord {
    let recipe = audit_schema_recipe(package, environment, features);
    let spec = ic_build::artifacts::WasmBuildSpec::new(
        package,
        target,
        &["root_probe"],
        recipe.invocation.environment,
        canic_host::canister_build::CARGO_CONFIGURATION_LIMITS,
    )
    .with_cargo_selection(recipe.invocation.selection)
    .with_additional_inputs([&context.config_path])
    .with_shared_incremental_target(
        context
            .workspace_root
            .join("target/test-artifacts/cargo-audit-root"),
    );
    ic_build::artifacts::build_wasm_canisters_cached(
        &spec,
        ic_host_process::child::OwnedChild::spawn_direct,
    )
    .expect("build selected App audit Root")
    .record()
    .clone()
}
