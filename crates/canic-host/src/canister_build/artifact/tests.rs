//! Focused checks for canister artifact artifact behavior.

use super::*;
use canic_core::ids::BuildNetwork;

#[test]
fn generated_infrastructure_inputs_are_stable_before_compilation() {
    let directory = crate::test_support::temp_dir("infrastructure-input-preparation");
    let package = directory.join("consumer");
    fs::create_dir_all(package.join("src")).unwrap();
    fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"fixture-consumer\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[workspace]\n",
    )
    .unwrap();
    fs::write(package.join("src/lib.rs"), "").unwrap();
    let context = WorkspaceBuildContext {
        role: "root".into(),
        profile: CanisterBuildProfile::Fast,
        environment: "local".into(),
        build_network: canic_core::ids::BuildNetwork::Local,
        workspace_root: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        icp_root: directory.clone(),
        config_path: package.join("canic.toml"),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };
    let spec = ic_testkit::artifacts::WasmBuildSpec::new(
        &package,
        &directory.join("target"),
        &["fixture-consumer"],
        "release",
    )
    .with_cargo_profile_args(["--release"]);
    let cold = ic_testkit::artifacts::resolve_cargo_build_inputs(&spec).unwrap();
    prepare_workspace_infrastructure_packages(&context).unwrap();
    assert!(
        !cold.is_content_current().unwrap(),
        "late package preparation changes an enclosing package's frozen inputs"
    );
    let prepared = ic_testkit::artifacts::resolve_cargo_build_inputs(&spec).unwrap();
    prepare_workspace_infrastructure_packages(&context).unwrap();
    assert!(prepared.is_content_current().unwrap());
    assert!(prepared.is_current(&spec).unwrap());
    for name in ["canic-fleet-coordinator", "canic-fleet-wasm-store"] {
        let root = crate::fleet_package::manifest_path(&context.config_path, name)
            .parent()
            .unwrap()
            .to_path_buf();
        assert!(root.join("Cargo.lock").is_file());
        assert!(root.join("src/lib.rs").is_file());
    }
    assert!(
        !directory.join("target").exists(),
        "preparation must not compile"
    );
    fs::write(
        crate::fleet_package::manifest_path(&context.config_path, "canic-fleet-wasm-store")
            .parent()
            .unwrap()
            .join("src/lib.rs"),
        "changed source",
    )
    .unwrap();
    assert!(
        !prepared.is_content_current().unwrap(),
        "preparing early must not exclude later source changes"
    );
    fs::remove_dir_all(directory).unwrap();
}
#[test]
fn configured_specs_group_into_one_cargo_command_per_workspace() {
    let specs = [
        build_spec("root", "canister-root", "/workspace"),
        build_spec("hub", "canister-hub", "/workspace"),
        build_spec("remote", "canister-remote", "/remote"),
    ];

    let groups = group_build_specs_by_workspace(&specs);

    assert_eq!(groups.len(), 2);
    assert_eq!(groups[Path::new("/workspace")].len(), 2);
    assert_eq!(groups[Path::new("/remote")].len(), 1);
}

#[test]
fn configured_batch_command_selects_every_group_package_once() {
    let context = build_context();
    let specs = [
        build_spec("root", "canister-root", "/workspace"),
        build_spec("hub", "canister-hub", "/workspace"),
    ];
    let spec_refs = specs.iter().collect::<Vec<_>>();

    let command = canister_cargo_batch_command(
        &context,
        Path::new("/workspace"),
        &spec_refs,
        CanisterBuildProfile::Debug,
    );
    let args = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let environment = command.get_envs().collect::<BTreeMap<_, _>>();

    assert_eq!(
        args,
        [
            "build",
            "--locked",
            "--keep-going",
            "--message-format=json-render-diagnostics",
            "--manifest-path",
            "/workspace/Cargo.toml",
            "--target",
            WASM_TARGET,
            "--package",
            "canister-root",
            "--package",
            "canister-hub",
        ]
    );
    assert_eq!(
        environment.get(std::ffi::OsStr::new(
            canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV
        )),
        Some(&Some(std::ffi::OsStr::new("1")))
    );
}

#[test]
fn profile_candid_pass_is_explicit_for_nonlocal_binding_derivation() {
    let context = build_context();
    let options = CanisterArtifactBuildOptions::default();
    let command = canister_profile_candid_command(
        &context,
        Path::new("/workspace/app/Cargo.toml"),
        CanisterBuildProfile::Fast,
        &options,
    );
    let args = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let environment = command.get_envs().collect::<BTreeMap<_, _>>();

    assert_eq!(args.first().map(String::as_str), Some("build"));
    assert_eq!(
        environment.get(std::ffi::OsStr::new(
            canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV
        )),
        Some(&Some(std::ffi::OsStr::new("1")))
    );
}

#[test]
fn declaration_and_runtime_passes_share_exact_cargo_features() {
    let context = build_context();
    let options = CanisterArtifactBuildOptions {
        cargo_features: ["qualification", "standalone-local"]
            .map(str::to_string)
            .into_iter()
            .collect(),
        default_features: false,
        sidecar_only_candid: true,
    };
    let declaration = canister_profile_candid_command(
        &context,
        Path::new("/workspace/app/Cargo.toml"),
        CanisterBuildProfile::Fast,
        &options,
    );
    let runtime = canister_runtime_command(
        &context,
        Path::new("/workspace/app/Cargo.toml"),
        CanisterBuildProfile::Fast,
        &options,
    );
    let declaration_args = declaration
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let runtime_args = runtime
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    for args in [&declaration_args, &runtime_args] {
        assert!(args.contains(&"--no-default-features".to_string()));
        assert!(
            args.windows(2)
                .any(|args| { args == ["--features", "qualification,standalone-local"] })
        );
    }
    assert_eq!(
        declaration
            .get_envs()
            .find(|(key, _)| {
                *key == std::ffi::OsStr::new(canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV)
            })
            .and_then(|(_, value)| value),
        Some(std::ffi::OsStr::new("1"))
    );
    assert_eq!(
        runtime.get_envs().find(|(key, _)| {
            *key == std::ffi::OsStr::new(canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV)
        }),
        Some((
            std::ffi::OsStr::new(canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV,),
            None,
        ))
    );
}

#[test]
fn configured_specs_bind_canonical_root_and_application_workspace() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config_path = workspace_root.join("apps/demo/canic.toml");
    let config = AppConfigSnapshot::load(&config_path).expect("load demo App config");
    let context = WorkspaceBuildContext {
        role: "root".to_string(),
        profile: CanisterBuildProfile::Fast,
        environment: "local".to_string(),
        build_network: BuildNetwork::Local,
        workspace_root: workspace_root.clone(),
        icp_root: workspace_root.clone(),
        config_path,
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };
    let roles = ["root", "app", "user_hub", "user_shard"].map(str::to_string);

    let specs = resolve_canister_artifact_build_specs(&context, config.model(), &roles)
        .expect("resolve configured demo build specs");

    assert_eq!(specs.len(), roles.len());
    assert!(
        specs
            .iter()
            .all(|spec| spec.package_version == env!("CARGO_PKG_VERSION"))
    );
    for spec in &specs {
        let expected_workspace = if spec.role == "root" {
            assert_eq!(spec.package_name, crate::canonical_root::PACKAGE);
            crate::canonical_root::manifest_path(&context.config_path)
                .parent()
                .unwrap()
                .to_path_buf()
        } else {
            workspace_root.clone()
        };
        assert_eq!(
            spec.cargo_workspace_root.canonicalize().unwrap(),
            expected_workspace.canonicalize().unwrap()
        );
    }
    let app = specs
        .iter()
        .find(|spec| spec.role == "app")
        .expect("app spec");
    let user_hub = specs
        .iter()
        .find(|spec| spec.role == "user_hub")
        .expect("user_hub spec");
    assert!(
        !app.capabilities
            .contains(&canic_core::role_contract::RoleCapabilityKey::AutomaticTopup)
    );
    assert!(
        user_hub
            .capabilities
            .contains(&canic_core::role_contract::RoleCapabilityKey::AutomaticTopup)
    );
}

#[test]
fn configured_spec_resolution_reports_every_invalid_role() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config_path = workspace_root.join("apps/demo/canic.toml");
    let config = AppConfigSnapshot::load(&config_path).expect("load demo App config");
    let context = WorkspaceBuildContext {
        role: "root".to_string(),
        profile: CanisterBuildProfile::Fast,
        environment: "local".to_string(),
        build_network: BuildNetwork::Local,
        workspace_root: workspace_root.clone(),
        icp_root: workspace_root,
        config_path,
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };
    let roles = ["missing-first", "missing-second"].map(str::to_string);

    let error = resolve_canister_artifact_build_specs(&context, config.model(), &roles)
        .expect_err("both invalid configured roles must fail");
    let failures = error
        .downcast_ref::<ConfiguredBuildSpecFailures>()
        .expect("typed configured build failure");
    let failed_roles = failures
        .0
        .iter()
        .map(|failure| failure.role.as_str())
        .collect::<Vec<_>>();

    assert_eq!(failed_roles, ["missing-first", "missing-second"]);

    let builder =
        CanisterArtifactBuilder::for_profile(context.profile).expect("installed build tools");
    let error = builder
        .build_workspace_app_artifacts(&context, &roles)
        .err()
        .expect("whole-App build must reject invalid roles before infrastructure compilation");
    assert!(
        error
            .downcast_ref::<ConfiguredBuildSpecFailures>()
            .is_some()
    );
}

fn build_context() -> WorkspaceBuildContext {
    WorkspaceBuildContext {
        role: "root".to_string(),
        profile: CanisterBuildProfile::Release,
        environment: "local".to_string(),
        build_network: BuildNetwork::Local,
        workspace_root: PathBuf::from("/workspace"),
        icp_root: PathBuf::from("/workspace"),
        config_path: PathBuf::from("/workspace/apps/demo/canic.toml"),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    }
}

fn build_spec(
    role: &str,
    package_name: &str,
    cargo_workspace_root: &str,
) -> CanisterArtifactBuildSpec {
    let artifact_root = PathBuf::from("/artifacts").join(role);
    CanisterArtifactBuildSpec {
        role: role.to_string(),
        package_name: package_name.to_string(),
        package_version: "0.101.51".to_string(),
        canic_version: "0.101.51".to_string(),
        capabilities: std::collections::BTreeSet::new(),
        package_manifest_path: PathBuf::from(cargo_workspace_root)
            .join(role)
            .join("Cargo.toml"),
        cargo_workspace_root: PathBuf::from(cargo_workspace_root),
        wasm_path: artifact_root.join(format!("{role}.wasm")),
        wasm_gz_path: artifact_root.join(format!("{role}.wasm.gz")),
        did_path: artifact_root.join(format!("{role}.did")),
        artifact_root,
    }
}
