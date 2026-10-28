use canic::{Error, candid_query, candid_update, query, update};

const WIRE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 2_097_152,
    nodes: 65_536,
    depth: 32,
};

const LIMITS: canic::endpoint::ArgumentLimits = canic::endpoint::ArgumentLimits {
    max_bytes: 1024,
    decoding_quota: 10_000,
    skipping_quota: 100,
    max_type_len: 32,
    max_header_len: 128,
};

#[candid_update(requires(caller::is_controller()), on_access_denied = "reject", decode = LIMITS)]
fn guarded_plain(value: u64) -> u64 {
    value
}

#[candid_query(public, composite, on_access_denied = "reject", decode = LIMITS)]
fn bounded_composite(value: u64) -> u64 {
    value
}

#[query(requires(auth::attested_local_subnet()), wire(id = 0x5445_5354_0000_0806, request = WIRE_LIMITS, response = WIRE_LIMITS))]
fn bounded_token(token: canic::dto::role_attestation::SignedRoleAttestation) -> Result<(), Error> {
    Ok(())
}

#[update(requires(auth::attested_local_subnet()), wire(id = 0x5445_5354_0000_0807, request = WIRE_LIMITS, response = WIRE_LIMITS))]
fn bounded_attestation(
    proof: canic::dto::role_attestation::SignedRoleAttestation,
) -> Result<(), Error> {
    Ok(())
}

#[query(public, wire(id = 0x5445_5354_0000_0808, request = WIRE_LIMITS, response = WIRE_LIMITS))]
fn byte_bounded_query(value: u64) -> Result<u64, Error> {
    Ok(value)
}

#[candid_update(requires(caller::is_controller()), on_access_denied = "reject")]
fn plain_without_decoder(value: u64) -> u64 {
    value
}

#[test]
fn bounded_and_plain_endpoint_forms_compile() {
    std::hint::black_box(guarded_plain);
    std::hint::black_box(bounded_composite);
    std::hint::black_box(bounded_token);
    std::hint::black_box(bounded_attestation);
    std::hint::black_box(byte_bounded_query);
    std::hint::black_box(plain_without_decoder);
}

#[query(public, composite, wire(id = 0x5445_5354_0000_0809, request = WIRE_LIMITS, response = WIRE_LIMITS))]
fn composite_probe() -> Result<(), Error> {
    Ok(())
}

#[update(requires(
    caller::is_fleet_admitted(),
    deployment::is_service_authority("database"),
), wire(id = 0x5445_5354_0000_080a, request = WIRE_LIMITS, response = WIRE_LIMITS))]
async fn service_authority_probe() -> Result<(), Error> {
    std::future::ready(()).await;
    Ok(())
}

#[test]
fn canic_query_accepts_composite_marker() {
    std::hint::black_box(composite_probe as fn() -> Result<(), Error>);
}

#[test]
fn canic_update_accepts_protected_service_authority_guard() {
    std::hint::black_box(service_authority_probe);
}
