//! Controlled Canic host with an optional empty, metrics-enabled IcyDB participant.
//! No provisioning fixture, entity, application query or application write is linked.

#![expect(
    clippy::unused_async,
    reason = "Canic lifecycle hooks require async signatures"
)]
#![cfg_attr(
    feature = "participant",
    expect(
        clippy::redundant_pub_crate,
        reason = "published IcyDB generates crate-visible bindings inside private actor modules"
    )
)]

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

canic::finish!();
