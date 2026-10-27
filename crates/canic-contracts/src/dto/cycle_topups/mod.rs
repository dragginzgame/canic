//! Native transport of automatic funding history; persisted records remain unchanged.
use crate::{
    cycles::Cycles,
    values::cycles::{CycleTopupEventStatus, CycleTopupFailureDisposition},
};

/// One observed automatic funding event.
#[derive(Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CycleTopupEvent {
    pub timestamp_secs: u64,
    pub sequence: u32,
    pub requested_cycles: Cycles,
    pub transferred_cycles: Option<Cycles>,
    pub status: CycleTopupEventStatus,
    pub error: Option<String>,
    pub parent_failure: Option<CycleTopupFailure>,
}

/// Exact parent operation and retry disposition attached to a failed event.
#[derive(Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CycleTopupFailure {
    pub parent: ic_platform::Principal,
    pub operation_id: [u8; 32],
    pub public_error_code: u16,
    pub disposition: CycleTopupFailureDisposition,
}
