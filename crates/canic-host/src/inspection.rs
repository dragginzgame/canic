//! Verified, read-only calls to the application-owned database inspection endpoint.

use crate::{Error, Network, Result, network, supplier};
use ic_platform::native::{Ed25519Identity, ReadClient, Secp256k1Identity};
use ic_platform_types::Principal;
use std::{io::Read, path::Path, time::Duration};

enum Caller {
    Anonymous,
    Ed25519(Ed25519Identity),
    Secp256k1(Secp256k1Identity),
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
        let (expiry, time) = network::context()?;
        let method = "db_inspect";
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
