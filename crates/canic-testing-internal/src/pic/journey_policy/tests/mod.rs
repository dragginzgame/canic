use super::*;
use std::path::Path;

fn configuration(name: &str) -> AppConfigSnapshot {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    AppConfigSnapshot::load(&workspace.join("canisters/audit/root_probe").join(name)).unwrap()
}

#[test]
fn ungrouped_import_fixture_retains_declared_admission() {
    let config = configuration("canic.toml");
    assert!(config.model().component_groups.is_empty());
    assert_eq!(
        component_admissions(&config),
        BTreeMap::from([("default".into(), 1)])
    );
}

#[test]
fn grouped_recovery_fixtures_retain_member_admissions() {
    for (name, expected) in [("retained-estate.toml", 2), ("two-workloads.toml", 1)] {
        let config = configuration(name);
        assert!(!config.model().component_groups.is_empty());
        assert_eq!(
            component_admissions(&config),
            BTreeMap::from([("default".into(), expected)])
        );
    }
}
