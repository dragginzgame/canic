//! PocketIC-only fixture entrypoint; absent from the repair artifact.

canic::start_fleet_root!();

#[ic_cdk::update]
fn test_seed_canic188() -> Result<(), canic::Error> {
    assert!(ic_cdk::api::is_controller(&ic_cdk::api::msg_caller()));
    canic::__internal::control_plane::api::canister_pool::CanisterPoolApi::seed_canic188_fixture();
    Ok(())
}

canic::finish!();
