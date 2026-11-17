use crate::{Error, Result, supplier, unhex};
use ic_platform::{
    certificate::{PreparedRootKey, TimePolicy, TimeWindow, VerificationLimits},
    native::ReadClient,
    native::ReadConfig,
};
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[cfg(test)]
mod tests;

/// Network and trust anchor are part of the approved plan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Network {
    /// IC mainnet, using the pinned mainnet trust root and root subnet.
    Mainnet,
    /// Explicit local loopback replica and its pinned DER root key in hex.
    Local {
        /// HTTP loopback URL; remote hosts are refused.
        url: String,
        /// Root key obtained from the selected local replica.
        root_key: String,
        /// Explicit root-subnet identity. Missing only in retained older bindings.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        root_subnet: Option<String>,
        /// Existing local canister whose subnet routes provisional creation.
        routing_canister: String,
    },
}

impl Network {
    /// Digest of the exact deployment binding used in fleet evidence.
    ///
    /// Local bindings include the URL, pinned root key, routing principal and,
    /// for newly admitted bindings, root-subnet identity. Absent historical
    /// identities remain absent in the digest encoding; it is not a canonical
    /// network identity shared by different gateway configurations.
    ///
    /// # Errors
    /// Refuses invalid network settings or failed serialization.
    pub fn binding_id(&self) -> Result<String> {
        self.validate()?;
        Ok(crate::artifact::digest(
            &serde_json::to_vec(self).map_err(supplier)?,
        ))
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if let Self::Local {
            url,
            root_key,
            root_subnet,
            routing_canister,
        } = self
        {
            principal(routing_canister)?;
            if let Some(root_subnet) = root_subnet {
                let id = principal(root_subnet)?;
                if id == Principal::anonymous() || id == Principal::management_canister() {
                    return Err(Error::Invalid(
                        "root subnet requires a subnet principal".into(),
                    ));
                }
            }
            let parsed = url::Url::parse(url).map_err(supplier)?;
            if parsed.scheme() != "http"
                || !matches!(parsed.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
                || !parsed.username().is_empty()
                || parsed.password().is_some()
                || parsed.query().is_some()
                || parsed.fragment().is_some()
                || parsed.path() != "/"
                || unhex(root_key)?.len() != 133
            {
                return Err(Error::Invalid(
                    "local network requires a loopback HTTP origin and 133-byte DER root key"
                        .into(),
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn url(&self) -> &str {
        match self {
            Self::Mainnet => "https://icp-api.io/",
            Self::Local { url, .. } => url,
        }
    }

    pub(crate) fn root(&self) -> &str {
        match self {
            Self::Mainnet => "mainnet",
            Self::Local { root_key, .. } => root_key,
        }
    }

    pub(crate) fn client(&self) -> Result<ReadClient> {
        self.validate()?;
        let (root, root_subnet) = match self {
            Self::Mainnet => (
                PreparedRootKey::from_der(include_bytes!("mainnet-root.der")).map_err(supplier)?,
                principal(MAINNET_ROOT_SUBNET)?,
            ),
            Self::Local {
                root_key,
                root_subnet,
                ..
            } => {
                let subnet = root_subnet.as_deref().ok_or_else(|| Error::Blocked(
                    "local network lacks an admitted root_subnet; retained approval is unchanged and no network request was sent".into()
                ))?;
                (
                    PreparedRootKey::from_der(&unhex(root_key)?).map_err(supplier)?,
                    principal(subnet)?,
                )
            }
        };
        ReadClient::new(ReadConfig {
            endpoint: self.url().into(),
            root,
            root_subnet,
            message_bytes: 2 * 1024 * 1024,
            request_timeout: Duration::from_secs(60),
            verification: VerificationLimits {
                tree_nodes: 16384,
                tree_depth: 64,
                delegation_bytes: 131072,
                canister_ranges: 4096,
            },
        })
        .map_err(supplier)
    }
}

pub(crate) fn principal(text: &str) -> Result<Principal> {
    Principal::from_text(text).map_err(supplier)
}

pub(crate) async fn bounded<T, E>(
    future: impl std::future::Future<Output = std::result::Result<T, E>>,
) -> Result<T>
where
    E: std::error::Error + Send + Sync + 'static,
{
    tokio::time::timeout(Duration::from_secs(60), future)
        .await
        .map_err(supplier)?
        .map_err(supplier)
}

/// Call the demo greeting as a read-only, uncertified query.
///
/// # Errors
/// Refuses oversized names, transport failures and responses outside the shared Wire contract.
pub async fn greet(network: &Network, canister: &str, name: &str) -> Result<canic::Greeting> {
    if name.len() > canic::MAX_NAME_BYTES {
        return Err(Error::Invalid("name exceeds 128 UTF-8 bytes".into()));
    }
    let bytes = ic_wire::encode(
        &name,
        ic_wire::Encoding::Frame {
            method: canic::GREET,
            direction: ic_wire::Direction::Request,
        },
        canic::LIMITS,
    )
    .map_err(supplier)?;
    let client = network.client()?;
    let reply = query(&client, principal(canister)?, "greet", &bytes).await?;
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: canic::GREET,
            direction: ic_wire::Direction::Response,
        },
        canic::LIMITS,
    )
    .map_err(supplier)
}

/// Certified subnet identity shared by bootstrap and artifact-delivery admission.
pub(crate) async fn subnet(client: &ReadClient, canister: &str) -> Result<Principal> {
    let (expiry, time) = context()?;
    Ok(
        bounded(client.read_canister(principal(canister)?, expiry, time, 10))
            .await?
            .subnet(),
    )
}

// Public mainnet trust inputs match the previously selected Agent and Topology.
pub(crate) const MAINNET_ROOT_SUBNET: &str =
    "tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe";

pub(crate) fn context() -> Result<(u64, TimePolicy)> {
    let now = crate::now()?;
    let expiry = now
        .checked_add(300_000_000_000)
        .ok_or_else(|| Error::Invalid("ingress expiry overflow".into()))?;
    let window = TimeWindow {
        reference_ns: now,
        max_age_ns: 300_000_000_000,
        // Preserve the former Agent's policy: reject stale certificates, but
        // permit certified future time (including explicit simulator advances).
        max_future_skew_ns: u64::MAX - now,
    };
    Ok((
        expiry,
        TimePolicy {
            certificate: window,
            delegation: window,
        },
    ))
}

pub(crate) async fn query(
    client: &ReadClient,
    canister: Principal,
    method: &str,
    args: &[u8],
) -> Result<Vec<u8>> {
    let (expiry, time) = context()?;
    bounded(client.query(canister, method, args, expiry, time)).await
}
