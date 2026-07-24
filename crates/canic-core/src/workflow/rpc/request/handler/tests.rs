use super::*;
use crate::{
    cdk::types::{Cycles, Principal, TC},
    config::schema::{CanisterKind, CyclesFundingPolicyConfig},
    dto::{
        error::ErrorCode,
        rpc::{
            AcknowledgePlacementReceiptRequest, CreateCanisterParent, CreateCanisterRequest,
            CyclesRequest, CyclesResponse, RecycleCanisterRequest, Request, RootRequestMetadata,
            UpgradeCanisterRequest,
        },
    },
    ids::CanisterRole,
    model::replay::{
        CommandKind, ExternalEffectDescriptor, OperationId, REPLAY_PAYLOAD_HASH_SCHEMA_VERSION,
        REPLAY_RECEIPT_SCHEMA_VERSION, RecoveryReason, ReplayActor, ReplayReceiptStatus,
    },
    ops::{
        cost_guard::CostGuardOps,
        replay::{
            guard::secs_to_ns,
            receipt::{mark_external_effect_in_flight, mark_recovery_required},
        },
        runtime::{
            cycles_funding::CyclesFundingLedgerOps,
            metrics::cycles_funding::{
                CyclesFundingDeniedReason, CyclesFundingMetricKey, CyclesFundingMetrics,
            },
        },
        storage::{
            intent::IntentStoreOps, registry::subnet::SubnetRegistryOps, replay::ReplayReceiptOps,
            state::fleet::FleetStateOps,
        },
    },
    replay_policy::CostClass,
    storage::stable::env::{Env, EnvData, EnvRecord},
    storage::stable::replay::ReplayReceiptRecord,
    storage::stable::state::fleet::{FleetMode, FleetStateData, FleetStateRecord},
    test::config::ConfigTestBuilder,
};
use candid::encode_one;
use std::collections::HashMap;

fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

fn meta(id: u8, ttl_ns: u64) -> RootRequestMetadata {
    RootRequestMetadata {
        request_id: [id; 32],
        ttl_ns,
    }
}

fn seed_root_replay_receipt(
    command_kind: &'static str,
    caller: Principal,
    request_id: [u8; 32],
    payload_hash: [u8; 32],
    expires_at_secs: u64,
    response_bytes: Option<Vec<u8>>,
) {
    let command_kind = CommandKind::new(command_kind).expect("command kind");
    let operation_id = OperationId::from_bytes(request_id);
    let key = ReplayReceiptOps::slot_key(&command_kind, operation_id);
    ReplayReceiptOps::upsert(
        key,
        ReplayReceiptRecord {
            schema_version: REPLAY_RECEIPT_SCHEMA_VERSION,
            command_kind: command_kind.as_str().to_string(),
            operation_id: operation_id.into_bytes(),
            actor: ReplayActor::direct_caller(caller),
            payload_hash_schema_version: REPLAY_PAYLOAD_HASH_SCHEMA_VERSION,
            payload_hash,
            status: if response_bytes.is_some() {
                ReplayReceiptStatus::Committed
            } else {
                ReplayReceiptStatus::Reserved
            },
            created_at_ns: secs_to_ns(1),
            updated_at_ns: secs_to_ns(1),
            expires_at_ns: Some(secs_to_ns(expires_at_secs)),
            response_schema_version: response_bytes.as_ref().map(|_| 1),
            response_bytes,
            staged_response_schema_version: None,
            staged_response_bytes: None,
            cost_guard_settlement: None,
            effect: None,
        },
    );
}

///
/// EnvRestore
///

struct EnvRestore(EnvData);

impl Drop for EnvRestore {
    fn drop(&mut self) {
        Env::import(self.0.clone());
    }
}

fn configure_root_env(root_pid: Principal) -> EnvRestore {
    let original = Env::export();
    Env::import(EnvData {
        record: EnvRecord {
            root_pid: Some(root_pid),
            subnet_role: Some(crate::ids::SubnetSlotId::DEFAULT),
            ..EnvRecord::default()
        },
    });
    EnvRestore(original)
}

fn cycles_funding_snapshot_map() -> HashMap<
    (
        CyclesFundingMetricKey,
        Option<Principal>,
        Option<CyclesFundingDeniedReason>,
    ),
    u128,
> {
    CyclesFundingMetrics::snapshot()
        .into_iter()
        .map(|(metric, child, reason, cycles)| ((metric, child, reason), cycles))
        .collect()
}

#[test]
fn root_capability_from_request_maps_provision() {
    let req = Request::CreateCanister(CreateCanisterRequest {
        canister_role: CanisterRole::new("app"),
        parent: CreateCanisterParent::Root,
        extra_arg: None,
        metadata: None,
    });

    let mapped = RootCapability::from_request(req);
    assert_eq!(mapped.descriptor().name, "Provision");
    assert_eq!(mapped.descriptor().command_kind, "root.provision");
}

#[test]
fn root_capability_from_request_maps_placement_receipt_acknowledgement() {
    let mapped = RootCapability::from_request(Request::AcknowledgePlacementReceipt(
        AcknowledgePlacementReceiptRequest {
            operation_id: [1; 32],
            metadata: None,
        },
    ));

    assert_eq!(mapped.descriptor().name, "AcknowledgePlacementReceipt");
    assert_eq!(
        mapped.descriptor().command_kind,
        "root.acknowledge_placement_receipt"
    );
    assert!(mapped.replay_input().is_none());
}

#[test]
fn root_capability_from_request_maps_placement_allocation() {
    let mapped =
        RootCapability::from_request(Request::AllocatePlacementChild(CreateCanisterRequest {
            canister_role: CanisterRole::new("placement"),
            parent: CreateCanisterParent::ThisCanister,
            extra_arg: None,
            metadata: None,
        }));

    assert_eq!(mapped.descriptor().name, "AllocatePlacementChild");
    assert_eq!(
        mapped.descriptor().command_kind,
        "root.allocate_placement_child"
    );
    assert!(mapped.replay_input().is_none());
}

#[test]
fn root_capability_from_request_maps_upgrade() {
    let req = Request::UpgradeCanister(UpgradeCanisterRequest {
        canister_pid: p(1),
        metadata: None,
    });

    let mapped = RootCapability::from_request(req);
    assert_eq!(mapped.descriptor().name, "Upgrade");
}

#[test]
fn root_capability_from_request_maps_recycle_canister() {
    let req = Request::RecycleCanister(RecycleCanisterRequest {
        canister_pid: p(4),
        metadata: None,
    });

    let mapped = RootCapability::from_request(req);
    assert_eq!(mapped.descriptor().name, "RecycleCanister");
}

#[test]
fn root_capability_from_request_maps_cycles() {
    let req = Request::Cycles(CyclesRequest {
        cycles: 42,
        metadata: None,
    });

    let mapped = RootCapability::from_request(req);
    assert_eq!(mapped.descriptor().name, "RequestCycles");
}

#[test]
fn root_capability_metadata_projection_covers_replay_protected_families() {
    let expected = meta(7, secs_to_ns(60));
    let requests = [
        Request::AllocatePlacementChild(CreateCanisterRequest {
            canister_role: CanisterRole::new("placement"),
            parent: CreateCanisterParent::ThisCanister,
            extra_arg: None,
            metadata: None,
        }),
        Request::CreateCanister(CreateCanisterRequest {
            canister_role: CanisterRole::new("app"),
            parent: CreateCanisterParent::Root,
            extra_arg: None,
            metadata: None,
        }),
        Request::UpgradeCanister(UpgradeCanisterRequest {
            canister_pid: p(2),
            metadata: None,
        }),
        Request::RecycleCanister(RecycleCanisterRequest {
            canister_pid: p(3),
            metadata: None,
        }),
        Request::Cycles(CyclesRequest {
            cycles: 100,
            metadata: None,
        }),
    ];

    for request in requests {
        let capability = RootCapability::from_request(request).with_metadata(expected);
        let replay = capability
            .replay_input()
            .expect("projected metadata must produce replay input");
        assert_eq!(replay.metadata, expected);
    }

    let acknowledgement = RootCapability::from_request(Request::AcknowledgePlacementReceipt(
        AcknowledgePlacementReceiptRequest {
            operation_id: [1; 32],
            metadata: None,
        },
    ))
    .with_metadata(expected);
    assert!(acknowledgement.replay_input().is_none());
}

#[test]
fn authorize_recycle_rejects_non_child_caller() {
    let root_pid = p(70);
    let _restore = configure_root_env(root_pid);
    SubnetRegistryOps::register_root(root_pid, 1);

    let caller = p(71);
    let child = p(72);
    let other_parent = p(73);
    SubnetRegistryOps::register_unchecked(
        other_parent,
        &CanisterRole::new("project_hub"),
        root_pid,
        vec![],
        2,
    )
    .expect("register sibling parent");
    SubnetRegistryOps::register_unchecked(
        child,
        &CanisterRole::new("project_instance"),
        other_parent,
        vec![],
        3,
    )
    .expect("register child");

    let ctx = RootContext {
        caller,
        self_pid: root_pid,
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::RecycleCanister(RecycleCanisterRequest {
        canister_pid: child,
        metadata: None,
    });

    let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
    assert_eq!(err.class(), crate::InternalErrorClass::Workflow);
}

#[test]
fn authorize_recycle_allows_direct_child_caller() {
    let root_pid = p(80);
    let _restore = configure_root_env(root_pid);
    SubnetRegistryOps::register_root(root_pid, 1);

    let caller = p(81);
    let child = p(82);
    SubnetRegistryOps::register_unchecked(
        caller,
        &CanisterRole::new("project_hub"),
        root_pid,
        vec![],
        2,
    )
    .expect("register parent");
    SubnetRegistryOps::register_unchecked(
        child,
        &CanisterRole::new("project_instance"),
        caller,
        vec![],
        3,
    )
    .expect("register child");

    let ctx = RootContext {
        caller,
        self_pid: root_pid,
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::RecycleCanister(RecycleCanisterRequest {
        canister_pid: child,
        metadata: None,
    });

    RootResponseWorkflow::authorize(&ctx, &capability)
        .expect("direct child recycle must authorize");
}

#[test]
fn authorize_denies_non_root_context() {
    let ctx = RootContext {
        caller: p(1),
        self_pid: p(9),
        is_root_env: false,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::ProvisionCanister(CreateCanisterRequest {
        canister_role: CanisterRole::new("app"),
        parent: CreateCanisterParent::Root,
        extra_arg: None,
        metadata: None,
    });

    let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
    assert_eq!(err.origin(), crate::InternalErrorOrigin::Ops);
}

#[test]
fn authorize_allows_provision_in_root_context() {
    let root_pid = p(9);
    let ctx = RootContext {
        caller: root_pid,
        self_pid: root_pid,
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::ProvisionCanister(CreateCanisterRequest {
        canister_role: CanisterRole::new("app"),
        parent: CreateCanisterParent::Root,
        extra_arg: None,
        metadata: None,
    });

    RootResponseWorkflow::authorize(&ctx, &capability).expect("must authorize");
}

#[test]
fn authorize_allows_structural_child_provision_for_registered_caller() {
    let root_pid = p(11);
    let caller = p(12);
    SubnetRegistryOps::register_root(root_pid, 1);
    SubnetRegistryOps::register_unchecked(
        caller,
        &CanisterRole::new("user_hub"),
        root_pid,
        vec![],
        2,
    )
    .expect("register caller");

    let ctx = RootContext {
        caller,
        self_pid: root_pid,
        is_root_env: true,
        subnet_id: root_pid,
        now: 5,
    };
    let capability = RootCapability::ProvisionCanister(CreateCanisterRequest {
        canister_role: CanisterRole::new("user_shard"),
        parent: CreateCanisterParent::ThisCanister,
        extra_arg: None,
        metadata: None,
    });

    RootResponseWorkflow::authorize(&ctx, &capability).expect("must authorize child provision");
}

#[test]
fn authorize_rejects_structural_child_provision_with_root_parent() {
    let root_pid = p(13);
    let caller = p(14);
    SubnetRegistryOps::register_root(root_pid, 1);
    SubnetRegistryOps::register_unchecked(
        caller,
        &CanisterRole::new("user_hub"),
        root_pid,
        vec![],
        2,
    )
    .expect("register caller");

    let ctx = RootContext {
        caller,
        self_pid: root_pid,
        is_root_env: true,
        subnet_id: root_pid,
        now: 5,
    };
    let capability = RootCapability::ProvisionCanister(CreateCanisterRequest {
        canister_role: CanisterRole::new("user_shard"),
        parent: CreateCanisterParent::Root,
        extra_arg: None,
        metadata: None,
    });

    let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
    assert_eq!(
        err.public_error()
            .expect("structural parent denial is public")
            .code,
        ErrorCode::Forbidden
    );
}

#[test]
fn root_provision_cost_guard_request_uses_deployment_policy() {
    let ctx = RootContext {
        caller: p(95),
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(2),
        now: 9_500,
    };
    let guard_request =
        execute::root_provision_cost_guard_request(&ctx, 5 * TC, 100 * TC, "root.provision");

    assert_eq!(guard_request.cost_class, CostClass::ManagementDeployment);
    assert_eq!(guard_request.command_kind.as_str(), "root.provision");
    assert_eq!(guard_request.quota_subject, ctx.caller);
    assert_eq!(guard_request.payer, ctx.self_pid);
    assert_eq!(guard_request.now_secs, ctx.now);
    assert_eq!(guard_request.quota_window_secs, 60);
    assert_eq!(guard_request.max_operations_per_window, 10);
    assert_eq!(guard_request.cycle_reservation_cycles, 5 * TC);
    assert_eq!(guard_request.min_cycles_after_reservation, TC);
}

#[test]
fn root_provision_marks_create_external_effect() {
    ReplayReceiptOps::reset_for_tests();
    CostGuardOps::reset_for_tests();

    let ctx = RootContext {
        caller: p(96),
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(2),
        now: 1_000,
    };
    let req = CreateCanisterRequest {
        canister_role: CanisterRole::new("project_hub"),
        parent: CreateCanisterParent::Root,
        extra_arg: None,
        metadata: Some(meta(31, secs_to_ns(60))),
    };
    let capability = RootCapability::ProvisionCanister(req.clone());
    let pending = match RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect("first replay should reserve")
    {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };

    let permit = CostGuardOps::reserve(execute::root_provision_cost_guard_request(
        &ctx,
        5 * TC,
        10 * TC,
        "root.provision",
    ))
    .expect("cost permit");
    execute::mark_root_provision_external_effect(
        &pending,
        &ctx,
        &req,
        p(42),
        &permit,
        "root.provision",
    )
    .expect("mark provision effect");

    let receipt = ReplayReceiptOps::get(pending.receipt_token.key())
        .expect("receipt")
        .into_receipt()
        .expect("receipt decodes");
    assert_eq!(receipt.status, ReplayReceiptStatus::ExternalEffectInFlight);
    assert_eq!(
        receipt.cost_guard_settlement,
        Some(permit.replay_settlement())
    );
    assert_eq!(
        receipt.effect,
        Some(ExternalEffectDescriptor::ManagementCreateCanister {
            command_kind: CommandKind::new("root.provision").expect("command kind"),
        })
    );

    CostGuardOps::abort(&permit).expect("abort cost permit");
    ReplayReceiptOps::reset_for_tests();
}

#[test]
fn preflight_validates_replay_before_policy() {
    ReplayReceiptOps::reset_for_tests();

    let ctx = RootContext {
        caller: p(1),
        self_pid: p(9),
        is_root_env: false,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::RequestCycles(CyclesRequest {
        cycles: 42,
        metadata: None,
    });

    let err = RootResponseWorkflow::preflight(&ctx, &capability)
        .expect_err("preflight should validate replay first");
    let public = err
        .public_error()
        .expect("missing operation id is a public hard-cut error");
    assert_eq!(public.code, ErrorCode::OperationIdRequired);
}

#[test]
fn preflight_aborts_reserved_replay_on_policy_denial() {
    ReplayReceiptOps::reset_for_tests();

    let ctx = RootContext {
        caller: p(1),
        self_pid: p(9),
        is_root_env: false,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::RequestCycles(CyclesRequest {
        cycles: 42,
        metadata: Some(meta(7, secs_to_ns(60))),
    });

    RootResponseWorkflow::preflight(&ctx, &capability)
        .expect_err("policy denial should fail preflight");

    let replay = RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect("denied preflight must not leave replay slot in-flight");
    let pending = match replay {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => {
            panic!("policy-denied replay should not return cached response")
        }
    };
    replay::abort_replay(pending).expect("abort replay");
}

#[test]
fn authorize_request_cycles_records_requested_and_child_not_found_denial_metrics() {
    CyclesFundingMetrics::reset();
    CyclesFundingLedgerOps::reset_for_tests();

    let child = p(71);
    let ctx = RootContext {
        caller: child,
        self_pid: p(9),
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::RequestCycles(CyclesRequest {
        cycles: 42,
        metadata: Some(meta(22, secs_to_ns(60))),
    });

    RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");

    let map = cycles_funding_snapshot_map();
    assert_eq!(
        map.get(&(CyclesFundingMetricKey::RequestedTotal, None, None)),
        Some(&42)
    );
    assert_eq!(
        map.get(&(CyclesFundingMetricKey::RequestedByChild, Some(child), None)),
        Some(&42)
    );
    assert_eq!(
        map.get(&(CyclesFundingMetricKey::DeniedTotal, None, None)),
        Some(&42)
    );
    assert_eq!(
        map.get(&(
            CyclesFundingMetricKey::DeniedToChild,
            Some(child),
            Some(CyclesFundingDeniedReason::ChildNotFound),
        )),
        Some(&42)
    );
    assert!(
        !map.contains_key(&(
            CyclesFundingMetricKey::DeniedGlobalKillSwitch,
            None,
            Some(CyclesFundingDeniedReason::KillSwitchDisabled),
        )),
        "kill-switch metric must not increment for child-not-found denial"
    );
}

#[test]
fn authorize_request_cycles_records_kill_switch_denial_metrics() {
    CyclesFundingMetrics::reset();
    CyclesFundingLedgerOps::reset_for_tests();

    let self_pid = p(90);
    let child = p(91);
    SubnetRegistryOps::register_root(self_pid, 1);
    SubnetRegistryOps::register_unchecked(child, &CanisterRole::new("test"), self_pid, vec![], 2)
        .expect("register child");

    FleetStateOps::import(FleetStateData {
        record: FleetStateRecord {
            mode: FleetMode::Enabled,
            cycles_funding_enabled: false,
        },
    });

    let ctx = RootContext {
        caller: child,
        self_pid,
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::RequestCycles(CyclesRequest {
        cycles: 33,
        metadata: Some(meta(23, secs_to_ns(60))),
    });

    let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
    assert_eq!(
        err.public_error()
            .expect("kill-switch denial is public")
            .code,
        ErrorCode::Unavailable
    );

    let map = cycles_funding_snapshot_map();
    assert_eq!(
        map.get(&(CyclesFundingMetricKey::RequestedTotal, None, None)),
        Some(&33)
    );
    assert_eq!(
        map.get(&(CyclesFundingMetricKey::DeniedTotal, None, None)),
        Some(&33)
    );
    assert_eq!(
        map.get(&(
            CyclesFundingMetricKey::DeniedToChild,
            Some(child),
            Some(CyclesFundingDeniedReason::KillSwitchDisabled),
        )),
        Some(&33)
    );
    assert_eq!(
        map.get(&(
            CyclesFundingMetricKey::DeniedGlobalKillSwitch,
            None,
            Some(CyclesFundingDeniedReason::KillSwitchDisabled),
        )),
        Some(&33)
    );

    FleetStateOps::import(FleetStateData {
        record: FleetStateRecord {
            mode: FleetMode::Enabled,
            cycles_funding_enabled: true,
        },
    });
}

#[test]
fn authorize_request_cycles_uses_configured_child_funding_policy() {
    CyclesFundingMetrics::reset();
    CyclesFundingLedgerOps::reset_for_tests();

    let self_pid = p(95);
    let child = p(96);
    let child_role = CanisterRole::new("funded_child");
    let _restore = configure_root_env(self_pid);

    let mut child_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
    child_cfg.cycles_funding = CyclesFundingPolicyConfig {
        max_per_request: Cycles::new(10),
        max_per_child: Cycles::new(30),
        cooldown_secs: 60,
    };
    let _config = ConfigTestBuilder::new()
        .with_prime_canister(child_role.clone(), child_cfg)
        .install();

    SubnetRegistryOps::register_root(self_pid, 1);
    SubnetRegistryOps::register_unchecked(child, &child_role, self_pid, vec![], 2)
        .expect("register child");

    FleetStateOps::import(FleetStateData {
        record: FleetStateRecord {
            mode: FleetMode::Enabled,
            cycles_funding_enabled: true,
        },
    });

    let ctx = RootContext {
        caller: child,
        self_pid,
        is_root_env: true,
        subnet_id: p(2),
        now: 1_000,
    };
    CyclesFundingLedgerOps::record_child_grant(child, 30, 1);

    let req = CyclesRequest {
        cycles: 1,
        metadata: Some(meta(24, secs_to_ns(60))),
    };

    let err = nonroot_cycles::authorize_root_request_cycles_plan(&ctx, &req)
        .expect_err("configured child budget must deny");
    assert_eq!(
        err.public_error()
            .expect("child budget denial is public")
            .code,
        ErrorCode::ResourceExhausted
    );
}

#[test]
fn authorize_request_cycles_rejects_a_competing_pending_child_operation() {
    ReplayReceiptOps::reset_for_tests();
    CyclesFundingMetrics::reset();
    CyclesFundingLedgerOps::reset_for_tests();

    let self_pid = p(97);
    let child = p(98);
    let child_role = CanisterRole::new("concurrent_funded_child");
    let _restore = configure_root_env(self_pid);

    let mut child_cfg = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
    child_cfg.cycles_funding = CyclesFundingPolicyConfig {
        max_per_request: Cycles::new(10),
        max_per_child: Cycles::new(30),
        cooldown_secs: 1,
    };
    let _config = ConfigTestBuilder::new()
        .with_prime_canister(child_role.clone(), child_cfg)
        .install();

    SubnetRegistryOps::register_root(self_pid, 1);
    SubnetRegistryOps::register_unchecked(child, &child_role, self_pid, vec![], 2)
        .expect("register child");
    FleetStateOps::import(FleetStateData {
        record: FleetStateRecord {
            mode: FleetMode::Enabled,
            cycles_funding_enabled: true,
        },
    });

    seed_root_replay_receipt("root.request_cycles.v1", child, [40; 32], [4; 32], 60, None);
    let ctx = RootContext {
        caller: child,
        self_pid,
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let req = CyclesRequest {
        cycles: 5,
        metadata: Some(meta(41, secs_to_ns(60))),
    };

    let err = nonroot_cycles::authorize_root_request_cycles_plan(&ctx, &req)
        .expect_err("a second child funding operation must reject before ledger mutation");

    assert_eq!(
        err.public_error()
            .expect("concurrent funding denial is public")
            .code,
        ErrorCode::Conflict
    );
    assert_eq!(
        CyclesFundingLedgerOps::snapshot(child),
        crate::model::cycles_funding::FundingLedgerSnapshot::default()
    );
    let metrics = cycles_funding_snapshot_map();
    assert_eq!(
        metrics.get(&(
            CyclesFundingMetricKey::DeniedToChild,
            Some(child),
            Some(CyclesFundingDeniedReason::OperationInProgress),
        )),
        Some(&5)
    );

    ReplayReceiptOps::reset_for_tests();
}

#[test]
fn request_cycles_cost_guard_request_uses_value_transfer_policy() {
    let ctx = RootContext {
        caller: p(92),
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(2),
        now: 9_000,
    };
    let guard_request =
        nonroot_cycles::request_cycles_cost_guard_request(&ctx, 2_000, 5_000_000_000);

    assert_eq!(guard_request.cost_class, CostClass::ValueTransfer);
    assert_eq!(
        guard_request.command_kind.as_str(),
        "root.request_cycles.v1"
    );
    assert_eq!(guard_request.quota_subject, ctx.caller);
    assert_eq!(guard_request.payer, ctx.self_pid);
    assert_eq!(guard_request.now_secs, ctx.now);
    assert_eq!(guard_request.quota_window_secs, 60);
    assert_eq!(guard_request.max_operations_per_window, 60);
    assert_eq!(guard_request.cycle_reservation_cycles, 2_000);
    assert_eq!(guard_request.min_cycles_after_reservation, 1_000_000_000);
}

#[test]
fn request_cycles_value_transfer_cost_guard_enforces_actor_quota() {
    let ctx = RootContext {
        caller: p(93),
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(2),
        now: 12_000,
    };

    for _ in 0..60 {
        let permit = CostGuardOps::reserve(nonroot_cycles::request_cycles_cost_guard_request(
            &ctx,
            1_000,
            5_000_000_000,
        ))
        .expect("quota reservation");
        CostGuardOps::complete(&permit, ctx.now).expect("complete quota reservation");
    }

    let err = CostGuardOps::reserve(nonroot_cycles::request_cycles_cost_guard_request(
        &ctx,
        1_000,
        5_000_000_000,
    ))
    .expect_err("same actor quota bucket exhausted");
    let err = crate::workflow::cost_guard::map_cost_guard_reserve_error(err);
    assert_eq!(
        err.public_error().expect("quota rejection is public").code,
        ErrorCode::ResourceExhausted
    );
}

#[test]
fn request_cycles_marks_deposit_external_effect() {
    ReplayReceiptOps::reset_for_tests();

    let ctx = RootContext {
        caller: p(94),
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(2),
        now: 1_000,
    };
    let capability = RootCapability::RequestCycles(CyclesRequest {
        cycles: 77,
        metadata: Some(meta(29, secs_to_ns(60))),
    });
    let pending = match RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect("first replay should reserve")
    {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };

    CostGuardOps::reset_for_tests();
    let permit = CostGuardOps::reserve(nonroot_cycles::request_cycles_cost_guard_request(
        &ctx,
        77,
        10 * TC,
    ))
    .expect("cost permit");
    nonroot_cycles::mark_request_cycles_external_effect(&pending, &ctx, 77, &permit)
        .expect("mark cycles effect");

    let receipt = ReplayReceiptOps::get(pending.receipt_token.key())
        .expect("receipt")
        .into_receipt()
        .expect("receipt decodes");
    assert_eq!(receipt.status, ReplayReceiptStatus::ExternalEffectInFlight);
    assert_eq!(
        receipt.cost_guard_settlement,
        Some(permit.replay_settlement())
    );
    assert_eq!(
        receipt.effect,
        Some(ExternalEffectDescriptor::ManagementCall {
            canister: ctx.caller,
            method: "deposit_cycles".to_string(),
        })
    );

    replay::abort_replay(pending).expect("abort replay");
    CostGuardOps::abort(&permit).expect("abort cost permit");
    CostGuardOps::reset_for_tests();
}

#[test]
fn request_cycles_releases_cost_reservation_when_effect_marking_fails() {
    ReplayReceiptOps::reset_for_tests();
    CostGuardOps::reset_for_tests();

    let now = crate::ops::ic::IcOps::now_secs();
    let ctx = RootContext {
        caller: p(94),
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(2),
        now,
    };
    let capability = RootCapability::RequestCycles(CyclesRequest {
        cycles: 77,
        metadata: Some(meta(30, secs_to_ns(60))),
    });
    let pending = match RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect("first replay should reserve")
    {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };
    let permit = CostGuardOps::reserve(nonroot_cycles::request_cycles_cost_guard_request(
        &ctx,
        77,
        10 * TC,
    ))
    .expect("cost permit");
    let settlement = permit.replay_settlement();
    assert_eq!(IntentStoreOps::pending_total().expect("pending intents"), 2);

    ReplayReceiptOps::remove(pending.receipt_token.key()).expect("remove replay receipt");
    let err = nonroot_cycles::mark_request_cycles_external_effect(&pending, &ctx, 77, &permit)
        .expect_err("missing replay receipt must reject before the external effect");

    assert_eq!(
        err.log_fields(),
        (
            crate::InternalErrorClass::Workflow,
            crate::InternalErrorOrigin::Workflow,
        )
    );
    assert_eq!(IntentStoreOps::pending_total().expect("pending intents"), 0);
    assert!(
        IntentStoreOps::is_committed_for_tests(settlement.quota_intent_id)
            .expect("quota intent state remains readable")
    );
    assert!(
        IntentStoreOps::is_aborted_for_tests(settlement.reservation_intent_id)
            .expect("cycle reservation intent state remains readable")
    );

    ReplayReceiptOps::reset_for_tests();
    CostGuardOps::reset_for_tests();
}

#[test]
fn payload_hash_ignores_metadata() {
    let hash_a = RootCapability::RequestCycles(CyclesRequest {
        cycles: 42,
        metadata: Some(meta(1, secs_to_ns(60))),
    })
    .payload_hash();
    let hash_b = RootCapability::RequestCycles(CyclesRequest {
        cycles: 42,
        metadata: Some(meta(9, secs_to_ns(120))),
    })
    .payload_hash();

    assert_eq!(hash_a, hash_b, "metadata must not affect payload hash");
}

#[test]
fn payload_hash_includes_capability_variant_discriminant() {
    let capability_hash = RootCapability::RequestCycles(CyclesRequest {
        cycles: 42,
        metadata: None,
    })
    .payload_hash();

    let struct_only_hash = {
        let bytes = encode_one(&CyclesRequest {
            cycles: 42,
            metadata: None,
        })
        .expect("encode");
        hash_domain_separated(REPLAY_PAYLOAD_HASH_DOMAIN, &bytes)
    };

    assert_ne!(
        capability_hash, struct_only_hash,
        "capability payload hash must include variant discriminant"
    );
}

#[test]
fn check_replay_rejects_invalid_ttl() {
    ReplayReceiptOps::reset_for_tests();

    let ctx = RootContext {
        caller: p(1),
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(2),
        now: 1_000,
    };

    let too_small = RootCapability::RequestCycles(CyclesRequest {
        cycles: 77,
        metadata: Some(meta(7, 0)),
    });
    let err = RootResponseWorkflow::check_replay(&ctx, &too_small).expect_err("must reject");
    assert_eq!(err.class(), crate::InternalErrorClass::Workflow);

    let too_large = RootCapability::RequestCycles(CyclesRequest {
        cycles: 77,
        metadata: Some(meta(7, MAX_ROOT_TTL_NS + 1)),
    });
    let err = RootResponseWorkflow::check_replay(&ctx, &too_large).expect_err("must reject");
    assert_eq!(err.class(), crate::InternalErrorClass::Workflow);
}

#[test]
fn check_replay_rejects_expired_entry() {
    ReplayReceiptOps::reset_for_tests();

    let ctx = RootContext {
        caller: p(7),
        self_pid: p(55),
        is_root_env: true,
        subnet_id: p(8),
        now: 10_000,
    };
    let capability = RootCapability::RequestCycles(CyclesRequest {
        cycles: 500,
        metadata: Some(meta(11, secs_to_ns(60))),
    });
    let replay_input = capability.replay_input().expect("metadata");
    let payload_hash = replay_input.payload_hash;
    let request_id = replay_input.metadata.request_id;
    let response_bytes = encode_one(Response::Cycles(CyclesResponse {
        cycles_transferred: 500,
    }))
    .expect("encode");

    seed_root_replay_receipt(
        replay_input.descriptor.command_kind,
        ctx.caller,
        request_id,
        payload_hash,
        9_999,
        Some(response_bytes),
    );

    RootResponseWorkflow::check_replay(&ctx, &capability).expect_err("must expire");
}

#[test]
fn check_replay_returns_cached_response_for_duplicate_same_payload() {
    ReplayReceiptOps::reset_for_tests();

    let ctx = RootContext {
        caller: p(1),
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(2),
        now: 1_000,
    };
    let capability = RootCapability::RequestCycles(CyclesRequest {
        cycles: 77,
        metadata: Some(meta(7, secs_to_ns(60))),
    });

    let pending = match RootResponseWorkflow::check_replay(&ctx, &capability).expect("first replay")
    {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };

    let response = Response::Cycles(CyclesResponse {
        cycles_transferred: 77,
    });
    replay::stage_response(&pending, &response).expect("stage response");
    RootResponseWorkflow::commit_replay(&pending).expect("commit");

    let preflight = RootResponseWorkflow::check_replay(&ctx, &capability).expect("must cache-hit");
    match preflight {
        replay::ReplayPreflight::Cached(Response::Cycles(response)) => {
            assert_eq!(response.cycles_transferred, 77);
        }
        replay::ReplayPreflight::Cached(other) => {
            panic!("expected cached cycles response, got: {other:?}");
        }
        replay::ReplayPreflight::Fresh(_) => panic!("duplicate replay must return cached response"),
    }
}

#[test]
fn abort_replay_preserves_recovery_required_external_effect_receipt() {
    ReplayReceiptOps::reset_for_tests();

    let ctx = RootContext {
        caller: p(5),
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(6),
        now: 1_000,
    };
    let capability = RootCapability::RequestCycles(CyclesRequest {
        cycles: 77,
        metadata: Some(meta(17, secs_to_ns(60))),
    });

    let pending = match RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect("first replay should reserve")
    {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };
    let key = pending.receipt_token.key();
    let effect = ExternalEffectDescriptor::ManagementCall {
        canister: ctx.caller,
        method: "deposit_cycles".to_string(),
    };

    mark_external_effect_in_flight(&pending.receipt_token, effect.clone(), secs_to_ns(1_001))
        .expect("mark external effect");
    mark_recovery_required(
        &pending.receipt_token,
        RecoveryReason::ExternalEffectStatusUnknown,
        secs_to_ns(1_002),
    )
    .expect("mark recovery required");
    replay::abort_replay(pending).expect("abort replay");

    let receipt = ReplayReceiptOps::get(key)
        .expect("recovery receipt must remain")
        .into_receipt()
        .expect("receipt decodes");
    assert_eq!(
        receipt.status,
        ReplayReceiptStatus::RecoveryRequired {
            reason: RecoveryReason::ExternalEffectStatusUnknown,
        }
    );
    assert_eq!(receipt.effect, Some(effect));

    RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect_err("recovery-required duplicate must not run fresh");
}

#[test]
fn abort_replay_cleanup_failure_preserves_primary_error_projection() {
    ReplayReceiptOps::reset_for_tests();

    let ctx = RootContext {
        caller: p(5),
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(6),
        now: 1_000,
    };
    let capability = RootCapability::RequestCycles(CyclesRequest {
        cycles: 77,
        metadata: Some(meta(18, secs_to_ns(60))),
    });
    let pending = match RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect("first replay should reserve")
    {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };
    let key = pending.receipt_token.key();
    let mut record = ReplayReceiptOps::get(key).expect("reserved receipt");
    record.schema_version = u32::MAX;
    ReplayReceiptOps::upsert(key, record);

    let error = replay::abort_replay_after_failure(
        pending,
        InternalError::public(crate::dto::error::Error::conflict("primary failure")),
    );

    assert_eq!(
        error.public_error().map(|error| error.code),
        Some(ErrorCode::Conflict)
    );
}

#[test]
fn check_replay_finishes_cost_settlement_without_reexecuting_capability() {
    ReplayReceiptOps::reset_for_tests();
    CostGuardOps::reset_for_tests();

    let ctx = RootContext {
        caller: p(5),
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(6),
        now: 1_000,
    };
    let req = CreateCanisterRequest {
        canister_role: CanisterRole::new("project_hub"),
        parent: CreateCanisterParent::Root,
        extra_arg: None,
        metadata: Some(meta(18, secs_to_ns(60))),
    };
    let capability = RootCapability::ProvisionCanister(req.clone());
    let pending = match RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect("first replay should reserve")
    {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };
    let permit = CostGuardOps::reserve(execute::root_provision_cost_guard_request(
        &ctx,
        5 * TC,
        10 * TC,
        "root.provision",
    ))
    .expect("cost permit");
    execute::mark_root_provision_external_effect(
        &pending,
        &ctx,
        &req,
        p(42),
        &permit,
        "root.provision",
    )
    .expect("mark costed effect");
    let response = Response::CreateCanister(crate::dto::rpc::CreateCanisterResponse {
        new_canister_pid: p(99),
    });
    replay::stage_response(&pending, &response).expect("stage terminal response");
    replay::mark_recovery_required(&pending, RecoveryReason::CostSettlementFailed)
        .expect("mark settlement recovery");

    let recovered = RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect("identical retry should finish accounting");
    match recovered {
        replay::ReplayPreflight::Cached(Response::CreateCanister(response)) => {
            assert_eq!(response.new_canister_pid, p(99));
        }
        other => panic!("expected cached recovered response, got {other:?}"),
    }
    let receipt = ReplayReceiptOps::get(pending.receipt_token.key())
        .expect("receipt")
        .into_receipt()
        .expect("receipt decodes");
    assert_eq!(receipt.status, ReplayReceiptStatus::Committed);

    ReplayReceiptOps::reset_for_tests();
    CostGuardOps::reset_for_tests();
}

#[test]
fn request_cycles_retry_finishes_cost_settlement_without_reexecuting_deposit() {
    ReplayReceiptOps::reset_for_tests();
    CostGuardOps::reset_for_tests();

    let ctx = RootContext {
        caller: p(5),
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(6),
        now: 1_000,
    };
    let capability = RootCapability::RequestCycles(CyclesRequest {
        cycles: 77,
        metadata: Some(meta(19, secs_to_ns(60))),
    });
    let pending = match RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect("first replay should reserve")
    {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };
    let permit = CostGuardOps::reserve(nonroot_cycles::request_cycles_cost_guard_request(
        &ctx,
        77,
        10 * TC,
    ))
    .expect("cost permit");
    nonroot_cycles::mark_request_cycles_external_effect(&pending, &ctx, 77, &permit)
        .expect("mark costed cycles effect");
    replay::stage_response(
        &pending,
        &Response::Cycles(CyclesResponse {
            cycles_transferred: 77,
        }),
    )
    .expect("stage terminal response");
    replay::mark_recovery_required(&pending, RecoveryReason::CostSettlementFailed)
        .expect("mark settlement recovery");

    let recovered = RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect("identical retry should finish accounting");
    assert!(matches!(
        recovered,
        replay::ReplayPreflight::Cached(Response::Cycles(CyclesResponse {
            cycles_transferred: 77
        }))
    ));
    let receipt = ReplayReceiptOps::get(pending.receipt_token.key())
        .expect("receipt")
        .into_receipt()
        .expect("receipt decodes");
    assert_eq!(receipt.status, ReplayReceiptStatus::Committed);

    ReplayReceiptOps::reset_for_tests();
    CostGuardOps::reset_for_tests();
}

#[test]
fn cost_recovery_without_staged_response_transitions_to_response_recovery() {
    ReplayReceiptOps::reset_for_tests();
    CostGuardOps::reset_for_tests();

    let ctx = RootContext {
        caller: p(5),
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(6),
        now: 1_000,
    };
    let capability = RootCapability::RequestCycles(CyclesRequest {
        cycles: 77,
        metadata: Some(meta(21, secs_to_ns(60))),
    });
    let pending = match RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect("first replay should reserve")
    {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };
    let permit = CostGuardOps::reserve(nonroot_cycles::request_cycles_cost_guard_request(
        &ctx,
        77,
        10 * TC,
    ))
    .expect("cost permit");
    nonroot_cycles::mark_request_cycles_external_effect(&pending, &ctx, 77, &permit)
        .expect("mark costed cycles effect");
    replay::mark_recovery_required(&pending, RecoveryReason::CostSettlementFailed)
        .expect("mark settlement recovery");

    RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect_err("missing staged response must fail closed after settlement");
    let receipt = ReplayReceiptOps::get(pending.receipt_token.key())
        .expect("receipt")
        .into_receipt()
        .expect("receipt decodes");
    assert_eq!(
        receipt.status,
        ReplayReceiptStatus::RecoveryRequired {
            reason: RecoveryReason::ResponseCommitFailed
        }
    );

    ReplayReceiptOps::reset_for_tests();
    CostGuardOps::reset_for_tests();
}

#[test]
fn response_commit_retry_promotes_staged_response_without_reexecution() {
    ReplayReceiptOps::reset_for_tests();

    let ctx = RootContext {
        caller: p(5),
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(6),
        now: 1_000,
    };
    let capability = RootCapability::RecycleCanister(RecycleCanisterRequest {
        canister_pid: p(55),
        metadata: Some(meta(20, secs_to_ns(60))),
    });
    let pending = match RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect("first replay should reserve")
    {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };
    replay::stage_response(&pending, &Response::RecycleCanister).expect("stage terminal response");
    replay::mark_recovery_required(&pending, RecoveryReason::ResponseCommitFailed)
        .expect("mark response recovery");

    let recovered = RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect("identical retry should promote staged response");
    assert!(matches!(
        recovered,
        replay::ReplayPreflight::Cached(Response::RecycleCanister)
    ));
    let receipt = ReplayReceiptOps::get(pending.receipt_token.key())
        .expect("receipt")
        .into_receipt()
        .expect("receipt decodes");
    assert_eq!(receipt.status, ReplayReceiptStatus::Committed);

    ReplayReceiptOps::reset_for_tests();
}

#[test]
fn check_replay_rejects_conflicting_payload_for_same_request_id() {
    ReplayReceiptOps::reset_for_tests();

    let ctx = RootContext {
        caller: p(3),
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(4),
        now: 2_000,
    };
    let base = RootCapability::RequestCycles(CyclesRequest {
        cycles: 10,
        metadata: Some(meta(8, secs_to_ns(60))),
    });
    let conflict = RootCapability::RequestCycles(CyclesRequest {
        cycles: 11,
        metadata: Some(meta(8, secs_to_ns(60))),
    });

    let pending = match RootResponseWorkflow::check_replay(&ctx, &base).expect("first replay") {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };
    replay::stage_response(
        &pending,
        &Response::Cycles(CyclesResponse {
            cycles_transferred: 10,
        }),
    )
    .expect("stage response");
    RootResponseWorkflow::commit_replay(&pending).expect("commit");

    RootResponseWorkflow::check_replay(&ctx, &conflict).expect_err("must conflict");
}

#[test]
fn check_replay_rejects_cross_variant_same_request_id() {
    ReplayReceiptOps::reset_for_tests();

    let ctx = RootContext {
        caller: p(3),
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(4),
        now: 2_000,
    };
    let upgrade = RootCapability::UpgradeCanister(UpgradeCanisterRequest {
        canister_pid: p(9),
        metadata: Some(meta(8, secs_to_ns(60))),
    });
    let cycles = RootCapability::RequestCycles(CyclesRequest {
        cycles: 11,
        metadata: Some(meta(8, secs_to_ns(60))),
    });

    let pending = match RootResponseWorkflow::check_replay(&ctx, &upgrade).expect("first replay") {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };
    replay::stage_response(&pending, &Response::UpgradeCanister).expect("stage response");
    RootResponseWorkflow::commit_replay(&pending).expect("commit");

    RootResponseWorkflow::check_replay(&ctx, &cycles).expect_err("must conflict");
}

#[test]
fn replay_purge_respects_limit_and_keeps_unexpired_entries() {
    ReplayReceiptOps::reset_for_tests();

    let ok = encode_one(Response::UpgradeCanister).expect("encode");

    for i in 0..5u8 {
        seed_root_replay_receipt(
            "root.upgrade.v1",
            p(i),
            [i; 32],
            [i; 32],
            10,
            Some(ok.clone()),
        );
    }

    for i in 200..202u8 {
        seed_root_replay_receipt(
            "root.upgrade.v1",
            p(i),
            [i; 32],
            [i; 32],
            999,
            Some(ok.clone()),
        );
    }

    let purged = ReplayReceiptOps::purge_expired(secs_to_ns(100), 3);
    assert_eq!(purged, 3, "purge must stop at the configured limit");
    assert_eq!(
        ReplayReceiptOps::len(),
        4,
        "expected 4 entries after first purge"
    );

    let purged = ReplayReceiptOps::purge_expired(secs_to_ns(100), 10);
    assert_eq!(purged, 2, "remaining expired entries must be purged");
    assert_eq!(
        ReplayReceiptOps::len(),
        2,
        "only unexpired entries should remain"
    );
}

#[test]
fn check_replay_rejects_when_capacity_reached() {
    ReplayReceiptOps::reset_for_tests();

    let response_bytes = encode_one(Response::Cycles(CyclesResponse {
        cycles_transferred: 1,
    }))
    .expect("encode");

    for i in 0..MAX_ROOT_REPLAY_ENTRIES {
        let mut request_id = [0u8; 32];
        request_id[..8].copy_from_slice(&(i as u64).to_be_bytes());

        seed_root_replay_receipt(
            "root.request_cycles.v1",
            p(u8::try_from(i % 250).expect("modulo keeps caller byte below u8 max")),
            request_id,
            [0u8; 32],
            5_000,
            Some(response_bytes.clone()),
        );
    }

    let ctx = RootContext {
        caller: p(1),
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(2),
        now: 1_000,
    };
    let capability = RootCapability::RequestCycles(CyclesRequest {
        cycles: 77,
        metadata: Some(meta(7, secs_to_ns(60))),
    });
    RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect_err("reservation must fail when store is at capacity");
}

#[test]
fn placement_receipt_acknowledgement_does_not_reserve_replay_capacity() {
    ReplayReceiptOps::reset_for_tests();

    let response_bytes = encode_one(Response::Cycles(CyclesResponse {
        cycles_transferred: 1,
    }))
    .expect("encode");
    for i in 0..MAX_ROOT_REPLAY_ENTRIES {
        let mut request_id = [0u8; 32];
        request_id[..8].copy_from_slice(&(i as u64).to_be_bytes());
        seed_root_replay_receipt(
            "root.request_cycles.v1",
            p(u8::try_from(i % 250).expect("modulo keeps caller byte below u8 max")),
            request_id,
            [0; 32],
            5_000,
            Some(response_bytes.clone()),
        );
    }

    let acknowledgement =
        RootCapability::AcknowledgePlacementReceipt(AcknowledgePlacementReceiptRequest {
            operation_id: [1; 32],
            metadata: Some(meta(7, secs_to_ns(60))),
        });
    assert!(acknowledgement.replay_input().is_none());
    assert_eq!(ReplayReceiptOps::len(), MAX_ROOT_REPLAY_ENTRIES);
}

#[test]
fn check_replay_rejects_when_caller_capacity_reached() {
    ReplayReceiptOps::reset_for_tests();

    let caller = p(9);
    let response_bytes = encode_one(Response::Cycles(CyclesResponse {
        cycles_transferred: 1,
    }))
    .expect("encode");

    for i in 0..MAX_ROOT_REPLAY_ENTRIES_PER_CALLER {
        let mut request_id = [0u8; 32];
        request_id[..8].copy_from_slice(&(i as u64).to_be_bytes());
        request_id[31] = 200;

        seed_root_replay_receipt(
            "root.request_cycles.v1",
            caller,
            request_id,
            [0u8; 32],
            5_000,
            Some(response_bytes.clone()),
        );
    }

    let ctx = RootContext {
        caller,
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(2),
        now: 1_000,
    };
    let capability = RootCapability::RequestCycles(CyclesRequest {
        cycles: 77,
        metadata: Some(meta(7, secs_to_ns(60))),
    });
    RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect_err("reservation must fail when caller is at capacity");
}
