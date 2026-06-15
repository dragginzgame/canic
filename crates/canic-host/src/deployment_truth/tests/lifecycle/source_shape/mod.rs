#[test]
fn external_lifecycle_uses_canonical_control_class_model() {
    let model = include_str!("../../../model/mod.rs");
    let lifecycle_sources = [
        include_str!("../../../lifecycle/mod.rs"),
        include_str!("../../../lifecycle/authority_plan/mod.rs"),
        include_str!("../../../lifecycle/authority_plan/authority/mod.rs"),
        include_str!("../../../lifecycle/authority_plan/plan/mod.rs"),
        include_str!("../../../lifecycle/authority_plan/policy/mod.rs"),
        include_str!("../../../lifecycle/authority_plan/validation/mod.rs"),
        include_str!("../../../lifecycle/external_lifecycle/mod.rs"),
        include_str!("../../../lifecycle/external_lifecycle/check/mod.rs"),
        include_str!("../../../lifecycle/external_lifecycle/critical_fix/mod.rs"),
        include_str!("../../../lifecycle/external_lifecycle/handoff/mod.rs"),
        include_str!("../../../lifecycle/external_lifecycle/pending/mod.rs"),
        include_str!("../../../lifecycle/external_lifecycle/validation/mod.rs"),
        include_str!("../../../lifecycle/external_upgrade/mod.rs"),
        include_str!("../../../lifecycle/external_upgrade/completion/mod.rs"),
        include_str!("../../../lifecycle/external_upgrade/consent/mod.rs"),
        include_str!("../../../lifecycle/external_upgrade/proposal/mod.rs"),
        include_str!("../../../lifecycle/external_upgrade/receipt/mod.rs"),
        include_str!("../../../lifecycle/external_upgrade/validation/mod.rs"),
        include_str!("../../../lifecycle/external_upgrade/verification/mod.rs"),
        include_str!("../../../lifecycle/external_upgrade/verification/check/mod.rs"),
        include_str!("../../../lifecycle/external_upgrade/verification/policy/mod.rs"),
        include_str!("../../../lifecycle/external_upgrade/verification/report/mod.rs"),
        include_str!("../../../lifecycle/external_upgrade/verification/shared/mod.rs"),
    ];

    assert_eq!(model.matches("pub enum CanisterControlClassV1").count(), 1);
    assert!(
        lifecycle_sources
            .iter()
            .any(|source| source.contains("CanisterControlClassV1"))
    );

    for forbidden in [
        "ExternalControlClass",
        "ExternalLifecycleControlClass",
        "LifecycleControlClass",
        "UserControlClass",
        "UserLifecycleControlClass",
    ] {
        assert!(
            lifecycle_sources
                .iter()
                .all(|source| !source.contains(forbidden)),
            "external lifecycle must project from CanisterControlClassV1; found {forbidden}"
        );
    }
}
