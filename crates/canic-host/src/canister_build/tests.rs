use super::{WorkspaceBuildContext, parse_parent_process_id, remove_stale_icp_candid_sidecars};
use crate::canister_build::model::CanisterArtifactSource;
use crate::test_support::temp_dir;
use canic_core::ids::BuildNetwork;
use std::fs;

#[test]
fn infrastructure_roles_use_the_canonical_built_in_artifact_sources() {
    assert_eq!(
        CanisterArtifactSource::for_role("fleet_coordinator"),
        CanisterArtifactSource::FleetCoordinator
    );
    assert_eq!(
        CanisterArtifactSource::for_role("wasm_store"),
        CanisterArtifactSource::WasmStore
    );
    assert_eq!(
        CanisterArtifactSource::for_role("root"),
        CanisterArtifactSource::DeclaredRole
    );
}

#[test]
fn parse_parent_process_id_accepts_proc_stat_shape() {
    let stat = "12345 (build_canister_ar) S 67890 0 0 0";
    assert_eq!(parse_parent_process_id(stat), Some(67890));
}

#[test]
fn remove_stale_icp_candid_sidecars_keeps_primary_role_did() {
    let temp_root = temp_dir("canic-canister-build-sidecars");
    let _ = fs::remove_dir_all(&temp_root);
    fs::create_dir_all(&temp_root).unwrap();

    for name in [
        "constructor.did",
        "service.did",
        "service.did.d.ts",
        "service.did.js",
        "app.did",
    ] {
        fs::write(temp_root.join(name), "x").unwrap();
    }

    remove_stale_icp_candid_sidecars(&temp_root).unwrap();

    assert!(!temp_root.join("constructor.did").exists());
    assert!(!temp_root.join("service.did").exists());
    assert!(!temp_root.join("service.did.d.ts").exists());
    assert!(!temp_root.join("service.did.js").exists());
    assert!(temp_root.join("app.did").exists());

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn build_context_distinguishes_environment_from_build_network() {
    let context = WorkspaceBuildContext {
        role: "app".to_string(),
        profile: super::CanisterBuildProfile::Fast,
        environment: "staging".to_string(),
        build_network: BuildNetwork::Ic,
        workspace_root: "/workspace".into(),
        icp_root: "/workspace".into(),
        config_path: "/workspace/apps/demo/canic.toml".into(),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };

    let lines = context.lines();

    assert!(lines.contains(&"environment: staging".to_string()));
    assert!(lines.contains(&"build network: ic".to_string()));
}

#[test]
fn release_build_artifacts_use_one_immutable_identity_namespace() {
    let release_build_id = canic_core::ids::ReleaseBuildId::from_nonce(
        canic_core::ids::ReleaseBuildNonce::from_random_bytes([9; 32]),
    );
    let context = WorkspaceBuildContext {
        role: "root".to_string(),
        profile: super::CanisterBuildProfile::Release,
        environment: "proof".to_string(),
        build_network: BuildNetwork::Ic,
        workspace_root: "/workspace".into(),
        icp_root: "/project".into(),
        config_path: "/workspace/apps/demo/canic.toml".into(),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: Some(release_build_id),
    };

    assert_eq!(
        context.artifact_root(),
        std::path::Path::new("/project/.canic/release-builds")
            .join(release_build_id.to_string())
            .join("artifacts")
    );
    assert_eq!(
        context.clone().with_role("app").artifact_root(),
        context.artifact_root()
    );
}

#[test]
fn unqualified_artifacts_keep_the_local_icp_build_surface() {
    let context = WorkspaceBuildContext {
        role: "root".to_string(),
        profile: super::CanisterBuildProfile::Release,
        environment: "local".to_string(),
        build_network: BuildNetwork::Local,
        workspace_root: "/workspace".into(),
        icp_root: "/project".into(),
        config_path: "/workspace/apps/demo/canic.toml".into(),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };

    assert_eq!(
        context.artifact_root(),
        std::path::Path::new("/project/.icp/local/canisters")
    );
}

#[test]
fn build_context_applies_exact_child_build_network() {
    let context = WorkspaceBuildContext {
        role: "app".to_string(),
        profile: super::CanisterBuildProfile::Fast,
        environment: "staging".to_string(),
        build_network: BuildNetwork::Ic,
        workspace_root: "/workspace".into(),
        icp_root: "/project".into(),
        config_path: "/workspace/apps/demo/canic.toml".into(),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: Some(canic_core::ids::ReleaseBuildId::from_nonce(
            canic_core::ids::ReleaseBuildNonce::from_random_bytes([7; 32]),
        )),
    };
    let mut command = std::process::Command::new("cargo");

    context.apply_to_command(&mut command);

    assert_eq!(
        command.get_envs().find(|(key, _)| {
            *key == std::ffi::OsStr::new(canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV)
        }),
        Some((
            std::ffi::OsStr::new(canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV),
            None,
        ))
    );
    let environment = command
        .get_envs()
        .filter_map(|(key, value)| value.map(|value| (key, value)))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        environment.get(std::ffi::OsStr::new("ICP_ENVIRONMENT")),
        Some(&std::ffi::OsStr::new("ic"))
    );
    assert_eq!(
        environment.get(std::ffi::OsStr::new(
            canic_core::role_contract::CANONICAL_BUILD_ICP_ROOT_ENV,
        )),
        Some(&std::ffi::OsStr::new("/project"))
    );
    assert_eq!(
        environment.get(std::ffi::OsStr::new(
            canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
        )),
        Some(&std::ffi::OsStr::new("/workspace/apps/demo/canic.toml"))
    );
    assert_eq!(
        environment
            .get(std::ffi::OsStr::new(canic_core::ids::RELEASE_BUILD_ID_ENV,))
            .copied(),
        context
            .release_build_id
            .map(|value| value.to_string())
            .as_deref()
            .map(std::ffi::OsStr::new)
    );
}

#[test]
fn unqualified_build_context_removes_an_ambient_release_build_id() {
    let context = WorkspaceBuildContext {
        role: "app".to_string(),
        profile: super::CanisterBuildProfile::Fast,
        environment: "local".to_string(),
        build_network: BuildNetwork::Local,
        workspace_root: "/workspace".into(),
        icp_root: "/project".into(),
        config_path: "/workspace/apps/demo/canic.toml".into(),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };
    let mut command = std::process::Command::new("cargo");
    command.env(canic_core::ids::RELEASE_BUILD_ID_ENV, "ambient");

    context.apply_to_command(&mut command);

    assert!(
        command.get_envs().any(|(key, value)| {
            key == canic_core::ids::RELEASE_BUILD_ID_ENV && value.is_none()
        })
    );
}

#[test]
fn configured_canister_build_uses_the_locked_resolver() {
    let context = WorkspaceBuildContext {
        role: "app".to_string(),
        profile: super::CanisterBuildProfile::Fast,
        environment: "local".to_string(),
        build_network: BuildNetwork::Local,
        workspace_root: "/workspace".into(),
        icp_root: "/project".into(),
        config_path: "/workspace/apps/demo/canic.toml".into(),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };
    let command = super::artifact::canister_cargo_build_command(
        &context,
        std::path::Path::new("/workspace/Cargo.toml"),
        super::CanisterBuildProfile::Fast,
    );
    assert!(command.get_args().any(|argument| argument == "--locked"));
}
