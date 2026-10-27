//! Native audit shapes and budgets; persisted measurements retain their owners.

pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 65_536,
    nodes: 4096,
    depth: 32,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 2_097_152,
    allocation: 8_388_608,
    nodes: 131_072,
    depth: 32,
};
pub const TIME: u64 = 0x4155_4454_0000_0010;
pub const ENV: u64 = 0x4155_4454_0000_0011;
pub const LOG: u64 = 0x4155_4454_0000_0012;
pub const COHORT: u64 = 0x4155_4454_0000_0013;
pub const BALANCE: u64 = 0x4155_4454_0000_0014;

/// Complete environment observation with native principals.
#[derive(serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Environment {
    pub fleet_subnet_root_pid: Option<ic_platform::Principal>,
    pub component_spec: Option<canic::ids::ComponentSpecId>,
    pub subnet_pid: Option<ic_platform::Principal>,
    pub root_pid: Option<ic_platform::Principal>,
    pub canister_role: Option<canic::ids::CanisterRole>,
    pub parent_pid: Option<ic_platform::Principal>,
}

/// One bounded log-page selection.
#[derive(serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct LogRequest {
    pub crate_name: Option<String>,
    pub topic: Option<String>,
    pub min_level: Option<canic::__internal::core::log::Level>,
    pub page: canic::dto::page::PageRequest,
}

/// Fixed cohort selection; nominal fanout remains controlled by the build configuration.
#[derive(serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CohortRequest {
    pub slot: u8,
    pub value: u64,
}
