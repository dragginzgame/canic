//! Disposable Wire contracts for real call/callback qualification.
use ic_platform::Principal;
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 8192,
    allocation: 32768,
    nodes: 1024,
    depth: 16,
};
pub const EXECUTE: u64 = 0x5254_5043_0000_0001;
pub const TARGET: u64 = 0x5254_5043_0000_0002;
pub const METRICS: u64 = 0x5254_5043_0000_0003;
pub const OBSERVE: u64 = 0x5254_5043_0000_0004;
pub const RELEASE: u64 = 0x5254_5043_0000_0005;
pub const ROUND: u64 = 0x5254_5043_0000_0006;
pub const EXTERNAL: u64 = 0x5254_5043_0000_0008;
pub const CASCADE: u64 = 0x5254_5043_0000_0009;

/// Select the caller continuation without adding transport retries.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum AfterCall {
    Raw,
    DecodeWire,
    DropUnpolled,
    DropSubmitted,
    TrapCallback,
}
/// Exact prepared transport inputs and disposable callback control.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CallRequest {
    pub target: Principal,
    pub method: String,
    pub bytes: Vec<u8>,
    pub cycles: u128,
    pub timeout_seconds: Option<u32>,
    pub after: AfterCall,
}
/// One result and its matching callback refund; absent when unobserved.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CallObservation {
    pub result: Option<Result<Vec<u8>, canic::Error>>,
    pub decoded: Option<Result<u64, canic::Error>>,
    pub rejection_code: Option<u32>,
    pub refunded: Option<u128>,
    pub cost: u128,
}
/// Production generic attempt counts plus a fixture destruction witness.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CallMetrics {
    pub started: u64,
    pub completed: u64,
    pub failed: u64,
    pub dropped: u64,
    pub bounded_started: u64,
    pub unbounded_started: u64,
}
/// Select a real callee reply, rejection or trap.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum TargetReply {
    Wire,
    Malformed,
    Reject,
    Trap,
}
/// Bounded target effect and real asynchronous reply gate.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TargetRequest {
    pub id: u8,
    pub token: u64,
    pub accept: u128,
    pub hold: bool,
    pub reply: TargetReply,
}
/// Committed target effects independent of caller availability errors.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TargetObservation {
    pub calls: Vec<u64>,
    pub accepted: Vec<u128>,
    pub held: Vec<bool>,
}

/// Exact management target and external Ledger identity for codec qualification.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ExternalRequest {
    pub target: Principal,
    pub ledger: Principal,
}
