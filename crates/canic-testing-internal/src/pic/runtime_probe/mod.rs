//! Expected native contracts for the installed timer integration fixture.
//! These test-owned identities exercise the actual endpoint declarations through Testkit.

use canic::__internal::ic_wire::Limits;
use ic_testkit::pic::WireCallSpec;
use serde::{Deserialize, Serialize};

const REQUEST_LIMITS: Limits = Limits {
    bytes: 256,
    allocation: 1024,
    nodes: 32,
    depth: 8,
};
const RESPONSE_LIMITS: Limits = Limits {
    bytes: 16_384,
    allocation: 65_536,
    nodes: 2048,
    depth: 32,
};

/// Native call contract for `begin_timer_probe_intent`.
pub const BEGIN_INTENT: WireCallSpec = WireCallSpec {
    request_limits: Limits {
        allocation: 4096,
        ..REQUEST_LIMITS
    },
    ..spec("begin_timer_probe_intent", 0x5254_5052_0000_0001)
};
/// Native call contract for `schedule_cost_probe_watchdog`.
pub const SCHEDULE_WATCHDOG: WireCallSpec =
    spec("schedule_cost_probe_watchdog", 0x5254_5052_0000_0002);
/// Native call contract for `cancel_cost_probe_watchdog`.
pub const CANCEL_WATCHDOG: WireCallSpec = spec("cancel_cost_probe_watchdog", 0x5254_5052_0000_0003);
/// Native call contract for `set_cost_probe_trap`.
pub const SET_WATCHDOG_TRAP: WireCallSpec = spec("set_cost_probe_trap", 0x5254_5052_0000_0004);
/// Native call contract for `fill_timer_registry`.
pub const FILL_REGISTRY: WireCallSpec = spec("fill_timer_registry", 0x5254_5052_0000_0005);
/// Native call contract for `reject_invalid_timer_identity`.
pub const REJECT_INVALID_IDENTITY: WireCallSpec =
    spec("reject_invalid_timer_identity", 0x5254_5052_0000_0006);
/// Native call contract for `timer_probe_counts`.
pub const COUNTS: WireCallSpec = spec("timer_probe_counts", 0x5254_5052_0000_0007);
/// Native call contract for `sample_public_metrics`.
pub const SAMPLE_METRICS: WireCallSpec = spec("sample_public_metrics", 0x5254_5052_0000_0008);
/// Native call contract for `qualify_public_metrics_sampling`.
pub const QUALIFY_SAMPLING: WireCallSpec =
    spec("qualify_public_metrics_sampling", 0x5254_5052_0000_0009);
/// Native call contract for `configure_public_sampler`.
pub const CONFIGURE_SAMPLER: WireCallSpec = spec("configure_public_sampler", 0x5254_5052_0000_000a);
/// Native call contract for `suspend_public_sampler_fixture`.
pub const SUSPEND_SAMPLER: WireCallSpec =
    spec("suspend_public_sampler_fixture", 0x5254_5052_0000_000b);
/// Native call contract for `qualify_history_sampling_cost`.
pub const HISTORY_SAMPLING_COST: WireCallSpec =
    spec("qualify_history_sampling_cost", 0x5254_5052_0000_000c);
/// Native call contract for `qualify_history_query_cost`.
pub const HISTORY_QUERY_COST: WireCallSpec =
    spec("qualify_history_query_cost", 0x5254_5052_0000_000d);

const fn spec(method: &'static str, method_id: u64) -> WireCallSpec {
    WireCallSpec {
        method,
        method_id,
        request_limits: REQUEST_LIMITS,
        response_limits: RESPONSE_LIMITS,
    }
}

/// Reserve one application intent through the production local-intent owner.
#[derive(Deserialize, Serialize)]
pub struct IntentRequest {
    pub resource_seed: u8,
    pub ttl_secs: Option<u64>,
}

/// Select bounded instrumentation and optional producer rejection.
#[derive(Serialize)]
pub struct SamplingRequest {
    pub checkpoints: u16,
    pub reject_family: bool,
}

/// Native runtime fixture `test` contract.
pub const TEST: WireCallSpec = spec("test", 0x5254_5052_0000_0010);

/// Native runtime fixture `memory_allocations_probe` contract.
pub const MEMORY: WireCallSpec = WireCallSpec {
    response_limits: canic::protocol::protected_memory_allocations::RESPONSE_LIMITS,
    ..spec("memory_allocations_probe", 0x5254_5052_0000_0011)
};

/// Native runtime fixture `perf_context_probe` contract.
pub const PERF_PROBE: WireCallSpec = spec("perf_context_probe", 0x5254_5052_0000_0012);

/// Native runtime fixture `perf_context_entered` contract.
pub const PERF_ENTERED: WireCallSpec = spec("perf_context_entered", 0x5254_5052_0000_0013);

/// Native runtime fixture `perf_context_completed` contract.
pub const PERF_COMPLETED: WireCallSpec = spec("perf_context_completed", 0x5254_5052_0000_0014);

/// Native runtime fixture `perf_context_metrics` contract.
pub const PERF_METRICS: WireCallSpec = spec("perf_context_metrics", 0x5254_5052_0000_0015);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_request_budget_admits_optional_ttl() {
        for resource_seed in [0, u8::MAX] {
            for ttl_secs in [None, Some(0), Some(u64::MAX)] {
                let request = IntentRequest {
                    resource_seed,
                    ttl_secs,
                };
                let bytes = ic_wire::native::encode_request(
                    BEGIN_INTENT.method_id,
                    &request,
                    BEGIN_INTENT.request_limits,
                )
                .expect("complete intent request fits its admission");
                let decoded: IntentRequest = ic_wire::native::decode_request(
                    BEGIN_INTENT.method_id,
                    &bytes,
                    BEGIN_INTENT.request_limits,
                )
                .expect("admitted intent request decodes within the same limits");
                assert_eq!(decoded.resource_seed, resource_seed);
                assert_eq!(decoded.ttl_secs, ttl_secs);
            }
        }
    }
}
