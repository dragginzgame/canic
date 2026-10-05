//! Map the service's maintained accounting into Canic's bounded public cache.
//!
//! No scans, remote calls, continuity mutations or independent accounting.

use crate::ops::HOST;
use canic::{
    diagnostics::codes,
    dto::{
        error::Error,
        public_status::{PublicMetric, PublicMetricKind},
    },
};

/// Read only this service's bounded metrics; the host composes the sole sampler.
pub fn sample() -> Result<Vec<PublicMetric>, Error> {
    HOST.with_borrow(|host| {
        let host = host
            .as_ref()
            .ok_or_else(|| Error::from_registered(codes::STORAGE_INACTIVE))?;
        let uploads = &host.installation.stores().uploads;
        let usage = uploads
            .usage()
            .map_err(|_| Error::from_registered(codes::STORAGE_INVALID_STATE))?;
        let observed_at_ns = ic_cdk::api::time();
        let canister_id = Some(ic_cdk::api::canister_self());
        Ok([
            (
                "blob.active_reservations",
                usage.active_reservations as u128,
                "count",
            ),
            ("blob.fenced", u128::from(uploads.is_fenced()), "boolean"),
            ("blob.liability_bytes", usage.liability_bytes, "bytes"),
            ("blob.logical_bytes", usage.logical_bytes, "bytes"),
            ("blob.operation_slots", usage.operations as u128, "count"),
            ("blob.physical_bytes", usage.physical_bytes, "bytes"),
            ("blob.reserved_bytes", usage.reserved_bytes, "bytes"),
        ]
        .into_iter()
        .map(|(name, value, unit)| PublicMetric {
            name: name.into(),
            canister_id,
            value,
            unit: unit.into(),
            observed_at_ns,
            kind: PublicMetricKind::Gauge,
        })
        .collect())
    })
}
