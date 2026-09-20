//! Controlled Canic composition with cumulative IcyDB binding/query/write subjects.

#![expect(
    clippy::unused_async,
    reason = "Canic lifecycle hooks require async signatures"
)]
#![expect(
    clippy::redundant_pub_crate,
    reason = "audit operations and generated IcyDB bindings retain crate-local visibility"
)]

mod operations;

use operations::AuditOutcome;

#[cfg(feature = "participant")]
canic::memory::ic_memory_range!(
    authority = "icydb.canic_composed_audit",
    start = 100,
    end = 106,
    mode = Allowed
);

#[cfg(feature = "participant")]
canic::memory::memory_bootstrap_admission!(
    identity = canic::memory::admission::PolicyIdentity::new("icydb.logical-memory-admission", 1)
        .expect("valid IcyDB admission identity"),
    prepare = icydb::db::prepare_memory_bootstrap,
);

#[cfg(feature = "participant")]
icydb::start!(participant);

#[cfg(feature = "participant")]
icydb::endpoints! {
    icydb_metrics(authorization = public);
    icydb_metrics_reset;
}

#[cfg(feature = "participant")]
canic::start!(lifecycle_participant(
    init = crate::__icydb_lifecycle_participant::init,
    post_upgrade = crate::__icydb_lifecycle_participant::post_upgrade,
),);

#[cfg(not(feature = "participant"))]
canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

// The wire adapters stay present in every subject, including the host control.
#[canic::canic_query(public)]
fn audit_binding(entity: u8) -> Result<AuditOutcome, canic::Error> {
    Ok(operations::binding(entity))
}

#[canic::canic_query(public)]
fn audit_page(entity: u8) -> Result<AuditOutcome, canic::Error> {
    Ok(operations::page(entity))
}

#[canic::canic_update(public)]
fn audit_insert(entity: u8, id: u64, value: u64) -> Result<AuditOutcome, canic::Error> {
    Ok(operations::insert(entity, id, value))
}

canic::finish!();
