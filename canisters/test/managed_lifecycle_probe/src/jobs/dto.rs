//! Immutable application commands for bounded Jobs endpoints.
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct EnqueueRequest {
    pub delta: u64,
    pub delay_secs: u64,
}

#[derive(Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ReconcileRequest {
    pub job: u128,
    pub sequence: u64,
}
