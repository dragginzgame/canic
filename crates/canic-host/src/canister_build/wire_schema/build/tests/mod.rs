//! Native selection, source custody and the real dedicated producer build.

use super::*;
use crate::{canister_build::CanisterBuildProfile, test_support::temp_dir};
use canic_contracts::ids::BuildNetwork;
use sha2_host::Digest as _;
use std::fs;

fn context() -> WorkspaceBuildContext {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../integrations/blob-service/consumer")
        .canonicalize()
        .unwrap();
    WorkspaceBuildContext {
        role: "blob".into(),
        profile: CanisterBuildProfile::Fast,
        environment: "local".into(),
        build_network: BuildNetwork::Local,
        workspace_root: root.clone(),
        icp_root: root.clone(),
        config_path: root.join("canic.toml"),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    }
}

#[test]
fn native_recipe_preserves_runtime_selection_without_mutating_runtime_invocation() {
    let context = context();
    let options = CanisterArtifactBuildOptions {
        cargo_features: BTreeSet::from(["application-feature".into()]),
        default_features: false,
        sidecar_only_candid: true,
    };
    let runtime = canister_runtime_command(
        &context,
        &context.workspace_root.join("Cargo.toml"),
        context.profile,
        &options,
    );
    let mut native = runtime.clone();
    select_native_recipe(&mut native, "aarch64-apple-darwin".into());
    assert_eq!(runtime.target, "wasm32-unknown-unknown");
    assert_eq!(native.target, "aarch64-apple-darwin");
    assert_eq!(native.recipe, CargoRecipe::Example("wire_schema".into()));
    assert_eq!(native.selection.profile, runtime.selection.profile);
    assert_eq!(native.environment, runtime.environment);
    assert_eq!(native.selection.locked, runtime.selection.locked);
    assert!(!native.selection.features.default_features);
    assert_eq!(
        native.selection.features.features,
        BTreeSet::from(["application-feature".into(), "wire-schema".into()])
    );
    assert!(!runtime.selection.features.features.contains("wire-schema"));
}

#[test]
fn compiler_host_requires_one_exact_native_target() {
    assert_eq!(
        parse_compiler_host("rustc 1.99.1\nhost: aarch64-apple-darwin\n").unwrap(),
        "aarch64-apple-darwin"
    );
    for invalid in [
        "",
        "host: \n",
        "host: wasm32-unknown-unknown\n",
        "host: a b\n",
        "host: a\nhost: b\n",
    ] {
        assert_eq!(
            parse_compiler_host(invalid),
            Err(WireSchemaBuildError::HostTarget)
        );
    }
}

#[test]
#[ignore = "requires CANIC_WIRE_FINAL_WASM and optional CANIC_WIRE_CONSUMER fixture selection"]
fn configured_native_producer_builds_and_retains_schema() {
    let context = producer_fixture_context();
    let wasm = std::env::var_os("CANIC_WIRE_FINAL_WASM").expect("managed final Wasm");
    let root = temp_dir("configured-wire-build");
    fs::create_dir_all(&root).unwrap();
    let output = root.join("wire-schema.json");
    let native = build_native_wire_contract(
        &context,
        resolve_recipe(&context, &CanisterArtifactBuildOptions::default()).unwrap(),
    )
    .unwrap();
    let result = build_workspace_wire_schema(
        &context,
        &CanisterArtifactBuildOptions::default(),
        Path::new(&wasm),
        &output,
    )
    .unwrap();
    let bytes = fs::read(&output).unwrap();
    assert_eq!(native, bytes);
    let schema: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(!schema["methods"].as_array().unwrap().is_empty());
    let expected: [u8; 32] = sha2_host::Sha256::digest(&bytes).into();
    assert_eq!(result.bindings.schema_sha256, expected);
    let bindings = fs::read(output.with_extension("ts")).unwrap();
    fs::remove_file(&output).unwrap();
    fs::write(output.with_extension("ts"), b"interrupted output").unwrap();
    let recovered = build_workspace_wire_schema(
        &context,
        &CanisterArtifactBuildOptions::default(),
        Path::new(&wasm),
        &output,
    )
    .unwrap();
    assert!(recovered.acquisition.is_reused());
    assert_eq!(fs::read(&output).unwrap(), bytes);
    assert_eq!(fs::read(output.with_extension("ts")).unwrap(), bindings);
    assert_eq!(recovered.bindings, result.bindings);
    drop((result, recovered));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn generic_native_snapshot_refuses_source_change_before_publication() {
    use ic_build::artifacts::{
        ArtifactCacheError, ArtifactCachePreparation, ArtifactCacheSpec, prepare_artifact_cache,
    };
    use ic_build::cargo::selection::CargoSelection;
    let root = temp_dir("native-schema-inputs");
    fs::create_dir_all(root.join("examples/wire_schema")).unwrap();
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"native-input-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[workspace]\n").unwrap();
    fs::write(
        root.join("Cargo.lock"),
        "version = 4\n[[package]]\nname = \"native-input-fixture\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let source = root.join("examples/wire_schema/main.rs");
    fs::write(&source, "fn main() {}\n").unwrap();
    let context = context();
    let invocation = canister_runtime_command(
        &context,
        &root.join("Cargo.toml"),
        context.profile,
        &CanisterArtifactBuildOptions::default(),
    );
    let target = compiler_host(&invocation).unwrap();
    let mut environment = invocation.environment;
    environment.remove(OsStr::new("CARGO_TARGET_DIR"));
    let build = WasmBuildSpec::new(
        &root,
        &root.join("target"),
        &["native-input-fixture"],
        environment,
        crate::canister_build::CARGO_CONFIGURATION_LIMITS,
    )
    .with_example("wire_schema")
    .with_target(&target)
    .with_cargo_selection(CargoSelection {
        locked: true,
        offline: true,
        ..Default::default()
    });
    let inputs = resolve_cargo_build_inputs(&build).unwrap();
    require_current_inputs(&inputs, &build, "test").unwrap();
    let output = root.join("target/schema.json");
    let cache = ArtifactCacheSpec::new(
        &root.join("target/cache"),
        "native-schema-test",
        "native-schema-test.v1",
    )
    .with_cargo_build_inputs("producer", &build, &inputs)
    .with_output("schema", &output);
    let ArtifactCachePreparation::Build(transaction) = prepare_artifact_cache(&cache).unwrap()
    else {
        panic!("fresh cache");
    };
    fs::write(transaction.output_path("schema").unwrap(), b"candidate").unwrap();
    fs::write(&source, "fn main() { println!(\"changed\"); }\n").unwrap();
    let error = require_current_inputs(&inputs, &build, "test").unwrap_err();
    assert!(matches!(
        error.downcast_ref::<WireSchemaBuildError>(),
        Some(WireSchemaBuildError::ChangedInputs {
            phase: "test",
            source_changed: true,
            ..
        })
    ));
    assert!(matches!(
        transaction.commit(),
        Err(ArtifactCacheError::CargoBuildInputsChanged { .. })
    ));
    assert!(!output.exists());
    fs::remove_dir_all(root).unwrap();
}

fn producer_fixture_context() -> WorkspaceBuildContext {
    let selected = std::env::var("CANIC_WIRE_CONSUMER").unwrap_or_else(|_| "dedicated".into());
    let mut context = context();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    match selected.as_str() {
        "dedicated" => return context,
        "embedded" => {
            context.workspace_root = root.join("integrations/blob-service/embedded-consumer");
            context.role = "backend".into();
            context.config_path = context.workspace_root.join("canic.toml");
        }
        "root" | "wasm_store" | "fleet_coordinator" => {
            context.workspace_root = root;
            context.role = selected;
            context.config_path = context.workspace_root.join("apps/test/canic.toml");
        }
        _ => panic!("unknown schema producer fixture"),
    }
    context.icp_root.clone_from(&context.workspace_root);
    context
}

#[test]
fn explicit_producer_recipe_requires_one_matching_locked_package() {
    let context = context();
    let configured = resolve_recipe(&context, &CanisterArtifactBuildOptions::default()).unwrap();
    validate_recipe(&configured).unwrap();
    let manifest = context.workspace_root.join("Cargo.toml");
    let mut invocation = canister_runtime_command(
        &context,
        &manifest,
        context.profile,
        &CanisterArtifactBuildOptions::default(),
    );
    invocation.packages = vec!["selected-probe".into()];
    let recipe = || NativeSchemaBuildRecipe {
        manifest: manifest.clone(),
        workspace: context.workspace_root.clone(),
        package: "selected-probe".into(),
        invocation: invocation.clone(),
    };
    validate_recipe(&recipe()).unwrap();
    for axis in 0..8 {
        let mut changed = recipe();
        match axis {
            0 => changed.package.clear(),
            1 => changed.invocation.working_directory = context.workspace_root.join("other"),
            2 => changed.invocation.manifest_path = None,
            3 => changed.invocation.packages = vec!["another-probe".into()],
            4 => {
                changed.invocation.packages = vec![changed.package.clone(), "another-probe".into()];
            }
            5 => changed.invocation.selection.locked = false,
            6 => changed.invocation.packages.clear(),
            7 => changed
                .invocation
                .configurations
                .push("build.target-dir=\"other\"".into()),
            _ => unreachable!(),
        }
        assert_eq!(
            validate_recipe(&changed),
            Err(WireSchemaBuildError::RecipeSelection)
        );
    }
}
