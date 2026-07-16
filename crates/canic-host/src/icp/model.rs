use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub(super) const LOCAL_NETWORK: &str = "local";
pub const REQUIRED_ICP_CLI_VERSION: &str = "1.0.0";
pub const ICP_CLI_SUPPORTED_VERSION_RANGE: &str = ">=1.0.0, <2.0.0";

/// Direct local replica endpoint used when ICP project state is unavailable.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalReplicaTarget {
    pub url: String,
    pub root_key: String,
}

///
/// IcpRawOutput
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IcpRawOutput {
    pub success: bool,
    pub status: String,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

///
/// IcpCli
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IcpCli {
    pub(super) executable: String,
    pub(super) environment: Option<String>,
    pub(super) network: Option<String>,
    pub(super) cwd: Option<PathBuf>,
    pub(super) local_replica: Option<LocalReplicaTarget>,
}

///
/// IcpCliVersion
///
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct IcpCliVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

///
/// IcpSnapshotCreateReceipt
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IcpSnapshotCreateReceipt {
    pub snapshot_id: String,
    pub taken_at_timestamp: Option<u64>,
    pub total_size_bytes: Option<u64>,
}

///
/// IcpCanisterStatusReport
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IcpCanisterStatusReport {
    pub id: String,
    pub name: Option<String>,
    pub status: String,
    pub settings: Option<IcpCanisterStatusSettings>,
    pub module_hash: Option<String>,
    pub memory_size: Option<String>,
    pub cycles: Option<String>,
    pub reserved_cycles: Option<String>,
    pub idle_cycles_burned_per_day: Option<String>,
}

///
/// IcpCanisterStatusSettings
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IcpCanisterStatusSettings {
    #[serde(default)]
    pub controllers: Vec<String>,
    pub compute_allocation: Option<String>,
    pub memory_allocation: Option<String>,
    pub freezing_threshold: Option<String>,
    pub reserved_cycles_limit: Option<String>,
    pub wasm_memory_limit: Option<String>,
    pub wasm_memory_threshold: Option<String>,
    pub log_memory_limit: Option<String>,
}
