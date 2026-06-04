//! Live mainnet IC NNS registry adapter for Canic host tools.

pub(crate) mod proto;

use candid::{CandidType, Decode, Deserialize, Encode, Principal};
use canic_subnet_catalog::{
    CATALOG_SCHEMA_VERSION, CatalogError, ClassificationSource, GeographicScope, MAINNET_NETWORK,
    MAINNET_REGISTRY_CANISTER_ID, RoutingRange, SubnetCatalog, SubnetInfo, SubnetKind,
    SubnetSpecialization,
};
use futures::{StreamExt, TryStreamExt, stream};
use ic_agent::Agent;
use prost::Message;
use proto::{
    CanisterId, LargeValueChunkKeys, NodeOperatorRecord, NodeRecord, RegistryErrorCode,
    RegistryGetLatestVersionResponse, RegistryGetValueRequest, RegistryGetValueResponse,
    RoutingTable, SubnetId, SubnetListRecord, SubnetRecord, SubnetType, UInt64Value,
    registry_get_value_response,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error as ThisError;

pub const DEFAULT_MAINNET_ENDPOINT: &str = "https://icp-api.io";
pub const MAINNET_GOVERNANCE_CANISTER_ID: &str = "rrkah-fqaaa-aaaaa-aaaaq-cai";

const SUBNET_LIST_KEY: &str = "subnet_list";
const ROUTING_TABLE_KEY: &str = "routing_table";
const SUBNET_RECORD_KEY_PREFIX: &str = "subnet_record_";
const NODE_RECORD_KEY_PREFIX: &str = "node_record_";
const NODE_OPERATOR_RECORD_KEY_PREFIX: &str = "node_operator_record_";
const NODE_PROVIDER_ENRICHMENT_CONCURRENCY: usize = 32;
const FIDUCIARY_SUBNET: &str = "pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae";
const EUROPEAN_SUBNET: &str = "bkfrj-6k62g-dycql-7h53p-atvkj-zg4to-gaogh-netha-ptybj-ntsgw-rqe";

///
/// MainnetRegistryFetchRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MainnetRegistryFetchRequest {
    pub endpoint: String,
    pub fetched_at: String,
    pub fetched_by: String,
}

impl MainnetRegistryFetchRequest {
    #[must_use]
    pub fn new(fetched_at: String) -> Self {
        Self {
            endpoint: DEFAULT_MAINNET_ENDPOINT.to_string(),
            fetched_at,
            fetched_by: "canic-ic-registry".to_string(),
        }
    }
}

///
/// MainnetRegistryVersion
///
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MainnetRegistryVersion {
    pub network: String,
    pub registry_canister_id: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub fetched_by: String,
    pub source_endpoint: String,
}

///
/// MainnetNodeProviderList
///
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MainnetNodeProviderList {
    pub network: String,
    pub governance_canister_id: String,
    pub registry_canister_id: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub fetched_by: String,
    pub source_endpoint: String,
    pub node_providers: Vec<MainnetNodeProvider>,
}

///
/// MainnetNodeProvider
///
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MainnetNodeProvider {
    pub principal: String,
    pub node_count: Option<u32>,
    pub reward_account_hex: Option<String>,
}

///
/// MainnetNodeOperatorList
///
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MainnetNodeOperatorList {
    pub network: String,
    pub registry_canister_id: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub fetched_by: String,
    pub source_endpoint: String,
    pub node_operators: Vec<MainnetNodeOperator>,
}

///
/// MainnetNodeOperator
///
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MainnetNodeOperator {
    pub principal: String,
    pub node_provider_principal: String,
    pub node_allowance: u64,
    pub data_center_id: String,
    pub node_count: Option<u32>,
}

///
/// RegistryFetchError
///
#[derive(Debug, ThisError)]
pub enum RegistryFetchError {
    #[error("failed to build IC agent for {endpoint}: {reason}")]
    AgentBuild { endpoint: String, reason: String },

    #[error("registry agent call {method} failed: {reason}")]
    AgentCall {
        method: &'static str,
        reason: String,
    },

    #[error("failed to encode protobuf {message}: {reason}")]
    ProtobufEncode {
        message: &'static str,
        reason: String,
    },

    #[error("failed to decode protobuf {message}: {reason}")]
    ProtobufDecode {
        message: &'static str,
        reason: String,
    },

    #[error("registry get_value for key {key} failed with code {code}: {reason}")]
    RegistryValue {
        key: String,
        code: String,
        reason: String,
    },

    #[error("registry get_value for key {key} returned no value content")]
    MissingValue { key: String },

    #[error("failed to encode candid {message}: {reason}")]
    CandidEncode {
        message: &'static str,
        reason: String,
    },

    #[error("failed to decode candid {message}: {reason}")]
    CandidDecode {
        message: &'static str,
        reason: String,
    },

    #[error("registry get_chunk for sha256 {sha256} failed: {reason}")]
    RegistryChunkRejected { sha256: String, reason: String },

    #[error("registry get_chunk for sha256 {sha256} returned no chunk content")]
    MissingChunkContent { sha256: String },

    #[error("registry get_chunk for sha256 {sha256} returned content with sha256 {actual_sha256}")]
    ChunkHashMismatch {
        sha256: String,
        actual_sha256: String,
    },

    #[error("registry protobuf field {field} was missing")]
    MissingField { field: &'static str },

    #[error("registry principal field {field} is invalid: {reason}")]
    InvalidPrincipal { field: &'static str, reason: String },

    #[error("registry subnet list was empty")]
    EmptySubnetList,

    #[error("registry routing table was empty")]
    EmptyRoutingTable,

    #[error(transparent)]
    Catalog(#[from] CatalogError),

    #[error("failed to create Tokio runtime for registry refresh: {0}")]
    Runtime(String),
}

///
/// RegistryValueContent
///
#[derive(Debug)]
enum RegistryValueContent {
    Value(Vec<u8>),
    LargeValueChunkKeys(LargeValueChunkKeys),
}

///
/// RegistryGetChunkRequest
///
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct RegistryGetChunkRequest {
    content_sha256: Option<Vec<u8>>,
}

///
/// RegistryChunk
///
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct RegistryChunk {
    content: Option<Vec<u8>>,
}

///
/// ListNodeProvidersResponse
///
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct ListNodeProvidersResponse {
    node_providers: Vec<GovernanceNodeProvider>,
}

///
/// GovernanceNodeProvider
///
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct GovernanceNodeProvider {
    id: Option<Principal>,
    reward_account: Option<GovernanceAccountIdentifier>,
}

///
/// GovernanceAccountIdentifier
///
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct GovernanceAccountIdentifier {
    hash: Vec<u8>,
}

pub fn fetch_mainnet_subnet_catalog(
    request: &MainnetRegistryFetchRequest,
) -> Result<SubnetCatalog, RegistryFetchError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| RegistryFetchError::Runtime(err.to_string()))?;
    runtime.block_on(fetch_mainnet_subnet_catalog_async(request))
}

pub fn fetch_mainnet_registry_version(
    request: &MainnetRegistryFetchRequest,
) -> Result<MainnetRegistryVersion, RegistryFetchError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| RegistryFetchError::Runtime(err.to_string()))?;
    runtime.block_on(fetch_mainnet_registry_version_async(request))
}

pub fn fetch_mainnet_node_provider_list(
    request: &MainnetRegistryFetchRequest,
) -> Result<MainnetNodeProviderList, RegistryFetchError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| RegistryFetchError::Runtime(err.to_string()))?;
    runtime.block_on(fetch_mainnet_node_provider_list_async(request))
}

pub fn fetch_mainnet_node_operator_list(
    request: &MainnetRegistryFetchRequest,
) -> Result<MainnetNodeOperatorList, RegistryFetchError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| RegistryFetchError::Runtime(err.to_string()))?;
    runtime.block_on(fetch_mainnet_node_operator_list_async(request))
}

pub async fn fetch_mainnet_registry_version_async(
    request: &MainnetRegistryFetchRequest,
) -> Result<MainnetRegistryVersion, RegistryFetchError> {
    let agent = Agent::builder()
        .with_url(&request.endpoint)
        .build()
        .map_err(|err| RegistryFetchError::AgentBuild {
            endpoint: request.endpoint.clone(),
            reason: err.to_string(),
        })?;
    let registry_canister = Principal::from_text(MAINNET_REGISTRY_CANISTER_ID).map_err(|err| {
        RegistryFetchError::InvalidPrincipal {
            field: "registry_canister_id",
            reason: err.to_string(),
        }
    })?;
    let registry_version = get_latest_version(&agent, &registry_canister).await?;
    Ok(MainnetRegistryVersion {
        network: MAINNET_NETWORK.to_string(),
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version,
        fetched_at: request.fetched_at.clone(),
        fetched_by: request.fetched_by.clone(),
        source_endpoint: request.endpoint.clone(),
    })
}

pub async fn fetch_mainnet_subnet_catalog_async(
    request: &MainnetRegistryFetchRequest,
) -> Result<SubnetCatalog, RegistryFetchError> {
    let agent = Agent::builder()
        .with_url(&request.endpoint)
        .build()
        .map_err(|err| RegistryFetchError::AgentBuild {
            endpoint: request.endpoint.clone(),
            reason: err.to_string(),
        })?;
    let registry_canister = Principal::from_text(MAINNET_REGISTRY_CANISTER_ID).map_err(|err| {
        RegistryFetchError::InvalidPrincipal {
            field: "registry_canister_id",
            reason: err.to_string(),
        }
    })?;
    let registry_version = get_latest_version(&agent, &registry_canister).await?;
    let subnet_list_bytes = get_registry_value(
        &agent,
        &registry_canister,
        SUBNET_LIST_KEY,
        registry_version,
    )
    .await?;
    let routing_table_bytes = get_registry_value(
        &agent,
        &registry_canister,
        ROUTING_TABLE_KEY,
        registry_version,
    )
    .await?;
    let subnet_list = decode_message::<SubnetListRecord>("SubnetListRecord", &subnet_list_bytes)?;
    let routing_table = decode_message::<RoutingTable>("RoutingTable", &routing_table_bytes)?;
    catalog_from_registry_records(
        request,
        registry_version,
        &agent,
        &registry_canister,
        subnet_list,
        routing_table,
    )
    .await
}

pub async fn fetch_mainnet_node_provider_list_async(
    request: &MainnetRegistryFetchRequest,
) -> Result<MainnetNodeProviderList, RegistryFetchError> {
    let agent = Agent::builder()
        .with_url(&request.endpoint)
        .build()
        .map_err(|err| RegistryFetchError::AgentBuild {
            endpoint: request.endpoint.clone(),
            reason: err.to_string(),
        })?;
    let governance_canister =
        Principal::from_text(MAINNET_GOVERNANCE_CANISTER_ID).map_err(|err| {
            RegistryFetchError::InvalidPrincipal {
                field: "governance_canister_id",
                reason: err.to_string(),
            }
        })?;
    let arg = Encode!().map_err(|err| RegistryFetchError::CandidEncode {
        message: "list_node_providers",
        reason: err.to_string(),
    })?;
    let bytes = agent
        .query(&governance_canister, "list_node_providers")
        .with_arg(arg)
        .call()
        .await
        .map_err(|err| RegistryFetchError::AgentCall {
            method: "list_node_providers",
            reason: err.to_string(),
        })?;
    let response = Decode!(&bytes, ListNodeProvidersResponse).map_err(|err| {
        RegistryFetchError::CandidDecode {
            message: "ListNodeProvidersResponse",
            reason: err.to_string(),
        }
    })?;
    let registry_canister = Principal::from_text(MAINNET_REGISTRY_CANISTER_ID).map_err(|err| {
        RegistryFetchError::InvalidPrincipal {
            field: "registry_canister_id",
            reason: err.to_string(),
        }
    })?;
    let registry_version = get_latest_version(&agent, &registry_canister).await?;
    let node_counts =
        fetch_node_provider_node_counts(&agent, &registry_canister, registry_version).await?;
    node_provider_list_from_response(request, response, node_counts, registry_version)
}

pub async fn fetch_mainnet_node_operator_list_async(
    request: &MainnetRegistryFetchRequest,
) -> Result<MainnetNodeOperatorList, RegistryFetchError> {
    let agent = Agent::builder()
        .with_url(&request.endpoint)
        .build()
        .map_err(|err| RegistryFetchError::AgentBuild {
            endpoint: request.endpoint.clone(),
            reason: err.to_string(),
        })?;
    let registry_canister = Principal::from_text(MAINNET_REGISTRY_CANISTER_ID).map_err(|err| {
        RegistryFetchError::InvalidPrincipal {
            field: "registry_canister_id",
            reason: err.to_string(),
        }
    })?;
    let registry_version = get_latest_version(&agent, &registry_canister).await?;
    let inventory =
        fetch_node_operator_inventory(&agent, &registry_canister, registry_version).await?;
    node_operator_list_from_inventory(request, inventory, registry_version)
}

async fn catalog_from_registry_records(
    request: &MainnetRegistryFetchRequest,
    registry_version: u64,
    agent: &Agent,
    registry_canister: &Principal,
    subnet_list: SubnetListRecord,
    routing_table: RoutingTable,
) -> Result<SubnetCatalog, RegistryFetchError> {
    if subnet_list.subnets.is_empty() {
        return Err(RegistryFetchError::EmptySubnetList);
    }
    if routing_table.entries.is_empty() {
        return Err(RegistryFetchError::EmptyRoutingTable);
    }

    let mut subnets = Vec::with_capacity(subnet_list.subnets.len());
    for subnet_raw in subnet_list.subnets {
        let subnet_principal = principal_text_from_raw(&subnet_raw, "subnet_list.subnets")?;
        let key = subnet_record_key(&subnet_principal);
        let record_bytes =
            get_registry_value(agent, registry_canister, &key, registry_version).await?;
        let record = decode_message::<SubnetRecord>("SubnetRecord", &record_bytes)?;
        subnets.push(subnet_info_from_record(&subnet_principal, &record));
    }

    subnets.sort_by(|left, right| left.subnet_principal.cmp(&right.subnet_principal));

    let mut routing_ranges = routing_ranges_from_table(&routing_table)?;
    routing_ranges.sort_by(|left, right| {
        left.start_canister_id
            .cmp(&right.start_canister_id)
            .then_with(|| left.end_canister_id.cmp(&right.end_canister_id))
            .then_with(|| left.subnet_principal.cmp(&right.subnet_principal))
    });

    let mut catalog = SubnetCatalog {
        catalog_schema_version: CATALOG_SCHEMA_VERSION,
        network: MAINNET_NETWORK.to_string(),
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version,
        fetched_at: request.fetched_at.clone(),
        fetched_by: request.fetched_by.clone(),
        source_endpoint: request.endpoint.clone(),
        resolver_backend: "local-nns-subnet-catalog".to_string(),
        subnets,
        routing_ranges,
    };
    apply_mainnet_annotations(&mut catalog);
    catalog.validate()?;
    Ok(catalog)
}

fn node_provider_list_from_response(
    request: &MainnetRegistryFetchRequest,
    response: ListNodeProvidersResponse,
    node_counts: BTreeMap<String, u32>,
    registry_version: u64,
) -> Result<MainnetNodeProviderList, RegistryFetchError> {
    let mut node_providers = response
        .node_providers
        .into_iter()
        .map(|node_provider| node_provider_from_governance(node_provider, &node_counts))
        .collect::<Result<Vec<_>, _>>()?;
    node_providers.sort_by(|left, right| left.principal.cmp(&right.principal));
    Ok(MainnetNodeProviderList {
        network: MAINNET_NETWORK.to_string(),
        governance_canister_id: MAINNET_GOVERNANCE_CANISTER_ID.to_string(),
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version,
        fetched_at: request.fetched_at.clone(),
        fetched_by: request.fetched_by.clone(),
        source_endpoint: request.endpoint.clone(),
        node_providers,
    })
}

fn node_provider_from_governance(
    node_provider: GovernanceNodeProvider,
    node_counts: &BTreeMap<String, u32>,
) -> Result<MainnetNodeProvider, RegistryFetchError> {
    let principal = node_provider
        .id
        .ok_or(RegistryFetchError::MissingField {
            field: "node_provider.id",
        })?
        .to_text();
    let reward_account_hex = node_provider
        .reward_account
        .map(|account| hex_bytes(&account.hash));
    let node_count = Some(node_counts.get(&principal).copied().unwrap_or(0));
    Ok(MainnetNodeProvider {
        principal,
        node_count,
        reward_account_hex,
    })
}

async fn fetch_node_provider_node_counts(
    agent: &Agent,
    registry_canister: &Principal,
    registry_version: u64,
) -> Result<BTreeMap<String, u32>, RegistryFetchError> {
    let inventory =
        fetch_node_operator_inventory(agent, registry_canister, registry_version).await?;
    node_provider_counts_from_records(
        &inventory.node_principals,
        &inventory.node_records,
        &inventory.node_operator_records,
    )
}

async fn fetch_node_operator_inventory(
    agent: &Agent,
    registry_canister: &Principal,
    registry_version: u64,
) -> Result<NodeOperatorInventory, RegistryFetchError> {
    let subnet_list_bytes =
        get_registry_value(agent, registry_canister, SUBNET_LIST_KEY, registry_version).await?;
    let subnet_list = decode_message::<SubnetListRecord>("SubnetListRecord", &subnet_list_bytes)?;
    if subnet_list.subnets.is_empty() {
        return Err(RegistryFetchError::EmptySubnetList);
    }

    let subnet_principals = subnet_list
        .subnets
        .iter()
        .map(|subnet_raw| principal_text_from_raw(subnet_raw, "subnet_list.subnets"))
        .collect::<Result<Vec<_>, _>>()?;
    let subnet_records = stream::iter(subnet_principals)
        .map(|subnet_principal| async move {
            let key = subnet_record_key(&subnet_principal);
            let record_bytes =
                get_registry_value(agent, registry_canister, &key, registry_version).await?;
            decode_message::<SubnetRecord>("SubnetRecord", &record_bytes)
        })
        .buffer_unordered(NODE_PROVIDER_ENRICHMENT_CONCURRENCY)
        .try_collect::<Vec<_>>()
        .await?;

    let node_principals = assigned_node_principals_from_subnets(&subnet_records)?;
    let node_records = stream::iter(node_principals.iter().cloned())
        .map(|node_principal| async move {
            let key = node_record_key(&node_principal);
            let record_bytes =
                get_registry_value(agent, registry_canister, &key, registry_version).await?;
            let record = decode_message::<NodeRecord>("NodeRecord", &record_bytes)?;
            Ok::<_, RegistryFetchError>((node_principal, record))
        })
        .buffer_unordered(NODE_PROVIDER_ENRICHMENT_CONCURRENCY)
        .try_collect::<BTreeMap<_, _>>()
        .await?;

    let mut node_operator_principals = BTreeSet::new();
    for record in node_records.values() {
        node_operator_principals.insert(principal_text_from_required_raw(
            &record.node_operator_id,
            "node_record.node_operator_id",
        )?);
    }

    let node_operator_records = stream::iter(node_operator_principals)
        .map(|node_operator_principal| async move {
            let key = node_operator_record_key(&node_operator_principal);
            let record_bytes =
                get_registry_value(agent, registry_canister, &key, registry_version).await?;
            let record = decode_message::<NodeOperatorRecord>("NodeOperatorRecord", &record_bytes)?;
            Ok::<_, RegistryFetchError>((node_operator_principal, record))
        })
        .buffer_unordered(NODE_PROVIDER_ENRICHMENT_CONCURRENCY)
        .try_collect::<BTreeMap<_, _>>()
        .await?;

    Ok(NodeOperatorInventory {
        node_principals,
        node_records,
        node_operator_records,
    })
}

fn node_operator_list_from_inventory(
    request: &MainnetRegistryFetchRequest,
    inventory: NodeOperatorInventory,
    registry_version: u64,
) -> Result<MainnetNodeOperatorList, RegistryFetchError> {
    let node_counts =
        node_operator_counts_from_records(&inventory.node_principals, &inventory.node_records)?;
    let mut node_operators = inventory
        .node_operator_records
        .into_iter()
        .map(|(principal, record)| node_operator_from_record(principal, record, &node_counts))
        .collect::<Result<Vec<_>, _>>()?;
    node_operators.sort_by(|left, right| left.principal.cmp(&right.principal));
    Ok(MainnetNodeOperatorList {
        network: MAINNET_NETWORK.to_string(),
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version,
        fetched_at: request.fetched_at.clone(),
        fetched_by: request.fetched_by.clone(),
        source_endpoint: request.endpoint.clone(),
        node_operators,
    })
}

fn node_operator_from_record(
    principal: String,
    record: NodeOperatorRecord,
    node_counts: &BTreeMap<String, u32>,
) -> Result<MainnetNodeOperator, RegistryFetchError> {
    let node_provider_principal = principal_text_from_required_raw(
        &record.node_provider_principal_id,
        "node_operator_record.node_provider_principal_id",
    )?;
    Ok(MainnetNodeOperator {
        node_count: Some(node_counts.get(&principal).copied().unwrap_or(0)),
        principal,
        node_provider_principal,
        node_allowance: record.node_allowance,
        data_center_id: record.dc_id,
    })
}

async fn get_latest_version(
    agent: &Agent,
    registry_canister: &Principal,
) -> Result<u64, RegistryFetchError> {
    let bytes = agent
        .query(registry_canister, "get_latest_version")
        .with_arg(Vec::<u8>::new())
        .call()
        .await
        .map_err(|err| RegistryFetchError::AgentCall {
            method: "get_latest_version",
            reason: err.to_string(),
        })?;
    let response = decode_message::<RegistryGetLatestVersionResponse>(
        "RegistryGetLatestVersionResponse",
        &bytes,
    )?;
    Ok(response.version)
}

async fn get_registry_value(
    agent: &Agent,
    registry_canister: &Principal,
    key: &str,
    version: u64,
) -> Result<Vec<u8>, RegistryFetchError> {
    let request = RegistryGetValueRequest {
        version: Some(UInt64Value { value: version }),
        key: key.as_bytes().to_vec(),
    };
    let mut arg = Vec::new();
    request
        .encode(&mut arg)
        .map_err(|err| RegistryFetchError::ProtobufEncode {
            message: "RegistryGetValueRequest",
            reason: err.to_string(),
        })?;
    let bytes = agent
        .query(registry_canister, "get_value")
        .with_arg(arg)
        .call()
        .await
        .map_err(|err| RegistryFetchError::AgentCall {
            method: "get_value",
            reason: err.to_string(),
        })?;
    let response = decode_message::<RegistryGetValueResponse>("RegistryGetValueResponse", &bytes)?;
    match registry_value_content_from_response(key, response)? {
        RegistryValueContent::Value(value) => Ok(value),
        RegistryValueContent::LargeValueChunkKeys(keys) => {
            get_large_registry_value(agent, registry_canister, &keys).await
        }
    }
}

fn registry_value_content_from_response(
    key: &str,
    response: RegistryGetValueResponse,
) -> Result<RegistryValueContent, RegistryFetchError> {
    if let Some(error) = response.error {
        return Err(RegistryFetchError::RegistryValue {
            key: key.to_string(),
            code: registry_error_code(error.code).to_string(),
            reason: error.reason,
        });
    }
    match response.content {
        Some(registry_get_value_response::Content::Value(value)) => {
            Ok(RegistryValueContent::Value(value))
        }
        Some(registry_get_value_response::Content::LargeValueChunkKeys(keys)) => {
            Ok(RegistryValueContent::LargeValueChunkKeys(keys))
        }
        None => Err(RegistryFetchError::MissingValue {
            key: key.to_string(),
        }),
    }
}

async fn get_large_registry_value(
    agent: &Agent,
    registry_canister: &Principal,
    keys: &LargeValueChunkKeys,
) -> Result<Vec<u8>, RegistryFetchError> {
    let mut value = Vec::new();
    for chunk_sha256 in &keys.chunk_content_sha256s {
        let chunk_content = get_registry_chunk(agent, registry_canister, chunk_sha256).await?;
        append_validated_chunk(&mut value, chunk_sha256, chunk_content)?;
    }
    Ok(value)
}

async fn get_registry_chunk(
    agent: &Agent,
    registry_canister: &Principal,
    content_sha256: &[u8],
) -> Result<Vec<u8>, RegistryFetchError> {
    let request = RegistryGetChunkRequest {
        content_sha256: Some(content_sha256.to_vec()),
    };
    let arg = Encode!(&request).map_err(|err| RegistryFetchError::CandidEncode {
        message: "RegistryGetChunkRequest",
        reason: err.to_string(),
    })?;
    let bytes = agent
        .query(registry_canister, "get_chunk")
        .with_arg(arg)
        .call()
        .await
        .map_err(|err| RegistryFetchError::AgentCall {
            method: "get_chunk",
            reason: err.to_string(),
        })?;
    let result = Decode!(&bytes, Result<RegistryChunk, String>).map_err(|err| {
        RegistryFetchError::CandidDecode {
            message: "Result<RegistryChunk, String>",
            reason: err.to_string(),
        }
    })?;
    match result {
        Ok(chunk) => chunk
            .content
            .ok_or_else(|| RegistryFetchError::MissingChunkContent {
                sha256: hex_bytes(content_sha256),
            }),
        Err(reason) => Err(RegistryFetchError::RegistryChunkRejected {
            sha256: hex_bytes(content_sha256),
            reason,
        }),
    }
}

fn append_validated_chunk(
    value: &mut Vec<u8>,
    expected_sha256: &[u8],
    chunk_content: Vec<u8>,
) -> Result<(), RegistryFetchError> {
    let actual_sha256 = sha256_digest(&chunk_content);
    if actual_sha256.as_slice() != expected_sha256 {
        return Err(RegistryFetchError::ChunkHashMismatch {
            sha256: hex_bytes(expected_sha256),
            actual_sha256: hex_bytes(&actual_sha256),
        });
    }
    value.extend(chunk_content);
    Ok(())
}

fn sha256_digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[usize::from(byte >> 4)] as char);
        out.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    out
}

fn decode_message<T>(message: &'static str, bytes: &[u8]) -> Result<T, RegistryFetchError>
where
    T: Message + Default,
{
    T::decode(bytes).map_err(|err| RegistryFetchError::ProtobufDecode {
        message,
        reason: err.to_string(),
    })
}

fn subnet_info_from_record(subnet_principal: &str, record: &SubnetRecord) -> SubnetInfo {
    let subnet_kind = match SubnetType::try_from(record.subnet_type).ok() {
        Some(
            SubnetType::Application | SubnetType::VerifiedApplication | SubnetType::CloudEngine,
        ) => SubnetKind::Application,
        Some(SubnetType::System) => SubnetKind::System,
        Some(SubnetType::Unspecified) | None => SubnetKind::Unknown,
    };
    let charges_apply_by_default = subnet_kind == SubnetKind::Application;
    SubnetInfo {
        subnet_principal: subnet_principal.to_string(),
        subnet_kind,
        subnet_kind_source: ClassificationSource::Registry,
        subnet_specialization: SubnetSpecialization::None,
        subnet_specialization_source: ClassificationSource::Computed,
        geographic_scope: GeographicScope::Global,
        geographic_scope_source: ClassificationSource::Computed,
        subnet_label: subnet_kind.as_str().to_string(),
        subnet_label_source: ClassificationSource::Computed,
        node_count: Some(u32::try_from(record.membership.len()).unwrap_or(u32::MAX)),
        charges_apply_by_default,
    }
}

fn routing_ranges_from_table(
    table: &RoutingTable,
) -> Result<Vec<RoutingRange>, RegistryFetchError> {
    table
        .entries
        .iter()
        .map(|entry| {
            let range = entry
                .range
                .as_ref()
                .ok_or(RegistryFetchError::MissingField {
                    field: "routing_table.entries.range",
                })?;
            let subnet_id = entry
                .subnet_id
                .as_ref()
                .ok_or(RegistryFetchError::MissingField {
                    field: "routing_table.entries.subnet_id",
                })?;
            Ok(RoutingRange {
                start_canister_id: canister_id_text(
                    range.start_canister_id.as_ref(),
                    "range.start",
                )?,
                end_canister_id: canister_id_text(range.end_canister_id.as_ref(), "range.end")?,
                subnet_principal: subnet_id_text(subnet_id)?,
            })
        })
        .collect()
}

fn canister_id_text(
    canister_id: Option<&CanisterId>,
    field: &'static str,
) -> Result<String, RegistryFetchError> {
    let principal = canister_id
        .and_then(|id| id.principal_id.as_ref())
        .ok_or(RegistryFetchError::MissingField { field })?;
    principal_text_from_raw(&principal.raw, field)
}

fn subnet_id_text(subnet_id: &SubnetId) -> Result<String, RegistryFetchError> {
    let principal = subnet_id
        .principal_id
        .as_ref()
        .ok_or(RegistryFetchError::MissingField {
            field: "routing_table.entries.subnet_id.principal_id",
        })?;
    principal_text_from_raw(&principal.raw, "routing_table.entries.subnet_id")
}

fn principal_text_from_raw(raw: &[u8], field: &'static str) -> Result<String, RegistryFetchError> {
    Principal::try_from_slice(raw)
        .map(|principal| principal.to_text())
        .map_err(|err| RegistryFetchError::InvalidPrincipal {
            field,
            reason: err.to_string(),
        })
}

fn principal_text_from_required_raw(
    raw: &[u8],
    field: &'static str,
) -> Result<String, RegistryFetchError> {
    if raw.is_empty() {
        return Err(RegistryFetchError::MissingField { field });
    }
    principal_text_from_raw(raw, field)
}

fn subnet_record_key(subnet_principal: &str) -> String {
    format!("{SUBNET_RECORD_KEY_PREFIX}{subnet_principal}")
}

fn node_record_key(node_principal: &str) -> String {
    format!("{NODE_RECORD_KEY_PREFIX}{node_principal}")
}

fn node_operator_record_key(node_operator_principal: &str) -> String {
    format!("{NODE_OPERATOR_RECORD_KEY_PREFIX}{node_operator_principal}")
}

fn assigned_node_principals_from_subnets(
    subnet_records: &[SubnetRecord],
) -> Result<BTreeSet<String>, RegistryFetchError> {
    let mut node_principals = BTreeSet::new();
    for record in subnet_records {
        for raw in &record.membership {
            node_principals.insert(principal_text_from_raw(raw, "subnet_record.membership")?);
        }
    }
    Ok(node_principals)
}

fn node_provider_counts_from_records(
    node_principals: &BTreeSet<String>,
    node_records: &BTreeMap<String, NodeRecord>,
    node_operator_records: &BTreeMap<String, NodeOperatorRecord>,
) -> Result<BTreeMap<String, u32>, RegistryFetchError> {
    let mut counts = BTreeMap::<String, u32>::new();
    for node_principal in node_principals {
        let node_record =
            node_records
                .get(node_principal)
                .ok_or(RegistryFetchError::MissingField {
                    field: "node_record",
                })?;
        let node_operator_principal = principal_text_from_required_raw(
            &node_record.node_operator_id,
            "node_record.node_operator_id",
        )?;
        let node_operator_record = node_operator_records.get(&node_operator_principal).ok_or(
            RegistryFetchError::MissingField {
                field: "node_operator_record",
            },
        )?;
        let node_provider_principal = principal_text_from_required_raw(
            &node_operator_record.node_provider_principal_id,
            "node_operator_record.node_provider_principal_id",
        )?;
        let count = counts.entry(node_provider_principal).or_default();
        *count = count.saturating_add(1);
    }
    Ok(counts)
}

fn node_operator_counts_from_records(
    node_principals: &BTreeSet<String>,
    node_records: &BTreeMap<String, NodeRecord>,
) -> Result<BTreeMap<String, u32>, RegistryFetchError> {
    let mut counts = BTreeMap::<String, u32>::new();
    for node_principal in node_principals {
        let node_record =
            node_records
                .get(node_principal)
                .ok_or(RegistryFetchError::MissingField {
                    field: "node_record",
                })?;
        let node_operator_principal = principal_text_from_required_raw(
            &node_record.node_operator_id,
            "node_record.node_operator_id",
        )?;
        let count = counts.entry(node_operator_principal).or_default();
        *count = count.saturating_add(1);
    }
    Ok(counts)
}

///
/// NodeOperatorInventory
///
struct NodeOperatorInventory {
    node_principals: BTreeSet<String>,
    node_records: BTreeMap<String, NodeRecord>,
    node_operator_records: BTreeMap<String, NodeOperatorRecord>,
}

fn apply_mainnet_annotations(catalog: &mut SubnetCatalog) {
    let annotations = mainnet_annotations();
    for subnet in &mut catalog.subnets {
        let Some(annotation) = annotations.get(subnet.subnet_principal.as_str()) else {
            continue;
        };
        subnet.subnet_specialization = annotation.specialization;
        subnet.subnet_specialization_source = ClassificationSource::Curated;
        subnet.geographic_scope = annotation.geographic_scope;
        subnet.geographic_scope_source = ClassificationSource::Curated;
        subnet.subnet_label.clone_from(&annotation.label);
        subnet.subnet_label_source = ClassificationSource::Curated;
    }
}

fn mainnet_annotations() -> BTreeMap<&'static str, MainnetAnnotation> {
    BTreeMap::from([
        (
            FIDUCIARY_SUBNET,
            MainnetAnnotation {
                specialization: SubnetSpecialization::Fiduciary,
                geographic_scope: GeographicScope::Global,
                label: "fiduciary".to_string(),
            },
        ),
        (
            EUROPEAN_SUBNET,
            MainnetAnnotation {
                specialization: SubnetSpecialization::European,
                geographic_scope: GeographicScope::Europe,
                label: "european".to_string(),
            },
        ),
    ])
}

///
/// MainnetAnnotation
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct MainnetAnnotation {
    specialization: SubnetSpecialization,
    geographic_scope: GeographicScope,
    label: String,
}

fn registry_error_code(code: i32) -> &'static str {
    match RegistryErrorCode::try_from(code).ok() {
        Some(RegistryErrorCode::MalformedMessage) => "malformed_message",
        Some(RegistryErrorCode::KeyNotPresent) => "key_not_present",
        Some(RegistryErrorCode::KeyAlreadyPresent) => "key_already_present",
        Some(RegistryErrorCode::VersionNotLatest) => "version_not_latest",
        Some(RegistryErrorCode::VersionBeyondLatest) => "version_beyond_latest",
        Some(RegistryErrorCode::Authorization) => "authorization",
        Some(RegistryErrorCode::InternalError) => "internal_error",
        None => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proto::{CanisterIdRange, PrincipalId, RoutingTableEntry};

    const SUBNET_A: &str = "pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae";
    const SUBNET_B: &str = "aaaaa-aa";
    const CANISTER_A: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";

    #[test]
    fn registry_records_convert_to_catalog_domain_structs() {
        let request = MainnetRegistryFetchRequest {
            endpoint: "https://icp-api.io".to_string(),
            fetched_at: "2026-06-04T00:00:00Z".to_string(),
            fetched_by: "test".to_string(),
        };
        let subnet_records = BTreeMap::from([
            (
                SUBNET_A.to_string(),
                subnet_record(SubnetType::Application, 34),
            ),
            (SUBNET_B.to_string(), subnet_record(SubnetType::System, 13)),
        ]);
        let catalog = catalog_from_parts_for_test(
            &request,
            42,
            subnet_list_record([SUBNET_A, SUBNET_B]),
            routing_table_record([(CANISTER_A, CANISTER_A, SUBNET_A)]),
            subnet_records,
        )
        .expect("catalog");

        assert_eq!(catalog.registry_version, 42);
        assert_eq!(catalog.subnets.len(), 2);
        assert_eq!(catalog.routing_ranges.len(), 1);
        let fiduciary = catalog.subnet_by_principal(SUBNET_A).expect("fiduciary");
        assert_eq!(
            fiduciary.subnet_specialization,
            SubnetSpecialization::Fiduciary
        );
        assert_eq!(fiduciary.node_count, Some(34));
        assert!(fiduciary.charges_apply_by_default);
        let system = catalog.subnet_by_principal(SUBNET_B).expect("system");
        assert_eq!(system.subnet_kind, SubnetKind::System);
        assert!(!system.charges_apply_by_default);
    }

    #[test]
    fn get_value_response_reports_large_value_chunk_keys() {
        let response = RegistryGetValueResponse {
            error: None,
            version: 1,
            content: Some(registry_get_value_response::Content::LargeValueChunkKeys(
                proto::LargeValueChunkKeys {
                    chunk_content_sha256s: vec![vec![1], vec![2]],
                },
            )),
            timestamp_nanoseconds: 0,
        };

        let content = registry_value_content_from_response("routing_table", response)
            .expect("large value chunk keys");

        match content {
            RegistryValueContent::LargeValueChunkKeys(keys) => {
                assert_eq!(keys.chunk_content_sha256s, vec![vec![1], vec![2]]);
            }
            RegistryValueContent::Value(value) => {
                panic!("expected chunk keys, got inline value {value:?}");
            }
        }
    }

    #[test]
    fn registry_get_chunk_request_candid_round_trips() {
        let request = RegistryGetChunkRequest {
            content_sha256: Some(vec![1, 2, 3]),
        };

        let bytes = Encode!(&request).expect("encode");
        let decoded = Decode!(&bytes, RegistryGetChunkRequest).expect("decode");

        assert_eq!(decoded, request);
    }

    #[test]
    fn governance_node_provider_response_converts_to_domain_structs() {
        let request = MainnetRegistryFetchRequest {
            endpoint: "https://icp-api.io".to_string(),
            fetched_at: "2026-06-04T00:00:00Z".to_string(),
            fetched_by: "test".to_string(),
        };
        let node_counts = BTreeMap::from([("aaaaa-aa".to_string(), 2)]);
        let response = ListNodeProvidersResponse {
            node_providers: vec![
                governance_node_provider("ryjl3-tyaaa-aaaaa-aaaba-cai", None),
                governance_node_provider("aaaaa-aa", Some(vec![0xab, 0xcd])),
            ],
        };

        let list = node_provider_list_from_response(&request, response, node_counts, 42)
            .expect("node providers");

        assert_eq!(list.network, MAINNET_NETWORK);
        assert_eq!(list.governance_canister_id, MAINNET_GOVERNANCE_CANISTER_ID);
        assert_eq!(list.registry_canister_id, MAINNET_REGISTRY_CANISTER_ID);
        assert_eq!(list.registry_version, 42);
        assert_eq!(list.node_providers.len(), 2);
        assert_eq!(list.node_providers[0].principal, "aaaaa-aa");
        assert_eq!(list.node_providers[0].node_count, Some(2));
        assert_eq!(
            list.node_providers[0].reward_account_hex.as_deref(),
            Some("abcd")
        );
        assert_eq!(
            list.node_providers[1].principal,
            "ryjl3-tyaaa-aaaaa-aaaba-cai"
        );
        assert_eq!(list.node_providers[1].node_count, Some(0));
        assert_eq!(list.node_providers[1].reward_account_hex, None);
    }

    #[test]
    fn node_provider_counts_follow_subnet_nodes_to_providers() {
        let provider_a = Principal::self_authenticating(b"provider-a").to_text();
        let provider_b = Principal::self_authenticating(b"provider-b").to_text();
        let operator_a = Principal::self_authenticating(b"operator-a").to_text();
        let operator_b = Principal::self_authenticating(b"operator-b").to_text();
        let node_a = Principal::self_authenticating(b"node-a").to_text();
        let node_b = Principal::self_authenticating(b"node-b").to_text();
        let node_c = Principal::self_authenticating(b"node-c").to_text();
        let subnet_records = vec![SubnetRecord {
            membership: vec![
                principal_raw(&node_a),
                principal_raw(&node_b),
                principal_raw(&node_c),
            ],
            subnet_type: SubnetType::Application as i32,
            canister_cycles_cost_schedule: 0,
        }];
        let node_principals =
            assigned_node_principals_from_subnets(&subnet_records).expect("node principals");
        let node_records = BTreeMap::from([
            (node_a, node_record(&operator_a)),
            (node_b, node_record(&operator_a)),
            (node_c, node_record(&operator_b)),
        ]);
        let node_operator_records = BTreeMap::from([
            (operator_a, node_operator_record(&provider_a)),
            (operator_b, node_operator_record(&provider_b)),
        ]);

        let counts = node_provider_counts_from_records(
            &node_principals,
            &node_records,
            &node_operator_records,
        )
        .expect("provider counts");

        assert_eq!(counts.get(&provider_a), Some(&2));
        assert_eq!(counts.get(&provider_b), Some(&1));
    }

    #[test]
    fn node_operator_list_follows_assigned_nodes_to_operator_records() {
        let request = MainnetRegistryFetchRequest {
            endpoint: "https://icp-api.io".to_string(),
            fetched_at: "2026-06-04T00:00:00Z".to_string(),
            fetched_by: "test".to_string(),
        };
        let provider = Principal::self_authenticating(b"provider").to_text();
        let operator_a = Principal::self_authenticating(b"operator-a").to_text();
        let operator_b = Principal::self_authenticating(b"operator-b").to_text();
        let node_a = Principal::self_authenticating(b"node-a").to_text();
        let node_b = Principal::self_authenticating(b"node-b").to_text();
        let node_c = Principal::self_authenticating(b"node-c").to_text();
        let inventory = NodeOperatorInventory {
            node_principals: BTreeSet::from([node_a.clone(), node_b.clone(), node_c.clone()]),
            node_records: BTreeMap::from([
                (node_a, node_record(&operator_a)),
                (node_b, node_record(&operator_a)),
                (node_c, node_record(&operator_b)),
            ]),
            node_operator_records: BTreeMap::from([
                (
                    operator_a.clone(),
                    NodeOperatorRecord {
                        node_operator_principal_id: principal_raw(&operator_a),
                        node_allowance: 4,
                        node_provider_principal_id: principal_raw(&provider),
                        dc_id: "dc-a".to_string(),
                    },
                ),
                (
                    operator_b.clone(),
                    NodeOperatorRecord {
                        node_operator_principal_id: principal_raw(&operator_b),
                        node_allowance: 7,
                        node_provider_principal_id: principal_raw(&provider),
                        dc_id: "dc-b".to_string(),
                    },
                ),
            ]),
        };

        let list =
            node_operator_list_from_inventory(&request, inventory, 42).expect("node operators");

        assert_eq!(list.network, MAINNET_NETWORK);
        assert_eq!(list.registry_canister_id, MAINNET_REGISTRY_CANISTER_ID);
        assert_eq!(list.registry_version, 42);
        assert_eq!(list.node_operators.len(), 2);
        let operator_a_row = list
            .node_operators
            .iter()
            .find(|operator| operator.principal == operator_a)
            .expect("operator a");
        assert_eq!(operator_a_row.node_provider_principal, provider);
        assert_eq!(operator_a_row.node_allowance, 4);
        assert_eq!(operator_a_row.data_center_id, "dc-a");
        assert_eq!(operator_a_row.node_count, Some(2));
        let operator_b_row = list
            .node_operators
            .iter()
            .find(|operator| operator.principal == operator_b)
            .expect("operator b");
        assert_eq!(operator_b_row.node_count, Some(1));
    }

    #[test]
    fn governance_node_provider_requires_principal() {
        let err = node_provider_from_governance(
            GovernanceNodeProvider {
                id: None,
                reward_account: None,
            },
            &BTreeMap::new(),
        )
        .expect_err("missing principal");

        assert!(matches!(
            err,
            RegistryFetchError::MissingField { field } if field == "node_provider.id"
        ));
    }

    #[test]
    fn validated_chunk_append_concatenates_matching_chunks() {
        let first = b"hello ".to_vec();
        let second = b"world".to_vec();
        let first_hash = sha256_digest(&first);
        let second_hash = sha256_digest(&second);
        let mut value = Vec::new();

        append_validated_chunk(&mut value, &first_hash, first).expect("first chunk");
        append_validated_chunk(&mut value, &second_hash, second).expect("second chunk");

        assert_eq!(value, b"hello world");
    }

    #[test]
    fn validated_chunk_append_rejects_hash_mismatch() {
        let expected = sha256_digest(b"expected");

        let err = append_validated_chunk(&mut Vec::new(), &expected, b"actual".to_vec())
            .expect_err("hash mismatch");

        assert!(matches!(
            err,
            RegistryFetchError::ChunkHashMismatch {
                sha256,
                actual_sha256
            } if sha256 == hex_bytes(&expected)
                && actual_sha256 == hex_bytes(&sha256_digest(b"actual"))
        ));
    }

    #[test]
    fn get_value_response_reports_registry_errors() {
        let response = RegistryGetValueResponse {
            error: Some(proto::RegistryError {
                code: RegistryErrorCode::KeyNotPresent as i32,
                reason: "missing".to_string(),
                key: b"routing_table".to_vec(),
            }),
            version: 1,
            content: None,
            timestamp_nanoseconds: 0,
        };

        let err =
            registry_value_content_from_response("routing_table", response).expect_err("registry");

        assert!(matches!(
            err,
            RegistryFetchError::RegistryValue {
                key,
                code,
                reason
            } if key == "routing_table" && code == "key_not_present" && reason == "missing"
        ));
    }

    fn catalog_from_parts_for_test(
        request: &MainnetRegistryFetchRequest,
        registry_version: u64,
        subnet_list: SubnetListRecord,
        routing_table: RoutingTable,
        subnet_records: BTreeMap<String, SubnetRecord>,
    ) -> Result<SubnetCatalog, RegistryFetchError> {
        if subnet_list.subnets.is_empty() {
            return Err(RegistryFetchError::EmptySubnetList);
        }
        if routing_table.entries.is_empty() {
            return Err(RegistryFetchError::EmptyRoutingTable);
        }
        let mut subnets = subnet_list
            .subnets
            .iter()
            .map(|subnet_raw| {
                let subnet_principal = principal_text_from_raw(subnet_raw, "subnet_list.subnets")?;
                let record = subnet_records.get(&subnet_principal).ok_or(
                    RegistryFetchError::MissingField {
                        field: "subnet_record",
                    },
                )?;
                Ok(subnet_info_from_record(&subnet_principal, record))
            })
            .collect::<Result<Vec<_>, RegistryFetchError>>()?;
        subnets.sort_by(|left, right| left.subnet_principal.cmp(&right.subnet_principal));
        let mut catalog = SubnetCatalog {
            catalog_schema_version: CATALOG_SCHEMA_VERSION,
            network: MAINNET_NETWORK.to_string(),
            registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
            registry_version,
            fetched_at: request.fetched_at.clone(),
            fetched_by: request.fetched_by.clone(),
            source_endpoint: request.endpoint.clone(),
            resolver_backend: "local-nns-subnet-catalog".to_string(),
            subnets,
            routing_ranges: routing_ranges_from_table(&routing_table)?,
        };
        apply_mainnet_annotations(&mut catalog);
        catalog.validate()?;
        Ok(catalog)
    }

    fn subnet_list_record<const N: usize>(subnets: [&str; N]) -> SubnetListRecord {
        SubnetListRecord {
            subnets: subnets.iter().map(|subnet| principal_raw(subnet)).collect(),
        }
    }

    fn subnet_record(subnet_type: SubnetType, members: usize) -> SubnetRecord {
        SubnetRecord {
            membership: (0..members)
                .map(|index| {
                    let index = u8::try_from(index).expect("fixture member index fits in u8");
                    principal_raw(&Principal::self_authenticating([index]).to_text())
                })
                .collect(),
            subnet_type: subnet_type as i32,
            canister_cycles_cost_schedule: 0,
        }
    }

    fn routing_table_record<const N: usize>(ranges: [(&str, &str, &str); N]) -> RoutingTable {
        RoutingTable {
            entries: ranges
                .iter()
                .map(|(start, end, subnet)| RoutingTableEntry {
                    range: Some(CanisterIdRange {
                        start_canister_id: Some(canister_id(start)),
                        end_canister_id: Some(canister_id(end)),
                    }),
                    subnet_id: Some(subnet_id(subnet)),
                })
                .collect(),
        }
    }

    fn canister_id(principal: &str) -> CanisterId {
        CanisterId {
            principal_id: Some(PrincipalId {
                raw: principal_raw(principal),
            }),
        }
    }

    fn subnet_id(principal: &str) -> SubnetId {
        SubnetId {
            principal_id: Some(PrincipalId {
                raw: principal_raw(principal),
            }),
        }
    }

    fn principal_raw(principal: &str) -> Vec<u8> {
        Principal::from_text(principal)
            .expect("principal")
            .as_slice()
            .to_vec()
    }

    fn governance_node_provider(
        principal: &str,
        reward_account_hash: Option<Vec<u8>>,
    ) -> GovernanceNodeProvider {
        GovernanceNodeProvider {
            id: Some(Principal::from_text(principal).expect("principal")),
            reward_account: reward_account_hash.map(|hash| GovernanceAccountIdentifier { hash }),
        }
    }

    fn node_record(node_operator: &str) -> NodeRecord {
        NodeRecord {
            node_operator_id: principal_raw(node_operator),
        }
    }

    fn node_operator_record(node_provider: &str) -> NodeOperatorRecord {
        NodeOperatorRecord {
            node_operator_principal_id: Vec::new(),
            node_allowance: 0,
            node_provider_principal_id: principal_raw(node_provider),
            dc_id: "dc1".to_string(),
        }
    }
}
