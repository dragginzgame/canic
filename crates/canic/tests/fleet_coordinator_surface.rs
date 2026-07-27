#![cfg(feature = "fleet-coordinator-canister")]

canic::start_fleet_coordinator!();
canic::finish!();

#[test]
fn fleet_coordinator_lifecycle_and_endpoint_surface_compiles() {}
