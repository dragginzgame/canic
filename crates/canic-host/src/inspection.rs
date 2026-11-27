//! Verified inspection queries using the operator’s existing application permissions.

use crate::{Error, Network, Result, network, supplier};
use ic_platform::native::{Ed25519Identity, ReadClient, Secp256k1Identity};
use ic_platform_types::Principal;
use std::{io::Read, path::Path, time::Duration};

enum Caller {
    Anonymous,
    Ed25519(Ed25519Identity),
    Secp256k1(Secp256k1Identity),
}

/// Result of a single live observation under the selected identity.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Observation<T> {
    /// Decoded reply.
    Reported(T),
    /// Individual refusal, timeout or unavailable endpoint.
    ObservationError(String),
}

impl<T> From<Result<T>> for Observation<T> {
    fn from(result: Result<T>) -> Self {
        match result {
            Ok(value) => Self::Reported(value),
            Err(error) => Self::ObservationError(error.to_string()),
        }
    }
}

/// Each field is observed independently at the chosen canister.
#[derive(Debug, serde::Serialize)]
pub struct Diagnostics {
    /// Application's current admission state.
    pub readiness: Observation<canic::readiness::State>,
    /// First page of controller-accessible log entries.
    pub logs: Observation<canic::logging::Page>,
    /// First page of endpoint counters.
    pub metrics: Observation<canic::metrics::Report>,
}

/// One explicit identity, target and fleet trust anchor for inspection queries.
pub struct InspectionClient {
    client: ReadClient,
    target: Principal,
    caller: Caller,
}

impl InspectionClient {
    /// Admit the target, network and optional PEM before issuing a request.
    /// # Errors
    /// Refuses invalid network bindings, principals and unreadable or invalid identities.
    pub fn new(network: &Network, target: &str, pem: Option<&Path>) -> Result<Self> {
        let target = network::principal(target)?;
        let caller = if let Some(path) = pem {
            const LIMIT: usize = 4096;
            let mut text = String::new();
            std::fs::File::open(path)
                .map_err(supplier)?
                .take(LIMIT as u64 + 1)
                .read_to_string(&mut text)
                .map_err(supplier)?;
            match Ed25519Identity::from_pem(&text, LIMIT) {
                Ok(identity) => Caller::Ed25519(identity),
                Err(_) => {
                    Caller::Secp256k1(Secp256k1Identity::from_pem(&text, LIMIT).map_err(|_| {
                        Error::Invalid(
                            "expected a bounded Ed25519 or secp256k1 PEM identity".into(),
                        )
                    })?)
                }
            }
        } else {
            Caller::Anonymous
        };
        Ok(Self {
            client: network.client()?,
            caller,
            target,
        })
    }

    /// Principal whose application permissions govern the response.
    #[must_use]
    pub fn caller(&self) -> Principal {
        match &self.caller {
            Caller::Anonymous => Principal::anonymous(),
            Caller::Ed25519(id) => id.principal(),
            Caller::Secp256k1(id) => id.principal(),
        }
    }

    /// Execute one verified inspection query within the caller's deadline.
    /// # Errors
    /// Propagates deadline, transport and signature-verification failures.
    pub async fn inspect(&self, payload: &[u8], timeout: Duration) -> Result<Vec<u8>> {
        self.query("canic_db_inspect", payload, timeout).await
    }

    async fn readiness(&self) -> Result<canic::readiness::State> {
        use canic::readiness::{METHOD, REQUEST_LIMITS, RESPONSE_LIMITS};
        let frame = |direction| ic_wire::Encoding::Frame {
            method: METHOD,
            direction,
        };
        let request = ic_wire::encode(&(), frame(ic_wire::Direction::Request), REQUEST_LIMITS)
            .map_err(supplier)?;
        let reply = self
            .query("canic_readiness", &request, Duration::from_secs(3))
            .await?;
        ic_wire::decode(&reply, frame(ic_wire::Direction::Response), RESPONSE_LIMITS)
            .map_err(supplier)
    }

    pub(crate) async fn runtime(&self) -> crate::fleet::Availability {
        use crate::fleet::Availability;
        use canic::metrics::{ReadMetrics, Report, Request};
        let observation = self.read::<ReadMetrics>(&Request::Runtime).await;
        match observation {
            Ok(Report::Runtime {
                version,
                liquid_cycles,
            }) => Availability::Running {
                version,
                liquid_cycles: liquid_cycles.map(|n| n.to_string()),
            },
            Ok(_) => Availability::Unknown("unexpected runtime report".into()),
            Err(err) => {
                if let Error::Supplier(cause) = &err
                    && let Some(ic_platform::native::ReadError::Rejected {
                        error_code: Some(code),
                        ..
                    }) = cause.downcast_ref()
                {
                    match code.as_str() {
                        "IC0508" => return Availability::Stopped,
                        "IC0509" => return Availability::Stopping,
                        _ => {}
                    }
                }
                match self.readiness().await {
                    Ok(readiness) => Availability::Responding {
                        readiness,
                        metrics_error: err.to_string(),
                    },
                    Err(readiness) => {
                        Availability::Unknown(format!("metrics: {err}; readiness: {readiness}"))
                    }
                }
            }
        }
    }

    async fn read<M: canic::method::Contract>(&self, input: &M::Input) -> Result<M::Output> {
        let args = M::encode_request(input).map_err(supplier)?;
        let bytes = self.query(M::NAME, &args, Duration::from_secs(3)).await?;
        M::decode_reply(&bytes).map_err(supplier)
    }

    /// Observe current application readiness and one page of endpoint metrics and logs.
    pub async fn diagnostics(&self) -> Diagnostics {
        let (metrics, logs, readiness) = tokio::join!(
            self.read::<canic::metrics::ReadMetrics>(&canic::metrics::Request::Endpoints(None)),
            self.read::<canic::logging::Logs>(&None),
            self.readiness(),
        );
        Diagnostics {
            metrics: metrics.into(),
            logs: logs.into(),
            readiness: readiness.into(),
        }
    }

    pub(crate) async fn query(
        &self,
        method: &str,
        payload: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>> {
        let (expiry, time) = network::context()?;
        tokio::time::timeout(timeout, async {
            match &self.caller {
                Caller::Anonymous => {
                    self.client
                        .query(self.target, method, payload, expiry, time)
                        .await
                }
                Caller::Ed25519(id) => {
                    self.client
                        .signed_query(id, self.target, method, payload, expiry, time)
                        .await
                }
                Caller::Secp256k1(id) => {
                    self.client
                        .signed_query_secp256k1(id, self.target, method, payload, expiry, time)
                        .await
                }
            }
        })
        .await
        .map_err(|_| Error::Blocked("inspection query deadline exceeded".into()))?
        .map_err(supplier)
    }
}
