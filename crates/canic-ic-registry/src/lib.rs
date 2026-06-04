//! Live mainnet IC NNS registry adapter for Canic host tools.

pub(crate) mod proto;

use candid::Principal;
use canic_subnet_catalog::{
    CATALOG_SCHEMA_VERSION, CatalogError, ClassificationSource, GeographicScope, MAINNET_NETWORK,
    MAINNET_REGISTRY_CANISTER_ID, RoutingRange, SubnetCatalog, SubnetInfo, SubnetKind,
    SubnetSpecialization,
};
use ic_agent::Agent;
use prost::Message;
use proto::{
    CanisterId, RegistryErrorCode, RegistryGetLatestVersionResponse, RegistryGetValueRequest,
    RegistryGetValueResponse, RoutingTable, SubnetId, SubnetListRecord, SubnetRecord, SubnetType,
    UInt64Value, registry_get_value_response,
};
use std::collections::BTreeMap;
use thiserror::Error as ThisError;

pub const DEFAULT_MAINNET_ENDPOINT: &str = "https://icp-api.io";

const SUBNET_LIST_KEY: &str = "subnet_list";
const ROUTING_TABLE_KEY: &str = "routing_table";
const SUBNET_RECORD_KEY_PREFIX: &str = "subnet_record_";
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

    #[error(
        "registry get_value for key {key} returned {chunks} large-value chunks; chunk retrieval is not implemented yet"
    )]
    ChunkedValue { key: String, chunks: usize },

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

pub fn fetch_mainnet_subnet_catalog(
    request: &MainnetRegistryFetchRequest,
) -> Result<SubnetCatalog, RegistryFetchError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| RegistryFetchError::Runtime(err.to_string()))?;
    runtime.block_on(fetch_mainnet_subnet_catalog_async(request))
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
    if let Some(error) = response.error {
        return Err(RegistryFetchError::RegistryValue {
            key: key.to_string(),
            code: registry_error_code(error.code).to_string(),
            reason: error.reason,
        });
    }
    match response.content {
        Some(registry_get_value_response::Content::Value(value)) => Ok(value),
        Some(registry_get_value_response::Content::LargeValueChunkKeys(keys)) => {
            Err(RegistryFetchError::ChunkedValue {
                key: key.to_string(),
                chunks: keys.chunk_content_sha256s.len(),
            })
        }
        None => Err(RegistryFetchError::MissingValue {
            key: key.to_string(),
        }),
    }
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

fn subnet_record_key(subnet_principal: &str) -> String {
    format!("{SUBNET_RECORD_KEY_PREFIX}{subnet_principal}")
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
    fn get_value_response_reports_chunked_values() {
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

        let err = value_from_response_for_test("routing_table", response).expect_err("chunked");

        assert!(matches!(
            err,
            RegistryFetchError::ChunkedValue {
                key,
                chunks: 2
            } if key == "routing_table"
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

        let err = value_from_response_for_test("routing_table", response).expect_err("registry");

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

    fn value_from_response_for_test(
        key: &str,
        response: RegistryGetValueResponse,
    ) -> Result<Vec<u8>, RegistryFetchError> {
        if let Some(error) = response.error {
            return Err(RegistryFetchError::RegistryValue {
                key: key.to_string(),
                code: registry_error_code(error.code).to_string(),
                reason: error.reason,
            });
        }
        match response.content {
            Some(registry_get_value_response::Content::Value(value)) => Ok(value),
            Some(registry_get_value_response::Content::LargeValueChunkKeys(keys)) => {
                Err(RegistryFetchError::ChunkedValue {
                    key: key.to_string(),
                    chunks: keys.chunk_content_sha256s.len(),
                })
            }
            None => Err(RegistryFetchError::MissingValue {
                key: key.to_string(),
            }),
        }
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
}
