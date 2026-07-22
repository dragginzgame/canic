// Category C - Artifact and repository-contract test; parses CONFIG.md's embedded example.

const GUIDE: &str = include_str!("../../../CONFIG.md");
const EXAMPLE_START: &str = "# CANIC_CONFIG_EXAMPLE_START\n";
const EXAMPLE_END: &str = "# CANIC_CONFIG_EXAMPLE_END";

#[test]
fn canonical_config_guide_example_is_accepted_by_the_current_schema() {
    let (_, after_start) = GUIDE
        .split_once(EXAMPLE_START)
        .expect("CONFIG.md must contain the canonical example start marker");
    let (example, _) = after_start
        .split_once(EXAMPLE_END)
        .expect("CONFIG.md must contain the canonical example end marker");

    let config = canic::__internal::core::bootstrap::parse_config_model(example)
        .expect("CONFIG.md canonical example must parse and validate");

    assert_eq!(config.fleet_name(), Some("example"));
    assert!(config.roles.contains_key(&canic::ids::CanisterRole::ROOT));
    assert_eq!(config.attached_roles().len(), 7);
}
