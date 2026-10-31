//! Native transport identities and request records for the timer integration fixture.
//! Framework scheduling and observability remain owned by their production modules.

use serde::{Deserialize, Serialize};

pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 256,
    allocation: 1024,
    nodes: 32,
    depth: 8,
};
/// The optional TTL needs additional cumulative codec allocation, not more input bytes.
pub const INTENT_REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    allocation: 4096,
    ..REQUEST_LIMITS
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 65_536,
    nodes: 2048,
    depth: 32,
};

pub const BEGIN_INTENT: u64 = 0x5254_5052_0000_0001;
pub const SCHEDULE_WATCHDOG: u64 = 0x5254_5052_0000_0002;
pub const CANCEL_WATCHDOG: u64 = 0x5254_5052_0000_0003;
pub const SET_WATCHDOG_TRAP: u64 = 0x5254_5052_0000_0004;
pub const FILL_REGISTRY: u64 = 0x5254_5052_0000_0005;
pub const REJECT_INVALID_IDENTITY: u64 = 0x5254_5052_0000_0006;
pub const COUNTS: u64 = 0x5254_5052_0000_0007;
pub const SAMPLE_METRICS: u64 = 0x5254_5052_0000_0008;
pub const QUALIFY_SAMPLING: u64 = 0x5254_5052_0000_0009;
pub const CONFIGURE_SAMPLER: u64 = 0x5254_5052_0000_000a;
pub const SUSPEND_SAMPLER: u64 = 0x5254_5052_0000_000b;
pub const HISTORY_SAMPLING_COST: u64 = 0x5254_5052_0000_000c;
pub const HISTORY_QUERY_COST: u64 = 0x5254_5052_0000_000d;

/// One bounded request to the installed timer fixture.
#[derive(Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct IntentRequest {
    pub resource_seed: u8,
    pub ttl_secs: Option<u64>,
}

/// One bounded request to the installed timer fixture.
#[derive(Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct SamplingRequest {
    pub checkpoints: u16,
    pub reject_family: bool,
}

pub const TEST: u64 = 0x5254_5052_0000_0010;
pub const MEMORY: u64 = 0x5254_5052_0000_0011;
pub const PERF_PROBE: u64 = 0x5254_5052_0000_0012;
pub const PERF_ENTERED: u64 = 0x5254_5052_0000_0013;
pub const PERF_COMPLETED: u64 = 0x5254_5052_0000_0014;
pub const PERF_METRICS: u64 = 0x5254_5052_0000_0015;
pub const ADAPTER_BINDING: u64 = 0x5254_5052_0000_0016;
