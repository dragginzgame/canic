//! Local projection of ICP configuration used by Canic's build and readiness checks.
//!
//! Reads inline membership and network declarations without resolving recipes,
//! fetching dependencies, or executing build/sync instructions.

use crate::icp_config::DEFAULT_LOCAL_GATEWAY_PORT;
use canic_contracts::ids::BuildNetwork;
use serde::{Deserialize, de::IgnoredAny};
use std::collections::{BTreeMap, BTreeSet};

///
/// IcpManifestError
///
/// Exact local manifest rejection; presentation does not drive control flow.
///

#[derive(Debug, thiserror::Error)]
pub enum IcpManifestError {
    #[error("ICP configuration exceeds the 1 MiB input limit")]
    TooLarge,

    #[error("invalid ICP YAML: {0}")]
    Yaml(#[source] Box<serde_yaml_ng::Error>),

    #[error("Canic's local ICP inspection does not resolve project dependencies")]
    Dependencies,

    #[error("Canic's local ICP inspection requires inline {section}; external manifest: {path}")]
    ExternalManifest { section: &'static str, path: String },

    #[error("duplicate {section} name: {name}")]
    DuplicateName { section: &'static str, name: String },

    #[error("invalid {section} name: {name}")]
    InvalidName { section: &'static str, name: String },

    #[error("ICP environment is not declared: {environment}")]
    UnknownEnvironment { environment: String },

    #[error("ICP environment {environment} references undeclared network {network}")]
    UnknownNetwork {
        environment: String,
        network: String,
    },
    #[error("ICP network {network} has unsupported mode {mode}")]
    NetworkMode { network: String, mode: String },

    #[error("ICP environment {environment} names undeclared canister {canister}")]
    UnknownCanister {
        environment: String,
        canister: String,
    },
    #[error("ICP environment {environment} repeats canister {canister}")]
    DuplicateCanister {
        environment: String,
        canister: String,
    },
    #[error("ICP local gateway port must be nonzero")]
    GatewayPort,

    #[error("the reserved ICP mainnet network cannot be redeclared in Canic configuration")]
    MainnetOverride,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RawManifest {
    canisters: Vec<ManifestItem<Canister>>,
    networks: Vec<ManifestItem<Network>>,
    environments: Vec<ManifestItem<Environment>>,
    dependencies: Vec<IgnoredAny>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ManifestItem<T> {
    Inline(T),
    Path(String),
}

#[derive(Deserialize)]
struct Canister {
    name: String,
}

#[derive(Deserialize)]
struct Network {
    name: String,
    mode: String,
    gateway: Option<Gateway>,
}

#[derive(Deserialize)]
struct Gateway {
    port: Option<u16>,
}

#[derive(Deserialize)]
struct Environment {
    name: String,
    network: Option<String>,
    canisters: Option<Vec<String>>,
}

///
/// IcpManifest
///
/// Checked inline declaration membership; instructions remain owned by ICP.
///

pub(super) struct IcpManifest {
    canisters: BTreeMap<String, Canister>,
    networks: BTreeMap<String, Network>,
    environments: BTreeMap<String, Environment>,
}

impl IcpManifest {
    pub(super) fn parse(source: &str) -> Result<Self, IcpManifestError> {
        if source.len() > 1024 * 1024 {
            return Err(IcpManifestError::TooLarge);
        }
        let document =
            parse_yaml_document(source).map_err(|error| IcpManifestError::Yaml(Box::new(error)))?;
        let raw = serde_yaml_ng::from_value::<Option<RawManifest>>(document)
            .map_err(|error| IcpManifestError::Yaml(Box::new(error)))?
            .unwrap_or_default();
        if !raw.dependencies.is_empty() {
            return Err(IcpManifestError::Dependencies);
        }
        let manifest = Self {
            canisters: named_items(raw.canisters, "canisters", |item| &item.name)?,
            networks: named_items(raw.networks, "networks", |item| &item.name)?,
            environments: named_items(raw.environments, "environments", |item| &item.name)?,
        };
        if manifest.networks.contains_key("ic") {
            return Err(IcpManifestError::MainnetOverride);
        }
        for network in manifest.networks.values() {
            if !matches!(network.mode.as_str(), "managed" | "connected") {
                return Err(IcpManifestError::NetworkMode {
                    network: network.name.clone(),
                    mode: network.mode.clone(),
                });
            }
        }
        for environment in manifest.environments.values() {
            manifest.build_network(&environment.name)?;
            let mut seen = BTreeSet::new();
            for canister in environment.canisters.iter().flatten() {
                if !manifest.canisters.contains_key(canister) {
                    return Err(IcpManifestError::UnknownCanister {
                        environment: environment.name.clone(),
                        canister: canister.clone(),
                    });
                }
                if !seen.insert(canister) {
                    return Err(IcpManifestError::DuplicateCanister {
                        environment: environment.name.clone(),
                        canister: canister.clone(),
                    });
                }
            }
        }
        manifest.local_gateway_port()?;
        Ok(manifest)
    }

    pub(super) fn build_network(
        &self,
        environment: &str,
    ) -> Result<BuildNetwork, IcpManifestError> {
        let network = match self.environments.get(environment) {
            Some(item) => item.network.as_deref().unwrap_or("local"),
            None if matches!(environment, "local" | "ic") => environment,
            None => {
                return Err(IcpManifestError::UnknownEnvironment {
                    environment: environment.into(),
                });
            }
        };
        match network {
            "ic" => Ok(BuildNetwork::Ic),
            "local" => Ok(BuildNetwork::Local),
            _ if self.networks.contains_key(network) => Ok(BuildNetwork::Local),
            _ => Err(IcpManifestError::UnknownNetwork {
                environment: environment.into(),
                network: network.into(),
            }),
        }
    }

    pub(super) fn has_canister(&self, name: &str) -> bool {
        self.canisters.contains_key(name)
    }

    pub(super) fn has_environment(&self, name: &str) -> bool {
        matches!(name, "local" | "ic") || self.environments.contains_key(name)
    }

    pub(super) fn contains(&self, environment: &str, canister: &str) -> bool {
        if !self.has_environment(environment) || !self.has_canister(canister) {
            return false;
        }
        self.environments
            .get(environment)
            .and_then(|item| item.canisters.as_ref())
            .is_none_or(|selected| selected.iter().any(|name| name == canister))
    }

    pub(super) fn local_gateway_port(&self) -> Result<u16, IcpManifestError> {
        let port = self
            .networks
            .get("local")
            .and_then(|network| network.gateway.as_ref())
            .and_then(|gateway| gateway.port)
            .unwrap_or(DEFAULT_LOCAL_GATEWAY_PORT);
        if port == 0 {
            return Err(IcpManifestError::GatewayPort);
        }
        Ok(port)
    }
}

fn named_items<T>(
    items: Vec<ManifestItem<T>>,
    section: &'static str,
    name: impl Fn(&T) -> &str,
) -> Result<BTreeMap<String, T>, IcpManifestError> {
    let mut result = BTreeMap::new();
    for item in items {
        let item = match item {
            ManifestItem::Inline(item) => item,
            ManifestItem::Path(path) => {
                return Err(IcpManifestError::ExternalManifest { section, path });
            }
        };
        let key = name(&item).to_string();
        if key.is_empty()
            || !key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(IcpManifestError::InvalidName { section, name: key });
        }
        if result.insert(key.clone(), item).is_some() {
            return Err(IcpManifestError::DuplicateName { section, name: key });
        }
    }
    Ok(result)
}

// Inspect the entire document before schema projection: ignored ICP fields must
// not hide duplicate keys, merge keys or unsupported tags. The same parsed value
// supplies the typed projection; untagged manifest entries retain strict field types.
// Reserve the << key even when quoted: the parsed tree does not retain scalar style.
fn parse_yaml_document(source: &str) -> Result<serde_yaml_ng::Value, serde_yaml_ng::Error> {
    use serde::de::Error as _;
    use serde_yaml_ng::Value;

    // Value deserialization rejects duplicate mapping keys, including ignored fields.
    let document: Value = serde_yaml_ng::from_str(source)?;
    let mut pending = vec![&document];
    while let Some(value) = pending.pop() {
        match value {
            Value::Tagged(_) => {
                return Err(serde_yaml_ng::Error::custom("unsupported ICP YAML tag"));
            }
            Value::Mapping(mapping) => {
                for (key, value) in mapping {
                    if key.as_str() == Some("<<") {
                        return Err(serde_yaml_ng::Error::custom(
                            "ICP YAML merge keys are unsupported",
                        ));
                    }
                    pending.push(key);
                    pending.push(value);
                }
            }
            Value::Sequence(sequence) => pending.extend(sequence),
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }
    Ok(document)
}
