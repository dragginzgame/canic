use crate::{Error, Result, supplier, unhex};
use ic_platform::{agent::Agent, principal::Principal};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Network and trust anchor are part of the approved plan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Network {
    /// IC mainnet, using the Agent's compiled mainnet trust root.
    Mainnet,
    /// Explicit local loopback replica and its pinned DER root key in hex.
    Local {
        /// HTTP loopback URL; remote hosts are refused.
        url: String,
        /// Root key obtained from the selected local replica.
        root_key: String,
        /// Existing local canister whose subnet routes provisional creation.
        routing_canister: String,
    },
}

impl Network {
    /// Digest of the exact deployment binding used in fleet evidence.
    ///
    /// Local bindings include the URL, pinned root key and routing principal.
    /// This preserves the existing fleet digest encoding; it is not a canonical
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
            routing_canister,
        } = self
        {
            principal(routing_canister)?;
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

    pub(crate) fn agent(&self) -> Result<Agent> {
        self.validate()?;
        let agent = Agent::builder()
            .with_url(self.url())
            .with_max_tcp_error_retries(0)
            .with_max_response_body_size(2 * 1024 * 1024)
            .with_max_polling_time(Duration::from_secs(60))
            .build()
            .map_err(supplier)?;
        if let Self::Local { root_key, .. } = self {
            agent.set_root_key(unhex(root_key)?);
        }
        Ok(agent)
    }
}

pub(crate) fn principal(text: &str) -> Result<Principal> {
    Principal::from_text(text).map_err(supplier)
}

pub(crate) async fn bounded<T>(
    future: impl std::future::Future<Output = std::result::Result<T, ic_platform::agent::AgentError>>,
) -> Result<T> {
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
    let bytes =
        ic_wire::native::encode_request(canic::GREET, &name, canic::LIMITS).map_err(supplier)?;
    let agent = network.agent()?;
    principal(canister)?;
    let reply = bounded(
        agent
            .query(&canister.parse().map_err(supplier)?, "greet")
            .with_arg(bytes)
            .call(),
    )
    .await?;
    ic_wire::native::decode_response(canic::GREET, &reply, canic::LIMITS).map_err(supplier)
}
