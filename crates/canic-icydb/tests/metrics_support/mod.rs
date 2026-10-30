//! Framework-owned consumer stand-ins; no database engine or storage backend.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod diagnostic {
    use serde::{Deserialize, Serialize};

    /// Consumer-owned authorization errors required by the mount contract.
    #[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub enum RuntimeBoundaryCode {
        SqlSurfaceControllerRequired,
        OperationalSurfaceControllerRequired,
        SchemaSurfaceControllerRequired,
    }
}

/// Consumer-owned error origin passed through by the adapter.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ErrorOrigin {
    Interface,
}

/// Minimal typed consumer failure; the adapter does not select its representation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Error {
    boundary: diagnostic::RuntimeBoundaryCode,
    origin: ErrorOrigin,
}

impl Error {
    pub const fn from_runtime_boundary(
        boundary: diagnostic::RuntimeBoundaryCode,
        origin: ErrorOrigin,
    ) -> Self {
        Self { boundary, origin }
    }
}

pub mod wire {
    pub const MAX_REQUEST_BYTES: usize = 4096;
    pub const MAX_REPLY_BYTES: usize = 4096;

    pub const fn limits(bytes: usize) -> ic_wire::Limits {
        ic_wire::Limits {
            bytes,
            // Wire accounts visitor values as well as payload allocation.
            allocation: 65536,
            nodes: 128,
            depth: 16,
        }
    }
}

pub mod metrics {
    use serde::{Deserialize, Serialize};

    /// Deliberately a fixture shape, not a copy of IcyDB's report schema.
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize, ic_wire::Wire)]
    pub struct MetricsReport {
        pub observation: ic_metrics::MeasurementCounter,
        pub instructions: ic_metrics::MeasurementAggregate,
    }
}

pub mod generated {
    pub const __ICYDB_START_BINDING: () = ();

    pub mod endpoint_handlers {
        use super::super::{Error, metrics::MetricsReport};

        #[expect(
            clippy::unnecessary_wraps,
            reason = "consumer handler has a fallible contract"
        )]
        pub fn metrics() -> Result<MetricsReport, Error> {
            Ok(MetricsReport {
                observation: ic_metrics::MeasurementCounter::from_parts(u128::MAX, false).unwrap(),
                instructions: ic_metrics::MeasurementAggregate::from_parts(
                    u64::MAX,
                    u128::MAX,
                    Some(u64::MAX),
                    true,
                    true,
                )
                .unwrap(),
            })
        }

        #[expect(
            clippy::unnecessary_wraps,
            reason = "consumer handler has a fallible contract"
        )]
        pub const fn metrics_reset() -> Result<(), Error> {
            Ok(())
        }
    }
}

/// Exercise native envelopes with consumer-owned success and typed failure shapes.
pub fn assert_consumer_wire_contract() {
    use canic_icydb::wire::Endpoint;
    let schema = ic_wire::typescript::native_schema::<metrics::MetricsReport>().unwrap();
    ic_wire::typescript::validate_payload_schema(&BTreeMap::new(), &schema).unwrap();
    let request_limits = wire::limits(wire::MAX_REQUEST_BYTES);
    let request = Endpoint::MetricsReset
        .encode_request((), request_limits)
        .unwrap();
    Endpoint::MetricsReset
        .decode_request::<()>(&request, request_limits)
        .unwrap();
    assert_eq!(generated::endpoint_handlers::metrics_reset(), Ok(()));
    let limits = wire::limits(wire::MAX_REPLY_BYTES);
    let response = generated::endpoint_handlers::metrics();
    let bytes = Endpoint::Metrics
        .encode_response(&response, limits)
        .unwrap();
    let decoded: Result<metrics::MetricsReport, Error> =
        Endpoint::Metrics.decode_response(&bytes, limits).unwrap();
    assert_eq!(decoded, response);
    assert_eq!(
        Endpoint::Metrics.decode_response::<Result<metrics::MetricsReport, Error>>(
            &bytes,
            ic_wire::Limits {
                allocation: 8192,
                ..limits
            },
        ),
        Err(ic_wire::Error::Limit),
    );
    assert_eq!(
        Endpoint::MetricsReset
            .decode_response::<Result<metrics::MetricsReport, Error>>(&bytes, limits),
        Err(ic_wire::Error::Identity),
    );

    let refused: Result<(), Error> = Err(Error::from_runtime_boundary(
        diagnostic::RuntimeBoundaryCode::OperationalSurfaceControllerRequired,
        ErrorOrigin::Interface,
    ));
    let bytes = Endpoint::MetricsReset
        .encode_response(refused, limits)
        .unwrap();
    assert_eq!(
        Endpoint::MetricsReset
            .decode_response::<Result<(), Error>>(&bytes, limits)
            .unwrap(),
        refused
    );
}
