use super::*;

#[test]
fn current_install_records_gates_before_activation_mutation() {
    let source = include_str!("../../mod.rs");
    let install_start = source
        .find("pub fn install_root(")
        .expect("install_root function exists");
    let install = &source[install_start..];
    assert_before(
        install,
        "prepare_install_deployment_truth(",
        "run_root_activation_phases(",
    );

    let prepare = include_str!("../../preparation/mod.rs");
    assert_before(
        prepare,
        "ensure_current_install_executor_capabilities(execution_context)?",
        "run_install_deployment_truth_safety_gate(",
    );

    let gate = include_str!("../../current_execution/mod.rs");
    assert_before(
        gate,
        "enforce_install_deployment_truth_gate(&deployment_truth_check)?",
        "write_current_install_execution_preflight_receipt(",
    );
    assert_before(
        gate,
        "write_current_install_execution_preflight_receipt(",
        "Ok(deployment_truth_check)",
    );
}

#[test]
fn current_install_check_paths_do_not_write_or_mutate_state() {
    let source = include_str!("../../truth_check/mod.rs");
    let check_paths = source_section(
        source,
        "pub fn check_install_deployment_truth(",
        "fn resolve_current_install_truth_inputs(",
    );

    for forbidden in [
        "write_install_state(",
        "write_install_state_with_deployment_truth_receipt(",
        "write_install_deployment_truth_receipt(",
        "write_current_install_execution_preflight_receipt(",
        "register_deployment_state(",
        "run_root_activation_phases(",
        "install_root(",
    ] {
        assert!(
            !check_paths.contains(forbidden),
            "read-only install check/preflight paths must not contain {forbidden}"
        );
    }
}
