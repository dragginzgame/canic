use super::super::*;
use crate::canister_build::CanisterBuildProfile;
use canic_contracts::ids::BuildNetwork;

#[test]
fn root_commands_bind_config_and_separate_declaration_from_runtime() {
    let context = WorkspaceBuildContext {
        role: "root".into(),
        profile: CanisterBuildProfile::Fast,
        environment: "local".into(),
        build_network: BuildNetwork::Local,
        workspace_root: "/workspace".into(),
        icp_root: "/workspace/app".into(),
        config_path: "/workspace/app/canic.toml".into(),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };
    let manifest = crate::canonical_root::manifest_path(&context.config_path);
    for declaration in [false, true] {
        let command =
            cargo_command_for_role(InfrastructureRole::Root, &context, &manifest, declaration)
                .command()
                .unwrap();
        let args = command.get_args().collect::<Vec<_>>();
        assert_eq!(args[0], "build");
        assert!(args.iter().any(|arg| *arg == "--locked"));
        assert!(args.iter().any(|arg| *arg == "--lib"));
        assert!(
            args.windows(2)
                .any(|args| args == ["--target", "wasm32-unknown-unknown"])
        );
        let env = command
            .get_envs()
            .collect::<std::collections::BTreeMap<_, _>>();
        let key = std::ffi::OsStr::new(canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV);
        assert_eq!(env[&key], Some(context.config_path.as_os_str()));
        let key = std::ffi::OsStr::new(canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV);
        assert_eq!(
            env.get(&key).copied().flatten(),
            declaration.then(|| std::ffi::OsStr::new("1"))
        );
        let key = std::ffi::OsStr::new(canic_core::role_contract::PROTOCOL_PROFILE_DIGEST_ENV);
        assert!(!env.contains_key(&key));
    }
}

#[test]
fn root_source_preserves_the_configuration_admitted_capabilities() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let context = WorkspaceBuildContext {
        role: "root".into(),
        profile: CanisterBuildProfile::Fast,
        environment: "local".into(),
        build_network: BuildNetwork::Local,
        workspace_root: workspace.clone(),
        icp_root: workspace.clone(),
        config_path: workspace.join("apps/demo/canic.toml"),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    };
    let config = crate::release_set::AppConfigSnapshot::load(&context.config_path).unwrap();
    let source = resolve_source(InfrastructureRole::Root, &context).unwrap();
    let contract = crate::role_contract::resolve_canonical_root_contract(config.model());
    let canic_core::role_contract::RoleContractResolution::Resolved { contract } = contract else {
        panic!("demo Root must have a resolved capability contract");
    };
    assert_eq!(source.capabilities, contract.capabilities);
    assert_eq!(
        source.manifest_path,
        crate::canonical_root::manifest_path(&context.config_path)
    );
    assert_eq!(source.package_name, crate::canonical_root::PACKAGE);
}
