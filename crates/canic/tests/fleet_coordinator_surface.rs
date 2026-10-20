#![cfg(feature = "fleet-coordinator-canister")]

canic::start_fleet_coordinator!();
canic::finish!();
ic_platform::candid::contract::export_service!(ic_platform::candid::contract);

#[test]
fn fleet_coordinator_lifecycle_and_endpoint_surface_compiles() {}

#[test]
fn generated_coordinator_candid_matches_canonical_contract() {
    ic_platform::candid_parser::service_equal(
        ic_platform::candid_parser::CandidSource::Text(&__export_service()),
        ic_platform::candid_parser::CandidSource::Text(include_str!(
            "../candid/fleet_coordinator.did"
        )),
    )
    .expect("generated Coordinator service equals the checked-in contract");
}
