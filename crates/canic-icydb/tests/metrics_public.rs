//! Compile real public-read/reset mounts against a framework-owned consumer.

mod metrics_support;

use canic_icydb::__private::cdk as __icydb_host_sdk;
use metrics_support::{self as __icydb_facade, generated as __icydb_generated};

canic_icydb::mount! {
    icydb_metrics(authorization = public);
    icydb_metrics_reset;
}

#[test]
fn public_metrics_mount_preserves_consumer_wire_types() {
    // Linking checks actual exports; invoking IC entrypoints requires PocketIC.
    let _: fn() = __icydb_export_icydb_metrics;
    let _: fn() = __icydb_export_icydb_metrics_reset;
    metrics_support::assert_consumer_wire_contract();
}
