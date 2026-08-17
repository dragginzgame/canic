use canic::{Error, canic_query, canic_update};

#[canic_query(public, composite)]
fn composite_probe() -> Result<(), Error> {
    Ok(())
}

#[canic_update(requires(caller::is_whitelisted(), deployment::is_service_authority("database"),))]
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
