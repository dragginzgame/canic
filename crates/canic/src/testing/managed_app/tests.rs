use super::*;

const APP_CONFIG: &str = r#"
[app]
name = "qualification"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"
fleet_admission = true

[component_specs.app]
component_role = "app"
maximum_instances = 1

[component_groups.app.components.app]
component_spec = "app"

[component_group_deployments.app]
component_group = "app"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#;

#[test]
fn managed_app_input_compiles_exact_group_authority() {
    let release_build_id = "01".repeat(32);
    let input = ManagedAppQualificationInput::new(
        APP_CONFIG,
        "app",
        "app",
        &release_build_id,
        vec![Principal::from_slice(&[0x51; 29])],
        vec![0, 97, 115, 109],
    );
    let app = Principal::from_slice(&[0x52; 29]);
    let compiled = compile_managed_app(&input, app).expect("compile managed App authority");

    assert_eq!(compiled.directory.operation_id.len(), 32);
    assert_eq!(
        compiled
            .directory
            .authority
            .component
            .provenance
            .component
            .canister_id,
        app
    );
    assert_eq!(
        compiled
            .directory
            .authority
            .component_group
            .as_ref()
            .expect("group authority")
            .members
            .len(),
        1
    );
    assert!(!compiled.init_args.is_empty());
}

#[test]
fn managed_app_input_rejects_ambiguous_component_occurrence() {
    let config = APP_CONFIG.replace(
        "[component_groups.app.components.app]",
        "[component_groups.app.components.first]",
    ) + r#"

[component_groups.app.components.second]
component_spec = "app"
"#;
    let release_build_id = "01".repeat(32);
    let input = ManagedAppQualificationInput::new(
        &config,
        "app",
        "app",
        &release_build_id,
        vec![Principal::from_slice(&[0x51; 29])],
        Vec::new(),
    );
    let error = compile_managed_app(&input, Principal::from_slice(&[0x52; 29]))
        .expect_err("reject ambiguous Component occurrence");
    assert!(matches!(error, ManagedAppQualificationError::Config(_)));
}
