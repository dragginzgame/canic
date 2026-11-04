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

#[test]
fn compiled_coordinator_payload_contract_matches_canonical_metadata() {
    use canic::__internal::core::ingress::payload_contract::{
        CANDID_PAYLOAD_CONTRACT_PREFIX, annotate_candid,
    };
    let generated = annotate_candid(String::new());
    let metadata = |source: &str| {
        source
            .lines()
            .find_map(|line| line.strip_prefix(CANDID_PAYLOAD_CONTRACT_PREFIX))
            .expect("compiled payload metadata")
            .to_owned()
    };
    assert_eq!(
        metadata(&generated),
        metadata(include_str!("../candid/fleet_coordinator.did"))
    );
}
