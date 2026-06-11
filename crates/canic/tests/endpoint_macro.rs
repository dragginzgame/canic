use canic::{Error, canic_query};

#[canic_query(composite)]
fn composite_probe() -> Result<(), Error> {
    Ok(())
}

canic::canic_emit_nonroot_auth_attestation_endpoints!();

#[test]
fn canic_query_accepts_composite_marker() {
    std::hint::black_box(composite_probe as fn() -> Result<(), Error>);
}

#[test]
fn nonroot_auth_emitter_exports_active_proof_installer() {
    std::hint::black_box(canic_install_active_delegation_proof);
}
