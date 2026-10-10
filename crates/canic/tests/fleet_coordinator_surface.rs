#![cfg(feature = "fleet-coordinator-canister")]

canic::start_fleet_coordinator!();
canic::finish!();
candid::export_service!();

#[test]
fn fleet_coordinator_lifecycle_and_endpoint_surface_compiles() {}

#[test]
fn generated_coordinator_candid_matches_canonical_contract() {
    candid_parser::utils::service_equal(
        candid_parser::utils::CandidSource::Text(&__export_service()),
        candid_parser::utils::CandidSource::Text(include_str!("../candid/fleet_coordinator.did")),
    )
    .expect("generated Coordinator service equals the checked-in contract");
}
