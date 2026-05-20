#![expect(clippy::unused_async)]

use canic::{api::canister::CanisterRole, cdk::types::Principal};

fn protected_endpoint() -> canic::api::ic::ProtectedInternalEndpoint {
    canic::api::ic::ProtectedInternalEndpoint::new(
        "wire_protected_update",
        [CanisterRole::new("project_hub")],
    )
}

fn multi_role_endpoint() -> canic::api::ic::ProtectedInternalEndpoint {
    canic::api::ic::ProtectedInternalEndpoint::new(
        "wire_multi_role_update",
        [
            CanisterRole::new("project_hub"),
            CanisterRole::new("admin_hub"),
        ],
    )
}

#[canic::canic_update(
    internal,
    name = "wire_system_add_project_to_user",
    requires(caller::has_role("project_hub"))
)]
async fn system_add_project_to_user(
    _user_id: Principal,
    _project_id: Principal,
) -> Result<(), canic::Error> {
    Ok(())
}

canic::canic_internal_client! {
    pub struct ProjectHubInternalClient {
        fn add_project = protected_endpoint; (
            user_id: Principal,
            project_id: Principal,
        ) -> ();

        fn generated_add_project = canic_internal_endpoint_system_add_project_to_user; (
            user_id: Principal,
            project_id: Principal,
        ) -> ();

        fn ping = protected_endpoint; () -> ();

        fn admin_repair = multi_role_endpoint, role = CanisterRole::new("admin_hub"); (
            project_id: Principal,
        ) -> ();
    }
}

const fn principal(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

#[test]
fn internal_client_macro_generates_typed_methods() {
    let client = ProjectHubInternalClient::new(principal(1))
        .with_bounded_wait()
        .with_cycles(10_000)
        .with_proof_ttl_secs(30);

    let _add_project = client.add_project(principal(2), principal(3));
    let _generated_add_project = client.generated_add_project(principal(2), principal(3));
    let _ping = client.ping();
    let _admin_repair = client.admin_repair(principal(4));
}

#[test]
fn protected_endpoint_macro_descriptor_is_client_compatible() {
    let descriptor = canic_internal_endpoint_system_add_project_to_user();

    assert_eq!(descriptor.method(), "wire_system_add_project_to_user");
    assert_eq!(
        descriptor.single_role(),
        Some(&CanisterRole::new("project_hub"))
    );
}

#[test]
fn internal_client_macro_accepts_shared_options() {
    let options = canic::api::ic::CanicInternalCallOptions::new()
        .with_bounded_wait()
        .with_cycles(1_000)
        .with_proof_ttl_secs(10);
    let client = ProjectHubInternalClient::new(principal(1)).with_options(options);

    let _ping = client.ping();
}
