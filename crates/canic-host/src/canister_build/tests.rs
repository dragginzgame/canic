use super::{WorkspaceBuildContext, parse_parent_process_id, remove_stale_icp_candid_sidecars};
use crate::test_support::temp_dir;
use std::fs;

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
        requested_profile: "unset".to_string(),
        environment: "staging".to_string(),
        build_network: "ic".to_string(),
        workspace_root: "/workspace".into(),
        icp_root: "/workspace".into(),
        config_path: "/workspace/fleets/demo/canic.toml".into(),
        local_replica: None,
    };

    let lines = context.lines();

    assert!(lines.contains(&"environment: staging".to_string()));
    assert!(lines.contains(&"build network: ic".to_string()));
    assert!(!lines.iter().any(|line| line == "network: staging"));
}

#[test]
fn build_context_applies_exact_child_environment() {
    let context = WorkspaceBuildContext {
        role: "app".to_string(),
        profile: super::CanisterBuildProfile::Fast,
        requested_profile: "fast".to_string(),
        environment: "staging".to_string(),
        build_network: "ic".to_string(),
        workspace_root: "/workspace".into(),
        icp_root: "/project".into(),
        config_path: "/workspace/fleets/demo/canic.toml".into(),
        local_replica: None,
    };
    let mut command = std::process::Command::new("cargo");

    context.apply_to_command(&mut command);

    assert!(
        command
            .get_envs()
            .any(|(key, value)| { key == "CANIC_ICP_LOCAL_NETWORK_URL" && value.is_none() })
    );
    assert!(
        command
            .get_envs()
            .any(|(key, value)| { key == "CANIC_ICP_LOCAL_ROOT_KEY" && value.is_none() })
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
        environment.get(std::ffi::OsStr::new("CANIC_WORKSPACE_ROOT")),
        Some(&std::ffi::OsStr::new("/workspace"))
    );
    assert_eq!(
        environment.get(std::ffi::OsStr::new("CANIC_ICP_ROOT")),
        Some(&std::ffi::OsStr::new("/project"))
    );
    assert_eq!(
        environment.get(std::ffi::OsStr::new("CANIC_CONFIG_PATH")),
        Some(&std::ffi::OsStr::new("/workspace/fleets/demo/canic.toml"))
    );
}

#[test]
fn build_context_applies_explicit_local_replica_to_child() {
    let mut context = WorkspaceBuildContext {
        role: "root".to_string(),
        profile: super::CanisterBuildProfile::Fast,
        requested_profile: "fast".to_string(),
        environment: "local".to_string(),
        build_network: "local".to_string(),
        workspace_root: "/workspace".into(),
        icp_root: "/project".into(),
        config_path: "/workspace/fleets/demo/canic.toml".into(),
        local_replica: None,
    };
    context.local_replica = Some(crate::icp::LocalReplicaTarget {
        url: "http://127.0.0.1:8000".to_string(),
        root_key: "abcd".to_string(),
    });
    let mut command = std::process::Command::new("cargo");

    context.apply_to_command(&mut command);

    assert!(command.get_envs().any(|(key, value)| {
        key == "CANIC_ICP_LOCAL_NETWORK_URL"
            && value.is_some_and(|value| value == "http://127.0.0.1:8000")
    }));
    assert!(command.get_envs().any(|(key, value)| {
        key == "CANIC_ICP_LOCAL_ROOT_KEY" && value.is_some_and(|value| value == "abcd")
    }));
}

#[test]
fn sequential_build_contexts_do_not_share_child_authority() {
    let first = WorkspaceBuildContext {
        role: "root".to_string(),
        profile: super::CanisterBuildProfile::Fast,
        requested_profile: "fast".to_string(),
        environment: "local".to_string(),
        build_network: "local".to_string(),
        workspace_root: "/first".into(),
        icp_root: "/first-project".into(),
        config_path: "/first/fleets/demo/canic.toml".into(),
        local_replica: Some(crate::icp::LocalReplicaTarget {
            url: "http://127.0.0.1:8000".to_string(),
            root_key: "first-key".to_string(),
        }),
    };
    let second = WorkspaceBuildContext {
        role: "app".to_string(),
        profile: super::CanisterBuildProfile::Release,
        requested_profile: "release".to_string(),
        environment: "staging".to_string(),
        build_network: "ic".to_string(),
        workspace_root: "/second".into(),
        icp_root: "/second-project".into(),
        config_path: "/second/fleets/prod/canic.toml".into(),
        local_replica: None,
    };
    let mut first_command = std::process::Command::new("cargo");
    let mut second_command = std::process::Command::new("cargo");

    first.apply_to_command(&mut first_command);
    second.apply_to_command(&mut second_command);

    assert!(first_command.get_envs().any(|(key, value)| {
        key == "CANIC_ICP_LOCAL_NETWORK_URL"
            && value.is_some_and(|value| value == "http://127.0.0.1:8000")
    }));

    assert!(second_command.get_envs().any(|(key, value)| {
        key == "CANIC_CONFIG_PATH"
            && value.is_some_and(|value| value == "/second/fleets/prod/canic.toml")
    }));
    assert!(second_command.get_envs().any(|(key, value)| {
        key == "ICP_ENVIRONMENT" && value.is_some_and(|value| value == "ic")
    }));
    assert!(
        second_command
            .get_envs()
            .any(|(key, value)| { key == "CANIC_ICP_LOCAL_NETWORK_URL" && value.is_none() })
    );
    assert!(
        second_command
            .get_envs()
            .any(|(key, value)| { key == "CANIC_ICP_LOCAL_ROOT_KEY" && value.is_none() })
    );
}
