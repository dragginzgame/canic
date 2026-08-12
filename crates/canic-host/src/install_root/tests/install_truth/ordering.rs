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
        "resolve_current_fleet_install_input(",
        "prepare_install_deployment_truth(",
    );
    assert_before(
        install,
        "prepare_install_deployment_truth(",
        "plan_fleet_install_session(",
    );

    let prepare = include_str!("../../preparation/mod.rs");
    assert_prebuild_gate_order(prepare);
    assert_before(
        install,
        "emit_manifest_with_phase(",
        "plan_fleet_install_session(",
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
        "plan_current_fleet_install(",
        "install_current_fleet_coordinator(",
    );
    let fleet_planning = source_section(
        source,
        "fn plan_current_fleet_install(",
        "fn resolve_current_fleet_install_input(",
    );
    assert_before(
        fleet_planning,
        "plan_current_fleet_install_session(",
        "persist_current_fleet_install_plan(",
    );
    assert_before(
        install,
        "install_current_fleet_coordinator(",
        "install_current_fleet_subnet_roots(",
    );
    assert_before(
        install,
        "install_current_fleet_subnet_roots(",
        "bootstrap_and_verify_fleet_subnet_root_stores(",
    );
    assert_before(
        install,
        "bootstrap_and_verify_fleet_subnet_root_stores(",
        "register_and_verify_fleet_subnet_roots_joining(",
    );
    assert_before(
        install,
        "register_and_verify_fleet_subnet_roots_joining(",
        "synchronize_and_verify_fleet_subnet_roots(",
    );
    assert_current_activation_order(install);
    let coordinator_install = source_section(
        source,
        "fn install_current_fleet_coordinator(",
        "fn install_current_fleet_subnet_roots(",
    );
    assert!(
        coordinator_install.contains("install_and_verify_fleet_coordinator("),
        "Coordinator wrapper must invoke the journalled install and verification workflow"
    );
    let root_install = source_section(
        source,
        "fn install_current_fleet_subnet_roots(",
        "fn persist_pre_root_receipts(",
    );
    assert!(
        root_install.contains("install_and_verify_fleet_subnet_roots("),
        "root wrapper must invoke the journalled multi-root install and verification workflow"
    );
}

fn assert_prebuild_gate_order(prepare: &str) {
    assert_before(
        prepare,
        "ensure_current_install_executor_capabilities(execution_context)?",
        "run_install_early_authority_preflight(",
    );
    assert_before(
        prepare,
        "run_install_early_authority_preflight(",
        "build_install_targets_with_phase(",
    );
    assert_before(
        prepare,
        "build_install_targets_with_phase(",
        "run_install_deployment_truth_safety_gate(",
    );
}

fn assert_current_activation_order(install: &str) {
    assert_before(
        install,
        "synchronize_and_verify_fleet_subnet_roots(",
        "activate_and_verify_fleet_registry(",
    );
    assert_before(
        install,
        "activate_and_verify_fleet_registry(",
        "activate_and_verify_fleet_subnet_root_registry_mirrors(",
    );
    assert_before(
        install,
        "activate_and_verify_fleet_subnet_root_registry_mirrors(",
        "prepare_current_fleet_subnet_root_component_registries(",
    );
    assert_before(
        install,
        "prepare_current_fleet_subnet_root_component_registries(",
        "install_fleet_components_and_publish_catalog(",
    );

    let component_install = include_str!("../../fleet_component_provisioning_install/mod.rs");
    assert_before(
        component_install,
        "FleetComponentProvisioningInstallPhase::RuntimesActivated =>",
        "FleetComponentProvisioningInstallPhase::CatalogPublicationInFlight =>",
    );
    assert!(
        component_install.contains("begin_fleet_catalog_publication("),
        "catalog publication intent must be durable after runtime activation"
    );

    let closeout = include_str!("../../fleet_catalog_closeout/mod.rs");
    let closeout = source_section(
        closeout,
        "pub(super) fn publish_installed_fleet_catalog(",
        "fn query_registry(",
    );
    assert_before(
        closeout,
        "validate_terminal_fleet_registry(",
        "query_root_summaries(",
    );
    assert_before(
        closeout,
        "query_root_summaries(",
        "publish_terminal_fleet_catalog(",
    );
}

#[test]
fn current_install_persists_truth_only_after_session_planning() {
    let source = include_str!("../../mod.rs");
    let install_start = source
        .find("pub fn install_root(")
        .expect("install_root function exists");
    let install = &source[install_start..];
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
        "plan_fleet_install_session(",
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
