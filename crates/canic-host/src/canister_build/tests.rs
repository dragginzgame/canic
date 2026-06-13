use super::{parse_parent_process_id, remove_stale_icp_candid_sidecars};
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
