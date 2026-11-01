//! Passive application-owned Wire projections; Jobs records remain private storage.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum StateView {
    Pending,
    Running,
    Uncertain,
    Completed,
    Failed,
    Paused,
    Cancelled,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct JobView {
    pub id: u128,
    pub state: StateView,
    pub sequence: u64,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct JobsView {
    pub total: u64,
    pub receipts: u64,
    pub jobs: Vec<JobView>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct DeliveryEvidence {
    pub job: u128,
    pub sequence: u64,
    pub delta: u64,
}
