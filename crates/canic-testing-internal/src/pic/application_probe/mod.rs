//! Expected native contracts for the installed application fixtures.
//! Testkit owns transport and decoding; these values qualify their real declarations.

use ic_testkit::pic::WireCallSpec;

const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 65_536,
    nodes: 128,
    depth: 16,
};
/// Full 32-byte identities and their scalar fields require more than 128 nodes.
const OPERATION_REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    nodes: 256,
    ..REQUEST_LIMITS
};
const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 262_144,
    nodes: 4096,
    depth: 32,
};

/// Child-to-parent funding through the fixture's actual RPC owner.
pub const REQUEST_CYCLES: WireCallSpec = spec("request_cycles_from_parent", 0x5445_5354_0000_0100);
/// On-demand scaling and subsequent child readiness.
pub const CREATE_WORKER: WireCallSpec = spec("create_worker", 0x5445_5354_0000_0200);
/// Pure scaling placement decision under the same local-only guard.
pub const PLAN_WORKER: WireCallSpec = spec("plan_create_worker", 0x5445_5354_0000_0201);

const fn spec(method: &'static str, method_id: u64) -> WireCallSpec {
    WireCallSpec {
        method,
        method_id,
        request_limits: REQUEST_LIMITS,
        response_limits: RESPONSE_LIMITS,
    }
}

/// Caller-returning Fleet admission probe in managed child and Hub fixtures.
pub const ADMISSION: WireCallSpec = spec("test_fleet_admission_probe", 0x5445_5354_0000_0300);
/// Unit-returning admission probe in the lifecycle test App.
pub const ADMISSION_UNIT: WireCallSpec = spec("test_fleet_admission_probe", 0x5445_5354_0000_0301);

/// Expected contract of the installed `managed_guard_public_probe` endpoint.
pub const MANAGED_PUBLIC: WireCallSpec = spec("managed_guard_public_probe", 0x5445_5354_0000_0400);

/// Expected contract of the installed `managed_guard_admission_probe` endpoint.
pub const MANAGED_GUARD: WireCallSpec =
    spec("managed_guard_admission_probe", 0x5445_5354_0000_0401);

/// Expected contract of the installed `managed_guard_owned_probe` endpoint.
pub const MANAGED_OWNED: WireCallSpec = spec("managed_guard_owned_probe", 0x5445_5354_0000_0402);

/// Expected contract of the installed `managed_guard_workflow_runs` endpoint.
pub const MANAGED_WORKFLOW_RUNS: WireCallSpec =
    spec("managed_guard_workflow_runs", 0x5445_5354_0000_0403);

/// Expected contract of the installed `canic_fleet_admission_parity_probe` endpoint.
pub const MANAGED_PARITY: WireCallSpec =
    spec("canic_fleet_admission_parity_probe", 0x5445_5354_0000_0404);

/// Expected contract of the installed `managed_initialization_bytes` endpoint.
pub const MANAGED_INITIALIZATION: WireCallSpec = WireCallSpec {
    response_limits: ic_wire::Limits {
        bytes: 131_072,
        allocation: 4_194_304,
        nodes: 131_072,
        depth: 32,
    },
    ..spec("managed_initialization_bytes", 0x5445_5354_0000_0405)
};

/// Installed checkpoint fixture transition contract.
pub const CHECKPOINT_TRANSITION: WireCallSpec = WireCallSpec {
    request_limits: ic_wire::Limits {
        allocation: 2_097_152,
        nodes: 4096,
        ..REQUEST_LIMITS
    },
    ..spec("checkpoint_transition", 0x5445_5354_0000_0406)
};

/// Installed checkpoint fixture restore contract.
pub const CHECKPOINT_RESTORE: WireCallSpec = WireCallSpec {
    request_limits: CHECKPOINT_TRANSITION.request_limits,
    ..spec("checkpoint_restore", 0x5445_5354_0000_0407)
};

/// Installed checkpoint fixture increment contract.
pub const CHECKPOINT_INCREMENT: WireCallSpec = spec("checkpoint_increment", 0x5445_5354_0000_0408);

/// Installed checkpoint fixture view contract.
pub const CHECKPOINT_VIEW: WireCallSpec = WireCallSpec {
    response_limits: ic_wire::Limits {
        allocation: CHECKPOINT_TRANSITION.request_limits.allocation,
        ..RESPONSE_LIMITS
    },
    ..spec("checkpoint_view", 0x5445_5354_0000_0409)
};

/// Installed recovery fixture `test_recovery_generation` contract.
pub const GENERATION: WireCallSpec = spec("test_recovery_generation", 0x5445_5354_0000_0501);
/// Installed recovery fixture `test_set_user_row` contract.
pub const SET_ROW: WireCallSpec = spec("test_set_user_row", 0x5445_5354_0000_0502);
/// Installed recovery fixture `test_user_rows` contract.
pub const ROWS: WireCallSpec = spec("test_user_rows", 0x5445_5354_0000_0503);
/// Installed recovery fixture `test_recovery_balance` contract.
pub const RECOVERY_BALANCE: WireCallSpec = spec("test_recovery_balance", 0x5445_5354_0000_0504);
/// Installed recovery fixture `test_topup_request` contract.
pub const TOPUP_REQUEST: WireCallSpec = spec("test_topup_request", 0x5445_5354_0000_0505);
/// Installed recovery fixture `test_topup_collision` contract.
pub const TOPUP_COLLISION: WireCallSpec = WireCallSpec {
    request_limits: OPERATION_REQUEST_LIMITS,
    ..spec("test_topup_collision", 0x5445_5354_0000_0506)
};
/// Installed recovery fixture `test_topup_demand` contract.
pub const TOPUP_DEMAND: WireCallSpec = spec("test_topup_demand", 0x5445_5354_0000_0507);
/// Installed recovery fixture `test_topup_exhaustion` contract.
pub const TOPUP_EXHAUSTION: WireCallSpec = spec("test_topup_exhaustion", 0x5445_5354_0000_0508);

/// One retained parent request, optionally discarding only its reply.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct TopupRequest {
    pub cycles: u128,
    pub discard_reply: bool,
}

/// Attempt to reuse another actor's parent-funding operation.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct TopupCollision {
    pub cycles: u128,
    pub operation_id: [u8; 32],
}

/// Installed sharding fixture `create_account` contract.
pub const CREATE_ACCOUNT: WireCallSpec = spec("create_account", 0x5445_5354_0000_0600);
/// Installed sharding fixture `test_create_fixture_child` contract.
pub const CREATE_FIXTURE_CHILD: WireCallSpec = WireCallSpec {
    request_limits: OPERATION_REQUEST_LIMITS,
    ..spec("test_create_fixture_child", 0x5445_5354_0000_0601)
};
/// Installed sharding fixture `plan_create_account` contract.
pub const PLAN_ACCOUNT: WireCallSpec = spec("plan_create_account", 0x5445_5354_0000_0602);

/// Exact placement observation with native principals at the endpoint boundary.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub enum IndexStatus {
    Pending {
        owner_pid: ic_platform::Principal,
        created_at: u64,
        provisional_pid: Option<ic_platform::Principal>,
    },
    Bound {
        instance_pid: ic_platform::Principal,
        bound_at: u64,
    },
}

/// Caller-retained child identity and bounded initialization work.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct StartupRequest {
    pub operation_id: [u8; 32],
    pub rounds: u32,
}
/// Installed index fixture `resolve_item` contract.
pub const RESOLVE_ITEM: WireCallSpec = spec("resolve_item", 0x5445_5354_0000_0610);
/// Installed index fixture `test_create_startup_child` contract.
pub const CREATE_STARTUP_CHILD: WireCallSpec = WireCallSpec {
    request_limits: OPERATION_REQUEST_LIMITS,
    ..spec("test_create_startup_child", 0x5445_5354_0000_0611)
};
/// Installed index fixture `test_startup_calls` contract.
pub const STARTUP_CALLS: WireCallSpec = spec("test_startup_calls", 0x5445_5354_0000_0612);

/// Installed caller-authority fixture `test_caller_probe` contract.
pub const CALLER_PROBE: WireCallSpec = spec("test_caller_probe", 0x5445_5354_0000_0620);
/// Installed caller-authority fixture `test_caller_target` contract.
pub const CALLER_TARGET: WireCallSpec = spec("test_caller_target", 0x5445_5354_0000_0621);
/// Installed caller-authority fixture `test_caller_hold` contract.
pub const CALLER_HOLD: WireCallSpec = spec("test_caller_hold", 0x5445_5354_0000_0622);
/// Installed caller-authority fixture `test_caller_effect_status` contract.
pub const CALLER_EFFECT_STATUS: WireCallSpec =
    spec("test_caller_effect_status", 0x5445_5354_0000_0623);
/// Installed caller-authority fixture `test_caller_effect` contract.
pub const CALLER_EFFECT: WireCallSpec = spec("test_caller_effect", 0x5445_5354_0000_0624);

/// Root-only raw balance control stays available while its activation fence is closed.
pub const AUDIT_ROOT_BALANCE: WireCallSpec = WireCallSpec {
    method: "audit_recovery_balance",
    method_id: 0x4155_4454_0000_0016,
    request_limits: ic_wire::Limits {
        bytes: 16_384,
        allocation: 65_536,
        nodes: 4096,
        depth: 32,
    },
    response_limits: ic_wire::Limits {
        bytes: 2_097_152,
        allocation: 8_388_608,
        nodes: 131_072,
        depth: 32,
    },
};

/// Release only the fixture hold under the raw controller guard.
pub const RELEASE_FIXTURE: WireCallSpec = spec("test_release_fixture", 0x5445_5354_0000_0900);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_fixture_budgets_admit_complete_current_requests() {
        for operation_id in [[0; 32], [u8::MAX; 32]] {
            for (spec, encoded) in [
                (
                    CREATE_FIXTURE_CHILD,
                    ic_wire::native::encode_request(
                        CREATE_FIXTURE_CHILD.method_id,
                        &operation_id,
                        CREATE_FIXTURE_CHILD.request_limits,
                    ),
                ),
                (
                    CREATE_STARTUP_CHILD,
                    ic_wire::native::encode_request(
                        CREATE_STARTUP_CHILD.method_id,
                        &StartupRequest {
                            operation_id,
                            rounds: u32::MAX,
                        },
                        CREATE_STARTUP_CHILD.request_limits,
                    ),
                ),
                (
                    TOPUP_COLLISION,
                    ic_wire::native::encode_request(
                        TOPUP_COLLISION.method_id,
                        &TopupCollision {
                            operation_id,
                            cycles: u128::MAX,
                        },
                        TOPUP_COLLISION.request_limits,
                    ),
                ),
            ] {
                let bytes =
                    encoded.expect("complete operation request fits its declared admission");
                assert!(bytes.len() <= spec.request_limits.bytes);
            }
        }
    }
}
