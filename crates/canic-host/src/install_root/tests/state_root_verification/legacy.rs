use super::*;

#[test]
fn deployment_state_allows_distinct_targets_that_share_root() {
    let root = temp_dir("canic-install-state-targets");
    let demo = sample_install_state(&root, "demo-local", "demo");
    let test = sample_install_state(&root, "demo-staging", "demo");

    write_install_state(&root, "local", &demo).expect("write demo state");
    write_install_state(&root, "local", &test).expect("write test state");

    assert_eq!(
        read_deployment_install_state(&root, "local", "demo-local")
            .expect("read demo")
            .expect("demo state exists"),
        demo
    );
    assert_eq!(
        read_deployment_install_state(&root, "local", "demo-staging")
            .expect("read test")
            .expect("test state exists"),
        test
    );

    fs::remove_dir_all(root).expect("clean temp dir");
}

#[test]
fn legacy_fleet_state_is_rejected_as_deployment_truth() {
    let root = temp_dir("canic-install-state-legacy");
    let legacy_path = legacy_fleet_install_state_path(&root, "local", "demo");
    fs::create_dir_all(legacy_path.parent().expect("legacy parent")).expect("create legacy dir");
    fs::write(&legacy_path, b"{}").expect("write legacy state");

    let err = read_deployment_install_state(&root, "local", "demo")
        .expect_err("legacy fleet state must fail closed");
    let message = err.to_string();

    assert!(message.contains("legacy fleet install state found"));
    assert!(message.contains(CURRENT_DEPLOYMENT_STATE_BOUNDARY_MESSAGE));
    assert!(message.contains(
        "canic deploy register demo --fleet-template <fleet-template> --root <principal> --allow-unverified"
    ));
    assert!(message.contains("canic install <fleet-template>"));
    assert!(message.contains(".canic/local/fleets/demo.json"));

    fs::remove_dir_all(root).expect("clean temp dir");
}
