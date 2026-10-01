use canic::{Error, canic_query, canic_update};

const LIMITS: canic::endpoint::ArgumentLimits = canic::endpoint::ArgumentLimits {
    max_bytes: 1024,
    decoding_quota: 10_000,
    skipping_quota: 100,
    max_type_len: 32,
    max_header_len: 128,
};

#[canic_update(requires(caller::is_controller()), on_access_denied = "reject", decode = LIMITS)]
fn guarded_plain(value: u64) -> u64 {
    value
}

#[canic_query(public, composite, on_access_denied = "reject", decode = LIMITS)]
fn bounded_composite(value: u64) -> u64 {
    value
}

#[canic_query(requires(auth::authenticated()), decode = LIMITS)]
fn bounded_token(token: canic::dto::auth::DelegatedToken) -> Result<(), Error> {
    Ok(())
}

#[canic_update(requires(auth::attested_local_subnet()), decode = LIMITS)]
fn bounded_attestation(proof: canic::dto::auth::SignedRoleAttestation) -> Result<(), Error> {
    Ok(())
}

#[canic_query(public, payload(max_bytes = 1024))]
fn byte_bounded_query(value: u64) -> Result<u64, Error> {
    Ok(value)
}

#[canic_update(requires(caller::is_controller()), on_access_denied = "reject")]
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

#[canic_query(public, composite)]
fn composite_probe() -> Result<(), Error> {
    Ok(())
}

#[canic_update(requires(
    caller::is_fleet_admitted(),
    deployment::is_service_authority("database"),
))]
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
