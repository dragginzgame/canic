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
        profile: "fast".to_string(),
        requested_profile: "unset".to_string(),
        environment: "staging".to_string(),
        build_network: "ic".to_string(),
        workspace_root: "/workspace".into(),
        icp_root: "/workspace".into(),
    };

    let lines = context.lines();

    assert!(lines.contains(&"environment: staging".to_string()));
    assert!(lines.contains(&"build network: ic".to_string()));
    assert!(!lines.iter().any(|line| line == "network: staging"));
}
