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
        "plan_fleet_install_activation(",
    );

    let prepare = include_str!("../../preparation/mod.rs");
    assert_before(
        prepare,
        "ensure_current_install_executor_capabilities(execution_context)?",
        "run_install_deployment_truth_safety_gate(",
    );
    assert_before(
        install,
        "emit_manifest_with_phase(",
        "plan_fleet_install_activation(",
    );
    let manifest_emission = include_str!("../../plan_artifacts/mod.rs");
    let manifest_emission = source_section(
        manifest_emission,
        "pub(super) fn emit_manifest_with_phase(",
        "fn application_file_build_outputs(",
    );
    assert_before(
        manifest_emission,
        "compile_and_persist_application_artifact_union(",
        "finalize_release_build_from_manifest(",
    );
    assert_before(
        install,
        "plan_fleet_install_activation(",
        "resolve_root_canister_after_manifest(",
    );
    assert_before(
        install,
        "recover_activation_root_canister(",
        "resolve_root_canister_after_manifest(",
    );

    let gate = include_str!("../../current_execution/mod.rs");
    assert_before(
        gate,
        "enforce_install_deployment_truth_gate(&deployment_truth_check)?",
        "current_install_execution_preflight_receipt(",
    );
    assert_before(
        gate,
        "current_install_execution_preflight_receipt(",
        "Ok(PreparedInstallSafetyGate",
    );
    assert_before(
        install,
        "plan_fleet_install_activation(",
        ".write_receipt(receipt)",
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

    for forbidden in ["write_install_deployment_truth_receipt(", "install_root("] {
        assert!(
            !check_paths.contains(forbidden),
            "read-only install check/preflight paths must not contain {forbidden}"
        );
    }
}
