use prost::Oneof;

///
/// RegistryError
///
#[derive(Clone, Eq, prost::Message, PartialEq)]
pub struct RegistryError {
    #[prost(enumeration = "RegistryErrorCode", tag = "1")]
    pub code: i32,
    #[prost(string, tag = "2")]
    pub reason: String,
    #[prost(bytes = "vec", tag = "3")]
    pub key: Vec<u8>,
}

///
/// RegistryErrorCode
///
#[derive(Clone, Copy, Debug, prost::Enumeration, Eq, PartialEq)]
#[repr(i32)]
pub enum RegistryErrorCode {
    MalformedMessage = 0,
    KeyNotPresent = 1,
    KeyAlreadyPresent = 2,
    VersionNotLatest = 3,
    VersionBeyondLatest = 4,
    Authorization = 5,
    InternalError = 999,
}

///
/// LargeValueChunkKeys
///
#[derive(Clone, Eq, prost::Message, PartialEq)]
pub struct LargeValueChunkKeys {
    #[prost(bytes = "vec", repeated, tag = "1")]
    pub chunk_content_sha256s: Vec<Vec<u8>>,
}

///
/// RegistryGetValueRequest
///
#[derive(Clone, Eq, prost::Message, PartialEq)]
pub struct RegistryGetValueRequest {
    #[prost(message, optional, tag = "1")]
    pub version: Option<UInt64Value>,
    #[prost(bytes = "vec", tag = "2")]
    pub key: Vec<u8>,
}

///
/// UInt64Value
///
#[derive(Clone, Eq, prost::Message, PartialEq)]
pub struct UInt64Value {
    #[prost(uint64, tag = "1")]
    pub value: u64,
}

///
/// RegistryGetValueResponse
///
/// This is the high-capacity `get_value` response wire shape. The upstream
/// proto calls it `HighCapacityRegistryGetValueResponse`; Canic keeps the
/// local name narrow because this adapter never exposes protobuf types.
#[derive(Clone, Eq, prost::Message, PartialEq)]
pub struct RegistryGetValueResponse {
    #[prost(message, optional, tag = "1")]
    pub error: Option<RegistryError>,
    #[prost(uint64, tag = "2")]
    pub version: u64,
    #[prost(oneof = "registry_get_value_response::Content", tags = "3, 4")]
    pub content: Option<registry_get_value_response::Content>,
    #[prost(uint64, tag = "5")]
    pub timestamp_nanoseconds: u64,
}

pub mod registry_get_value_response {
    use super::{LargeValueChunkKeys, Oneof};

    ///
    /// Content
    ///
    #[derive(Clone, Eq, Oneof, PartialEq)]
    pub enum Content {
        #[prost(bytes = "vec", tag = "3")]
        Value(Vec<u8>),
        #[prost(message, tag = "4")]
        LargeValueChunkKeys(LargeValueChunkKeys),
    }
}

///
/// RegistryGetLatestVersionResponse
///
#[derive(Clone, Eq, prost::Message, PartialEq)]
pub struct RegistryGetLatestVersionResponse {
    #[prost(uint64, tag = "1")]
    pub version: u64,
}

///
/// PrincipalId
///
#[derive(Clone, Eq, prost::Message, PartialEq)]
pub struct PrincipalId {
    #[prost(bytes = "vec", tag = "1")]
    pub raw: Vec<u8>,
}

///
/// CanisterId
///
#[derive(Clone, Eq, prost::Message, PartialEq)]
pub struct CanisterId {
    #[prost(message, optional, tag = "1")]
    pub principal_id: Option<PrincipalId>,
}

///
/// SubnetId
///
#[derive(Clone, Eq, prost::Message, PartialEq)]
pub struct SubnetId {
    #[prost(message, optional, tag = "1")]
    pub principal_id: Option<PrincipalId>,
}

///
/// SubnetListRecord
///
#[derive(Clone, Eq, prost::Message, PartialEq)]
pub struct SubnetListRecord {
    #[prost(bytes = "vec", repeated, tag = "2")]
    pub subnets: Vec<Vec<u8>>,
}

///
/// SubnetRecord
///
#[derive(Clone, Eq, prost::Message, PartialEq)]
pub struct SubnetRecord {
    #[prost(bytes = "vec", repeated, tag = "3")]
    pub membership: Vec<Vec<u8>>,
    #[prost(enumeration = "SubnetType", tag = "15")]
    pub subnet_type: i32,
    #[prost(enumeration = "CanisterCyclesCostSchedule", tag = "30")]
    pub canister_cycles_cost_schedule: i32,
}

///
/// NodeRecord
///
#[derive(Clone, Eq, prost::Message, PartialEq)]
pub struct NodeRecord {
    #[prost(bytes = "vec", tag = "15")]
    pub node_operator_id: Vec<u8>,
}

///
/// NodeOperatorRecord
///
#[derive(Clone, Eq, prost::Message, PartialEq)]
pub struct NodeOperatorRecord {
    #[prost(bytes = "vec", tag = "1")]
    pub node_operator_principal_id: Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub node_allowance: u64,
    #[prost(bytes = "vec", tag = "3")]
    pub node_provider_principal_id: Vec<u8>,
    #[prost(string, tag = "4")]
    pub dc_id: String,
}

///
/// DataCenterRecord
///
#[derive(Clone, prost::Message, PartialEq)]
pub struct DataCenterRecord {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub region: String,
    #[prost(string, tag = "3")]
    pub owner: String,
    #[prost(message, optional, tag = "4")]
    pub gps: Option<Gps>,
}

///
/// Gps
///
#[derive(Clone, prost::Message, PartialEq)]
pub struct Gps {
    #[prost(float, tag = "1")]
    pub latitude: f32,
    #[prost(float, tag = "2")]
    pub longitude: f32,
}

///
/// SubnetType
///
#[derive(Clone, Copy, Debug, prost::Enumeration, Eq, PartialEq)]
#[repr(i32)]
pub enum SubnetType {
    Unspecified = 0,
    Application = 1,
    System = 2,
    VerifiedApplication = 4,
    CloudEngine = 5,
}

///
/// CanisterCyclesCostSchedule
///
#[derive(Clone, Copy, Debug, prost::Enumeration, Eq, PartialEq)]
#[repr(i32)]
pub enum CanisterCyclesCostSchedule {
    Unspecified = 0,
    Normal = 1,
    Free = 2,
}

///
/// CanisterIdRange
///
#[derive(Clone, Eq, prost::Message, PartialEq)]
pub struct CanisterIdRange {
    #[prost(message, optional, tag = "3")]
    pub start_canister_id: Option<CanisterId>,
    #[prost(message, optional, tag = "4")]
    pub end_canister_id: Option<CanisterId>,
}

///
/// RoutingTable
///
#[derive(Clone, Eq, prost::Message, PartialEq)]
pub struct RoutingTable {
    #[prost(message, repeated, tag = "1")]
    pub entries: Vec<RoutingTableEntry>,
}

///
/// RoutingTableEntry
///
#[derive(Clone, Eq, prost::Message, PartialEq)]
pub struct RoutingTableEntry {
    #[prost(message, optional, tag = "1")]
    pub range: Option<CanisterIdRange>,
    #[prost(message, optional, tag = "2")]
    pub subnet_id: Option<SubnetId>,
}
