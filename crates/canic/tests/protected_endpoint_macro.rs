#![expect(clippy::unused_async)]

use canic::{api::canister::CanisterRole, cdk::types::Principal};

#[canic::canic_update(
    internal,
    name = "wire_system_add_project_to_user",
    requires(caller::has_role("project_hub"))
)]
async fn system_add_project_to_user(
    user_id: Principal,
    project_id: Principal,
) -> Result<(), canic::Error> {
    let _ = (user_id, project_id);

    Ok(())
}

canic::canic_protected_endpoint! {
    pub fn shared_project_update_endpoint =
        "wire_shared_project_update",
        role = CanisterRole::new("project_hub");

    fn shared_multi_role_project_endpoint =
        "wire_shared_multi_role_project_update",
        roles = [
            CanisterRole::new("project_hub"),
            CanisterRole::new("admin_hub"),
        ];
}

#[test]
fn protected_endpoint_macro_descriptor_is_available() {
    let descriptor = canic_internal_endpoint_system_add_project_to_user();

    assert_eq!(descriptor.method(), "wire_system_add_project_to_user");
    assert_eq!(
        descriptor.single_role(),
        Some(&CanisterRole::new("project_hub"))
    );
}

#[test]
fn protected_endpoint_descriptor_macro_supports_shared_protocol_modules() {
    let single_role = shared_project_update_endpoint();
    assert_eq!(single_role.method(), "wire_shared_project_update");
    assert_eq!(
        single_role.single_role(),
        Some(&CanisterRole::new("project_hub"))
    );

    let multi_role = shared_multi_role_project_endpoint();
    assert_eq!(multi_role.method(), "wire_shared_multi_role_project_update");
    assert!(multi_role.accepts_role(&CanisterRole::new("project_hub")));
    assert!(multi_role.accepts_role(&CanisterRole::new("admin_hub")));
    assert!(multi_role.single_role().is_none());
}
