use super::*;

#[test]
fn current_install_records_gates_before_activation_mutation() {
    let source = include_str!("../../mod.rs");
    let install_start = source
        .find("pub fn install_root(")
        .expect("install_root function exists");
    let install = &source[install_start..];
    assert_current_fresh_fleet_admission_order(source, install);
    assert_before(
        install,
        "current_install_build_inputs(",
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

#[test]
fn every_pre_provisioning_registry_gate_accepts_the_same_proven_successor() {
    let join = include_str!("../../fleet_subnet_root_registry_join/mod.rs");
    assert!(
        join.contains("require_joining_or_recovered_registry("),
        "the earliest Registry join gate must accept the shared proven recovery state"
    );

    for (label, source) in [
        (
            "root synchronization",
            include_str!("../../fleet_subnet_root_registry_sync/mod.rs"),
        ),
        (
            "Registry activation",
            include_str!("../../fleet_registry_activation/mod.rs"),
        ),
    ] {
        assert!(
            source.contains("require_active_or_service_successor_registry("),
            "{label} must accept the shared proven recovery state"
        );
    }
}

fn assert_current_fresh_fleet_admission_order(source: &str, install: &str) {
    assert_before(
        install,
        "prepare_and_admit_current_fresh_fleet(",
        "current_install_build_inputs(",
    );
    let admission = source_section(
        source,
        "fn prepare_and_admit_current_fresh_fleet(",
        "fn prepare_current_fresh_fleet_preflight(",
    );
    assert_before(
        admission,
        "FleetCatalogAcquisition::RefreshMissingOrInvalid",
        "FleetCatalogAcquisition::CacheOnly",
    );
    assert_before(
        admission,
        "prepare_current_fresh_fleet_preflight(",
        "require_recompiled_fresh_fleet_plan(",
    );
    let fresh_preflight = source_section(
        source,
        "fn prepare_current_fresh_fleet_preflight(",
        "fn install_current_fleet_infrastructure(",
    );
    assert_fresh_preflight_order(fresh_preflight);
}

fn assert_fresh_preflight_order(preflight: &str) {
    assert_before(
        preflight,
        "resolve_canonical_network_id_from_root(",
        "resolve_current_fleet_install_input(",
    );
    assert_before(
        preflight,
        "resolve_current_fleet_install_input(",
        "current_install_preflight_release_source(",
    );
    assert_before(
        preflight,
        "current_install_preflight_release_source(",
        "compile_current_fresh_fleet_preflight(",
    );
    assert_before(
        preflight,
        "compile_current_fresh_fleet_preflight(",
        "fresh_fleet_maximum_operator_debit(",
    );
    assert_before(
        preflight,
        "fresh_fleet_maximum_operator_debit(",
        "observe_fresh_fleet_operator_funding(",
    );
    assert_before(
        preflight,
        "observe_fresh_fleet_operator_funding(",
        "load_fresh_fleet_decision_authority(",
    );
    assert_before(
        preflight,
        "load_fresh_fleet_decision_authority(",
        "compile_fresh_fleet_deployment_plan(",
    );
    assert_before(
        preflight,
        "compile_fresh_fleet_deployment_plan(",
        "require_fresh_fleet_plan_digest(",
    );

    let source = include_str!("../../mod.rs");
    let resolution = source_section(
        source,
        "fn resolve_current_fleet_install_input(",
        "fn current_fleet_install_input_path(",
    );
    assert!(
        resolution.contains("load_and_resolve_fleet_install_input("),
        "install catalog acquisition must use the live-capable loader"
    );
    assert!(
        resolution.contains("load_and_resolve_fleet_install_input_for_preflight("),
        "exact install recompilation must use a cache-only validated load"
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
