use super::*;

#[test]
fn renders_registry_roles_from_decoded_role_list() {
    assert_eq!(
        render_registry_roles(&["root".to_string(), "worker".to_string()]),
        "root, worker"
    );
    assert_eq!(render_registry_roles(&[]), "<empty>");
}
