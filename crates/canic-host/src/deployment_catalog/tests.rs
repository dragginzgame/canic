use super::*;
use crate::test_support::temp_dir;

#[test]
fn catalog_lists_deployment_target_state_sorted_by_deployment() {
    let root = temp_dir("canic-catalog-list");
    write_state(&root, "local", sample_state("zeta", "demo", "root-z"));
    write_state(&root, "local", sample_state("alpha", "demo", "root-a"));
    let request = request(&root);

    let report = build_deployment_catalog_report(&request).expect("catalog");

    fs::remove_dir_all(root).expect("clean");
    assert_eq!(
        report
            .entries
            .iter()
            .map(|entry| entry.deployment.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha", "zeta"]
    );
    assert_eq!(report.entries[0].fleet.as_deref(), Some("demo"));
    assert_eq!(report.entries[0].network.as_deref(), Some("local"));
    assert_eq!(report.entries[0].root_principal.as_deref(), Some("root-a"));
    assert_eq!(
        report.entries[0].root_verification,
        DeploymentCatalogRootVerificationV1::Verified
    );
    let state_ref = report.entries[0]
        .local_state_ref
        .as_ref()
        .expect("fingerprint");
    assert_eq!(state_ref.kind, "deployment_state");
    assert_eq!(
        state_ref.path.as_deref(),
        Some(".canic/local/deployments/alpha.json")
    );
}

#[test]
fn catalog_returns_empty_warning_when_deployment_state_is_missing() {
    let root = temp_dir("canic-catalog-empty");
    fs::create_dir_all(&root).expect("create temp root");
    let request = request(&root);

    let report = build_deployment_catalog_report(&request).expect("catalog");

    fs::remove_dir_all(root).expect("clean");
    assert!(report.entries.is_empty());
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.code == "catalog.no_deployment_state")
    );
}

#[test]
fn catalog_ignores_legacy_fleet_state() {
    let root = temp_dir("canic-catalog-legacy");
    let legacy = root.join(".canic/local/fleets");
    fs::create_dir_all(&legacy).expect("legacy dir");
    fs::write(legacy.join("demo.json"), "{}").expect("legacy state");
    let request = request(&root);

    let report = build_deployment_catalog_report(&request).expect("catalog");

    fs::remove_dir_all(root).expect("clean");
    assert!(report.entries.is_empty());
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.code == "catalog.legacy_fleet_state_ignored")
    );
}

#[test]
fn catalog_warns_and_keeps_valid_entries_when_one_entry_is_malformed() {
    let root = temp_dir("canic-catalog-malformed");
    write_state(&root, "local", sample_state("demo", "demo", "root"));
    let dir = root.join(".canic/local/deployments");
    fs::write(dir.join("broken.json"), "{not-json").expect("broken state");
    let request = request(&root);

    let report = build_deployment_catalog_report(&request).expect("catalog");

    fs::remove_dir_all(root).expect("clean");
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].deployment, "demo");
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.code == "catalog.malformed_deployment_state")
    );
}

#[test]
fn catalog_inspect_filters_known_deployment() {
    let root = temp_dir("canic-catalog-inspect");
    write_state(&root, "local", sample_state("alpha", "demo", "root-a"));
    write_state(&root, "local", sample_state("beta", "demo", "root-b"));
    let request = request(&root);

    let report = inspect_deployment_catalog_report(&request, "beta").expect("inspect");

    fs::remove_dir_all(root).expect("clean");
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].deployment, "beta");
}

#[test]
fn catalog_inspect_rejects_unknown_deployment() {
    let root = temp_dir("canic-catalog-unknown");
    write_state(&root, "local", sample_state("alpha", "demo", "root-a"));
    let request = request(&root);

    let err = inspect_deployment_catalog_report(&request, "demo").expect_err("unknown deployment");

    fs::remove_dir_all(root).expect("clean");
    assert!(matches!(
        err,
        DeploymentCatalogError::UnknownDeployment { deployment, .. } if deployment == "demo"
    ));
}

#[test]
fn catalog_text_uses_deployment_target_terms() {
    let root = temp_dir("canic-catalog-text");
    write_state(&root, "local", sample_state("demo-local", "demo", "root"));
    let request = request(&root);
    let report = build_deployment_catalog_report(&request).expect("catalog");

    let text = deployment_catalog_report_text(&report);

    fs::remove_dir_all(root).expect("clean");
    assert!(text.contains("Deployment catalog:"));
    assert!(text.contains("demo-local"));
    assert!(text.contains("root_verification: verified"));
    assert!(!text.contains("fleet template catalog"));
}

fn request(root: &Path) -> DeploymentCatalogRequest {
    DeploymentCatalogRequest {
        icp_root: root.to_path_buf(),
        network: "local".to_string(),
        generated_at: "unix:54".to_string(),
    }
}

fn write_state(root: &Path, network: &str, state: InstallState) {
    let path = root
        .join(".canic")
        .join(network)
        .join("deployments")
        .join(format!("{}.json", state.deployment_name));
    fs::create_dir_all(path.parent().expect("state parent")).expect("state dir");
    fs::write(path, serde_json::to_vec_pretty(&state).expect("state json")).expect("write state");
}

fn sample_state(deployment: &str, fleet: &str, root: &str) -> InstallState {
    InstallState {
        schema_version: 2,
        deployment_name: deployment.to_string(),
        fleet_template: fleet.to_string(),
        created_at_unix_secs: 1,
        updated_at_unix_secs: 2,
        network: "local".to_string(),
        root_target: "root".to_string(),
        root_canister_id: root.to_string(),
        root_verification: RootVerificationStatus::Verified,
        root_build_target: "root".to_string(),
        workspace_root: ".".to_string(),
        icp_root: ".".to_string(),
        config_path: "fleets/demo/canic.toml".to_string(),
        release_set_manifest_path: ".canic/local/release-set.json".to_string(),
    }
}
