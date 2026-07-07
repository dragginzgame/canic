use super::*;

#[test]
fn renders_registry_roles_from_decoded_role_list() {
    assert_eq!(
        render_registry_roles(&["root".to_string(), "worker".to_string()]),
        "root, worker"
    );
    assert_eq!(render_registry_roles(&[]), "<empty>");
}

#[test]
fn registry_roles_json_query_preserves_diagnostic_summaries() {
    assert_eq!(
        registry_roles_from_json(r#"{"Ok":[{"role":"root"},{"role":"worker"}]}"#),
        "root, worker"
    );
    assert_eq!(registry_roles_from_json(r#"{"Ok":[]}"#), "<empty>");
    assert_eq!(
        registry_roles_from_json(r#"{"Err":"registry unavailable"}"#),
        "<unavailable>"
    );
}
