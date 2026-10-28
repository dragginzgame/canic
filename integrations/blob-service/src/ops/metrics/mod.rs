//! Map the service's maintained accounting into Canic's bounded public cache.
//!
//! No scans, remote calls or independent accounting. Observe the current platform
//! version through Blob's owner before reporting its recovery fence.

pub mod operations;
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
    HOST.with_borrow_mut(|host| {
        let host = host
            .as_mut()
            .ok_or_else(|| Error::from_registered(codes::STORAGE_INACTIVE))?;
        host.observe_platform_version(ic_platform::cdk::api::canister_version());
        let uploads = &host.stores().uploads;
        let usage = uploads
            .usage()
            .map_err(|_| Error::from_registered(codes::STORAGE_INVALID_STATE))?;
        let observed_at_ns = ic_platform::cdk::api::time();
        let canister_id = Some(ic_platform::cdk::api::canister_self());
        let mut metrics: Vec<PublicMetric> = [
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
        .collect();
        let observations = canic::api::metrics::providers::MetricProviderApi::snapshot("blob")
            .map_err(|_| Error::from_registered(codes::STORAGE_INVALID_STATE))?;
        let durable = observations.first().is_some_and(|row| {
            row.storage == canic::api::metrics::providers::MetricStorage::Stable
        });
        for row in observations {
            use canic::api::metrics::providers::ProviderMetricValue;
            let name = format!("{}.{}", row.provider, row.name);
            let (total, saturated) = match row.value {
                ProviderMetricValue::Counter(value) => (value.total(), value.total_overflowed()),
                ProviderMetricValue::Measurement(value) => {
                    metrics.push(PublicMetric {
                        name: format!("{name}.samples"),
                        canister_id,
                        value: u128::from(value.samples()),
                        unit: "events".into(),
                        observed_at_ns,
                        kind: PublicMetricKind::Counter {
                            window_id: row.window,
                            saturated: value.samples_overflowed(),
                        },
                    });
                    if let Some(maximum) = value.maximum() {
                        metrics.push(PublicMetric {
                            name: format!("{name}.maximum"),
                            canister_id,
                            value: u128::from(maximum),
                            unit: row.unit.into(),
                            observed_at_ns,
                            kind: PublicMetricKind::Gauge,
                        });
                    }
                    (value.total(), value.total_overflowed())
                }
            };
            metrics.push(PublicMetric {
                name,
                canister_id,
                value: total,
                unit: row.unit.into(),
                observed_at_ns,
                kind: PublicMetricKind::Counter {
                    window_id: row.window,
                    saturated,
                },
            });
        }
        metrics.push(PublicMetric {
            name: "blob.metrics.durable".into(),
            canister_id,
            value: u128::from(durable),
            unit: "boolean".into(),
            observed_at_ns,
            kind: PublicMetricKind::Gauge,
        });
        Ok(metrics)
    })
}
