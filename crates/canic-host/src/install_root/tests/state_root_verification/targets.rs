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
