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
        environment: "staging".to_string(),
        build_network: "ic".to_string(),
        workspace_root: "/workspace".into(),
        icp_root: "/workspace".into(),
        config_path: "/workspace/fleets/demo/canic.toml".into(),
        local_replica: None,
        refresh_canonical_wasm_store_did: false,
    };

    let lines = context.lines();

    assert!(lines.contains(&"environment: staging".to_string()));
    assert!(lines.contains(&"build network: ic".to_string()));
}

#[test]
fn build_context_applies_exact_child_build_network() {
    let context = WorkspaceBuildContext {
        role: "app".to_string(),
        profile: super::CanisterBuildProfile::Fast,
        environment: "staging".to_string(),
        build_network: "ic".to_string(),
        workspace_root: "/workspace".into(),
        icp_root: "/project".into(),
        config_path: "/workspace/fleets/demo/canic.toml".into(),
        local_replica: None,
        refresh_canonical_wasm_store_did: false,
    };
    let mut command = std::process::Command::new("cargo");

    context.apply_to_command(&mut command);

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
        Some(&std::ffi::OsStr::new("/workspace/fleets/demo/canic.toml"))
    );
}
