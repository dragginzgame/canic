use super::*;
use crate::{
    cdk::types::{Principal, TC},
    config::{Config, ConfigModel},
    dto::{
        auth::{InternalInvocationProofRequest, RoleAttestationRequest},
        rpc::{
            CreateCanisterParent, CreateCanisterRequest, CyclesRequest, CyclesResponse,
            RecycleCanisterRequest, RootRequestMetadata, UpgradeCanisterRequest,
            UpgradeCanisterResponse,
        },
    },
    ids::CanisterRole,
    ops::{
        cost_guard::CostGuardOps,
        replay::{
            guard::secs_to_ns,
            model::{
                CommandKind, EcdsaPurpose, ExternalEffectDescriptor, OperationId,
                REPLAY_PAYLOAD_HASH_SCHEMA_VERSION, REPLAY_RECEIPT_SCHEMA_VERSION, RecoveryReason,
                ReplayActor, ReplayReceiptStatus,
            },
            receipt::{mark_external_effect_in_flight, mark_recovery_required},
        },
        runtime::{
            cycles_funding::CyclesFundingLedgerOps,
            metrics::cycles_funding::{
                CyclesFundingDeniedReason, CyclesFundingMetricKey, CyclesFundingMetrics,
            },
        },
        storage::{
            index::app::AppIndexOps, registry::subnet::SubnetRegistryOps, replay::ReplayReceiptOps,
            state::app::AppStateOps,
        },
    },
    replay_policy::CostClass,
    storage::stable::env::{Env, EnvRecord},
    storage::stable::index::app::AppIndexRecord,
    storage::stable::replay::ReplayReceiptRecord,
    storage::stable::state::app::{AppMode, AppStateRecord},
};
use candid::encode_one;
use std::collections::HashMap;

fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

fn meta(id: u8, ttl_seconds: u64) -> RootRequestMetadata {
    RootRequestMetadata {
        request_id: [id; 32],
        ttl_seconds,
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
            effect: None,
        },
    );
}

///
/// EnvRestore
///

struct EnvRestore(EnvRecord);

impl Drop for EnvRestore {
    fn drop(&mut self) {
        Env::import(self.0.clone());
    }
}

fn configure_root_env(root_pid: Principal) -> EnvRestore {
    let original = Env::export();
    Env::import(EnvRecord {
        root_pid: Some(root_pid),
        subnet_role: Some(crate::ids::SubnetRole::PRIME),
        ..EnvRecord::default()
    });
    EnvRestore(original)
}

///
/// AppIndexRestore
///

struct AppIndexRestore(AppIndexRecord);

impl Drop for AppIndexRestore {
    fn drop(&mut self) {
        AppIndexOps::import_allow_incomplete(self.0.clone()).expect("restore app index");
    }
}

fn configure_app_index(entries: Vec<(CanisterRole, Principal)>) -> AppIndexRestore {
    let original = AppIndexOps::data();
    AppIndexOps::import_allow_incomplete(AppIndexRecord { entries }).expect("import app index");
    AppIndexRestore(original)
}

///
/// ConfigReset
///

struct ConfigReset;

impl Drop for ConfigReset {
    fn drop(&mut self) {
        Config::reset_for_tests();
    }
}

fn configure_role_epoch(role: &CanisterRole, epoch: u64) -> ConfigReset {
    Config::reset_for_tests();
    let mut cfg = ConfigModel::test_default();
    cfg.auth
        .role_attestation
        .min_accepted_epoch_by_role
        .insert(role.as_str().to_string(), epoch);
    Config::init_from_model_for_tests(cfg).expect("install role epoch test config");
    ConfigReset
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
fn map_request_maps_provision() {
    let req = Request::CreateCanister(CreateCanisterRequest {
        canister_role: CanisterRole::new("app"),
        parent: CreateCanisterParent::Root,
        extra_arg: None,
        metadata: None,
    });

    let mapped = RootResponseWorkflow::map_request(req);
    assert_eq!(mapped.capability_name(), "Provision");
}

#[test]
fn map_request_maps_upgrade() {
    let req = Request::UpgradeCanister(UpgradeCanisterRequest {
        canister_pid: p(1),
        metadata: None,
    });

    let mapped = RootResponseWorkflow::map_request(req);
    assert_eq!(mapped.capability_name(), "Upgrade");
}

#[test]
fn map_request_maps_recycle_canister() {
    let req = Request::RecycleCanister(RecycleCanisterRequest {
        canister_pid: p(4),
        metadata: None,
    });

    let mapped = RootResponseWorkflow::map_request(req);
    assert_eq!(mapped.capability_name(), "RecycleCanister");
}

#[test]
fn map_request_maps_cycles() {
    let req = Request::Cycles(CyclesRequest {
        cycles: 42,
        metadata: None,
    });

    let mapped = RootResponseWorkflow::map_request(req);
    assert_eq!(mapped.capability_name(), "RequestCycles");
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
    assert!(
        err.to_string().contains("is not a child of caller"),
        "expected non-child denial, got: {err}"
    );
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
fn map_request_maps_issue_role_attestation() {
    let req = Request::IssueRoleAttestation(RoleAttestationRequest {
        subject: p(2),
        role: CanisterRole::new("test"),
        subnet_id: Some(p(7)),
        audience: p(8),
        ttl_secs: 120,
        epoch: 1,
        metadata: None,
    });

    let mapped = RootResponseWorkflow::map_request(req);
    assert_eq!(mapped.capability_name(), "IssueRoleAttestation");
}

#[test]
fn map_request_maps_issue_internal_invocation_proof() {
    let req = Request::IssueInternalInvocationProof(InternalInvocationProofRequest {
        subject: p(2),
        role: CanisterRole::new("project_hub"),
        subnet_id: Some(p(7)),
        audience: p(8),
        audience_method: "system_add_project_to_user".to_string(),
        ttl_secs: 120,
        metadata: None,
    });

    let mapped = RootResponseWorkflow::map_request(req);
    assert_eq!(mapped.capability_name(), "IssueInternalInvocationProof");
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
    let capability = RootCapability::Provision(CreateCanisterRequest {
        canister_role: CanisterRole::new("app"),
        parent: CreateCanisterParent::Root,
        extra_arg: None,
        metadata: None,
    });

    let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
    assert!(
        err.to_string().contains("root"),
        "expected root-env denial, got: {err}"
    );
}

#[test]
fn authorize_allows_provision_in_root_context() {
    let ctx = RootContext {
        caller: p(1),
        self_pid: p(9),
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::Provision(CreateCanisterRequest {
        canister_role: CanisterRole::new("app"),
        parent: CreateCanisterParent::Root,
        extra_arg: None,
        metadata: None,
    });

    RootResponseWorkflow::authorize(&ctx, &capability).expect("must authorize");
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
    let guard_request = execute::root_provision_cost_guard_request(&ctx, 5 * TC, 100 * TC);

    assert_eq!(guard_request.cost_class, CostClass::ManagementDeployment);
    assert_eq!(guard_request.command_kind.as_str(), "root.provision.v1");
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
        metadata: Some(meta(31, 60)),
    };
    let capability = RootCapability::Provision(req.clone());
    let pending = match RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect("first replay should reserve")
    {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };

    execute::mark_root_provision_external_effect(&pending, &ctx, &req, p(42));

    let receipt = ReplayReceiptOps::get(pending.receipt_token.key())
        .expect("receipt")
        .into_receipt()
        .expect("receipt decodes");
    assert_eq!(receipt.status, ReplayReceiptStatus::ExternalEffectInFlight);
    assert_eq!(
        receipt.effect,
        Some(ExternalEffectDescriptor::ManagementCreateCanister {
            command_kind: CommandKind::new("root.provision.v1").expect("command kind"),
        })
    );

    RootResponseWorkflow::abort_replay(pending);
}

#[test]
fn preflight_authorize_then_replay_denies_before_replay_validation() {
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

    let err = RootResponseWorkflow::preflight(
        &ctx,
        &capability,
        AuthorizationPipelineOrder::AuthorizeThenReplay,
    )
    .expect_err("authorize-then-replay should deny before replay validation");
    assert!(
        !err.to_string().contains("missing replay metadata"),
        "expected policy denial before replay validation, got: {err}"
    );
}

#[test]
fn preflight_replay_then_authorize_validates_replay_before_policy() {
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

    let err = RootResponseWorkflow::preflight(
        &ctx,
        &capability,
        AuthorizationPipelineOrder::ReplayThenAuthorize,
    )
    .expect_err("replay-then-authorize should validate replay first");
    assert!(
        err.to_string().contains("missing replay metadata"),
        "expected replay metadata error first, got: {err}"
    );
}

#[test]
fn preflight_replay_then_authorize_aborts_reserved_replay_on_policy_denial() {
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
        metadata: Some(meta(7, 60)),
    });

    let err = RootResponseWorkflow::preflight(
        &ctx,
        &capability,
        AuthorizationPipelineOrder::ReplayThenAuthorize,
    )
    .expect_err("policy denial should fail preflight");
    assert!(
        err.to_string().contains("not found") || err.to_string().contains("not a child"),
        "expected caller topology denial, got: {err}"
    );

    let replay = RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect("denied preflight must not leave replay slot in-flight");
    let pending = match replay {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => {
            panic!("policy-denied replay should not return cached response")
        }
    };
    RootResponseWorkflow::abort_replay(pending);
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
        metadata: Some(meta(22, 60)),
    });

    let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
    assert!(
        err.to_string().contains("not found"),
        "expected child-not-found denial, got: {err}"
    );

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

    AppStateOps::import(AppStateRecord {
        mode: AppMode::Enabled,
        cycles_funding_enabled: false,
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
        metadata: Some(meta(23, 60)),
    });

    let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
    assert!(
        err.to_string().contains("cycles funding disabled"),
        "expected kill-switch denial, got: {err}"
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

    AppStateOps::import(AppStateRecord {
        mode: AppMode::Enabled,
        cycles_funding_enabled: true,
    });
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
    assert!(err.to_string().contains("quota exceeded"));
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
        metadata: Some(meta(29, 60)),
    });
    let pending = match RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect("first replay should reserve")
    {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };

    nonroot_cycles::mark_request_cycles_external_effect(&pending, &ctx, 77);

    let receipt = ReplayReceiptOps::get(pending.receipt_token.key())
        .expect("receipt")
        .into_receipt()
        .expect("receipt decodes");
    assert_eq!(receipt.status, ReplayReceiptStatus::ExternalEffectInFlight);
    assert_eq!(
        receipt.effect,
        Some(ExternalEffectDescriptor::ManagementCall {
            canister: ctx.caller,
            method: "deposit_cycles".to_string(),
        })
    );

    RootResponseWorkflow::abort_replay(pending);
}

#[test]
fn authorize_rejects_role_attestation_when_subject_mismatches_caller() {
    let ctx = RootContext {
        caller: p(1),
        self_pid: p(9),
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::IssueRoleAttestation(RoleAttestationRequest {
        subject: p(3),
        role: CanisterRole::new("test"),
        subnet_id: None,
        audience: p(8),
        ttl_secs: 60,
        epoch: 0,
        metadata: None,
    });

    let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
    assert!(
        err.to_string().contains("must match caller"),
        "expected subject/caller mismatch error, got: {err}"
    );
}

#[test]
fn authorize_rejects_role_attestation_when_subject_not_registered() {
    let subject = p(41);
    let ctx = RootContext {
        caller: subject,
        self_pid: p(9),
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::IssueRoleAttestation(RoleAttestationRequest {
        subject,
        role: CanisterRole::new("test"),
        subnet_id: None,
        audience: p(8),
        ttl_secs: 60,
        epoch: 0,
        metadata: None,
    });

    let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
    assert!(
        err.to_string().contains("not registered"),
        "expected subject not registered error, got: {err}"
    );
}

#[test]
fn authorize_rejects_role_attestation_when_requested_role_differs_from_registry() {
    let subject = p(42);
    SubnetRegistryOps::register_root(subject, 1);

    let ctx = RootContext {
        caller: subject,
        self_pid: p(9),
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::IssueRoleAttestation(RoleAttestationRequest {
        subject,
        role: CanisterRole::new("test"),
        subnet_id: None,
        audience: p(8),
        ttl_secs: 60,
        epoch: 0,
        metadata: None,
    });

    let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
    assert!(
        err.to_string().contains("role mismatch"),
        "expected role mismatch error, got: {err}"
    );
}

#[test]
fn authorize_rejects_role_attestation_when_subnet_mismatch() {
    let subject = p(44);
    SubnetRegistryOps::register_root(subject, 1);

    let ctx = RootContext {
        caller: subject,
        self_pid: p(9),
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::IssueRoleAttestation(RoleAttestationRequest {
        subject,
        role: CanisterRole::ROOT,
        subnet_id: Some(p(7)),
        audience: p(8),
        ttl_secs: 60,
        epoch: 0,
        metadata: None,
    });

    let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
    assert!(
        err.to_string().contains("subnet mismatch"),
        "expected subnet mismatch error, got: {err}"
    );
}

#[test]
fn authorize_rejects_role_attestation_invalid_ttl_before_execution() {
    let subject = p(45);
    SubnetRegistryOps::register_root(subject, 1);

    let ctx = RootContext {
        caller: subject,
        self_pid: p(9),
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::IssueRoleAttestation(RoleAttestationRequest {
        subject,
        role: CanisterRole::ROOT,
        subnet_id: None,
        audience: p(8),
        ttl_secs: 0,
        epoch: 0,
        metadata: None,
    });

    let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
    assert!(
        err.to_string().contains("ttl_secs"),
        "expected ttl denial, got: {err}"
    );
}

#[test]
fn authorize_accepts_internal_invocation_proof_from_app_index_role() {
    let subject = p(51);
    let audience = p(52);
    let role = CanisterRole::new("project_hub");
    let _app_index = configure_app_index(vec![
        (role.clone(), subject),
        (CanisterRole::new("user_hub"), audience),
    ]);

    let ctx = RootContext {
        caller: subject,
        self_pid: p(9),
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::IssueInternalInvocationProof(InternalInvocationProofRequest {
        subject,
        role,
        subnet_id: Some(p(2)),
        audience,
        audience_method: "system_add_project_to_user".to_string(),
        ttl_secs: 60,
        metadata: None,
    });

    RootResponseWorkflow::authorize(&ctx, &capability)
        .expect("AppIndex role should authorize internal proof issuance");
}

#[test]
fn authorize_rejects_internal_invocation_proof_with_unknown_audience() {
    let subject = p(53);
    let role = CanisterRole::new("project_hub");
    let _app_index = configure_app_index(vec![(role.clone(), subject)]);

    let ctx = RootContext {
        caller: subject,
        self_pid: p(9),
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::IssueInternalInvocationProof(InternalInvocationProofRequest {
        subject,
        role,
        subnet_id: None,
        audience: p(54),
        audience_method: "system_add_project_to_user".to_string(),
        ttl_secs: 60,
        metadata: None,
    });

    let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
    assert!(
        err.to_string().contains("audience"),
        "expected audience denial, got: {err}"
    );
}

#[test]
fn authorize_rejects_internal_invocation_proof_invalid_ttl_before_execution() {
    let subject = p(55);
    let audience = p(56);
    let role = CanisterRole::new("project_hub");
    let _app_index = configure_app_index(vec![
        (role.clone(), subject),
        (CanisterRole::new("user_hub"), audience),
    ]);

    let ctx = RootContext {
        caller: subject,
        self_pid: p(9),
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::IssueInternalInvocationProof(InternalInvocationProofRequest {
        subject,
        role,
        subnet_id: None,
        audience,
        audience_method: "system_add_project_to_user".to_string(),
        ttl_secs: 0,
        metadata: None,
    });

    let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
    assert!(
        err.to_string().contains("ttl_secs"),
        "expected ttl denial, got: {err}"
    );
}

#[test]
fn build_role_attestation_uses_root_generated_time_window_and_epoch() {
    let ctx = RootContext {
        caller: p(1),
        self_pid: p(9),
        is_root_env: true,
        subnet_id: p(2),
        now: 1_000,
    };
    let role = CanisterRole::new("test");
    let _config = configure_role_epoch(&role, 7);
    let req = RoleAttestationRequest {
        subject: p(1),
        role,
        subnet_id: Some(p(7)),
        audience: p(8),
        ttl_secs: 120,
        epoch: 5,
        metadata: None,
    };

    let payload = RootResponseWorkflow::build_role_attestation(&ctx, &req).expect("payload");
    assert_eq!(payload.subject, req.subject);
    assert_eq!(payload.role, req.role);
    assert_eq!(payload.subnet_id, req.subnet_id);
    assert_eq!(payload.audience, req.audience);
    assert_eq!(payload.issued_at, 1_000);
    assert_eq!(payload.expires_at, 1_120);
    assert_eq!(payload.epoch, 7);
}

#[test]
fn build_internal_invocation_proof_uses_root_generated_epoch_and_method() {
    let ctx = RootContext {
        caller: p(1),
        self_pid: p(9),
        is_root_env: true,
        subnet_id: p(2),
        now: 1_000,
    };
    let req = InternalInvocationProofRequest {
        subject: p(1),
        role: CanisterRole::new("project_hub"),
        subnet_id: Some(p(7)),
        audience: p(8),
        audience_method: "system_add_project_to_user".to_string(),
        ttl_secs: 120,
        metadata: None,
    };

    let payload =
        RootResponseWorkflow::build_internal_invocation_proof(&ctx, &req).expect("payload");
    assert_eq!(payload.subject, req.subject);
    assert_eq!(payload.role, req.role);
    assert_eq!(payload.subnet_id, req.subnet_id);
    assert_eq!(payload.audience, req.audience);
    assert_eq!(payload.audience_method, req.audience_method);
    assert_eq!(payload.issued_at, 1_000);
    assert_eq!(payload.expires_at, 1_120);
    assert_eq!(payload.epoch, 0);
}

#[test]
fn build_role_attestation_rejects_invalid_ttl() {
    let ctx = RootContext {
        caller: p(1),
        self_pid: p(9),
        is_root_env: true,
        subnet_id: p(2),
        now: 1_000,
    };
    let mut req = RoleAttestationRequest {
        subject: p(1),
        role: CanisterRole::new("test"),
        subnet_id: Some(p(7)),
        audience: p(8),
        ttl_secs: 0,
        epoch: 5,
        metadata: None,
    };

    let zero_ttl =
        RootResponseWorkflow::build_role_attestation(&ctx, &req).expect_err("must reject");
    assert!(
        zero_ttl.to_string().contains("ttl_secs"),
        "expected ttl error for zero ttl, got: {zero_ttl}"
    );

    req.ttl_secs = DEFAULT_MAX_ROLE_ATTESTATION_TTL_SECONDS + 1;
    let too_large =
        RootResponseWorkflow::build_role_attestation(&ctx, &req).expect_err("must reject");
    assert!(
        too_large.to_string().contains("ttl_secs"),
        "expected ttl error for too-large ttl, got: {too_large}"
    );
}

#[test]
fn build_internal_invocation_proof_rejects_invalid_ttl() {
    let ctx = RootContext {
        caller: p(1),
        self_pid: p(9),
        is_root_env: true,
        subnet_id: p(2),
        now: 1_000,
    };
    let mut req = InternalInvocationProofRequest {
        subject: p(1),
        role: CanisterRole::new("project_hub"),
        subnet_id: Some(p(7)),
        audience: p(8),
        audience_method: "system_add_project_to_user".to_string(),
        ttl_secs: 0,
        metadata: None,
    };

    let zero_ttl =
        RootResponseWorkflow::build_internal_invocation_proof(&ctx, &req).expect_err("must reject");
    assert!(
        zero_ttl.to_string().contains("ttl_secs"),
        "expected ttl error for zero ttl, got: {zero_ttl}"
    );

    req.ttl_secs = DEFAULT_MAX_ROLE_ATTESTATION_TTL_SECONDS + 1;
    let too_large =
        RootResponseWorkflow::build_internal_invocation_proof(&ctx, &req).expect_err("must reject");
    assert!(
        too_large.to_string().contains("ttl_secs"),
        "expected ttl error for too-large ttl, got: {too_large}"
    );
}

#[test]
fn build_internal_invocation_proof_rejects_blank_method() {
    let ctx = RootContext {
        caller: p(1),
        self_pid: p(9),
        is_root_env: true,
        subnet_id: p(2),
        now: 1_000,
    };
    let req = InternalInvocationProofRequest {
        subject: p(1),
        role: CanisterRole::new("project_hub"),
        subnet_id: Some(p(7)),
        audience: p(8),
        audience_method: "   ".to_string(),
        ttl_secs: 120,
        metadata: None,
    };

    let err =
        RootResponseWorkflow::build_internal_invocation_proof(&ctx, &req).expect_err("must reject");
    assert!(
        err.to_string().contains("audience_method"),
        "expected method error, got: {err}"
    );
}

#[test]
fn build_root_auth_material_rejects_expiry_overflow() {
    let ctx = RootContext {
        caller: p(1),
        self_pid: p(9),
        is_root_env: true,
        subnet_id: p(2),
        now: u64::MAX,
    };
    let role_req = RoleAttestationRequest {
        subject: p(1),
        role: CanisterRole::new("test"),
        subnet_id: Some(p(7)),
        audience: p(8),
        ttl_secs: 1,
        epoch: 5,
        metadata: None,
    };
    let proof_req = InternalInvocationProofRequest {
        subject: p(1),
        role: CanisterRole::new("project_hub"),
        subnet_id: Some(p(7)),
        audience: p(8),
        audience_method: "system_add_project_to_user".to_string(),
        ttl_secs: 1,
        metadata: None,
    };

    let role_err =
        RootResponseWorkflow::build_role_attestation(&ctx, &role_req).expect_err("must reject");
    let proof_err = RootResponseWorkflow::build_internal_invocation_proof(&ctx, &proof_req)
        .expect_err("must reject");

    assert!(
        role_err.to_string().contains("ttl_secs"),
        "expected ttl error for overflow, got: {role_err}"
    );
    assert!(
        proof_err.to_string().contains("ttl_secs"),
        "expected ttl error for overflow, got: {proof_err}"
    );
}

#[test]
fn payload_hash_ignores_metadata() {
    let hash_a = RootCapability::RequestCycles(CyclesRequest {
        cycles: 42,
        metadata: Some(meta(1, 60)),
    })
    .payload_hash();
    let hash_b = RootCapability::RequestCycles(CyclesRequest {
        cycles: 42,
        metadata: Some(meta(9, 120)),
    })
    .payload_hash();

    assert_eq!(hash_a, hash_b, "metadata must not affect payload hash");
}

#[test]
fn role_attestation_payload_hash_ignores_request_epoch() {
    let request = |epoch| {
        RootCapability::IssueRoleAttestation(RoleAttestationRequest {
            subject: p(1),
            role: CanisterRole::new("project_hub"),
            subnet_id: Some(p(2)),
            audience: p(3),
            ttl_secs: 60,
            epoch,
            metadata: Some(meta(1, 60)),
        })
    };

    assert_eq!(
        request(0).payload_hash(),
        request(7).payload_hash(),
        "RoleAttestationRequest.epoch is caller-supplied legacy input and must not affect replay identity"
    );
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
    assert!(
        err.to_string().contains("invalid replay ttl"),
        "expected ttl validation error, got: {err}"
    );

    let too_large = RootCapability::RequestCycles(CyclesRequest {
        cycles: 77,
        metadata: Some(meta(7, MAX_ROOT_TTL_SECONDS + 1)),
    });
    let err = RootResponseWorkflow::check_replay(&ctx, &too_large).expect_err("must reject");
    assert!(
        err.to_string().contains("invalid replay ttl"),
        "expected ttl validation error, got: {err}"
    );
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
        metadata: Some(meta(11, 60)),
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

    let err = RootResponseWorkflow::check_replay(&ctx, &capability).expect_err("must expire");
    assert!(
        err.to_string().contains("replay request expired"),
        "expected replay expiration error, got: {err}"
    );
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
        metadata: Some(meta(7, 60)),
    });

    let pending = match RootResponseWorkflow::check_replay(&ctx, &capability).expect("first replay")
    {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };

    let response = Response::Cycles(CyclesResponse {
        cycles_transferred: 77,
    });
    RootResponseWorkflow::commit_replay(&pending, &response).expect("commit");

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
    let capability = RootCapability::IssueRoleAttestation(RoleAttestationRequest {
        subject: p(5),
        role: CanisterRole::new("app"),
        subnet_id: None,
        audience: p(9),
        ttl_secs: 60,
        epoch: 0,
        metadata: Some(meta(17, 60)),
    });

    let pending = match RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect("first replay should reserve")
    {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };
    let key = pending.receipt_token.key();
    let effect = ExternalEffectDescriptor::ThresholdEcdsaSign {
        key_id_hash: [1; 32],
        purpose: EcdsaPurpose::RoleAttestation,
        message_hash: [2; 32],
    };

    mark_external_effect_in_flight(&pending.receipt_token, effect.clone(), secs_to_ns(1_001));
    mark_recovery_required(
        &pending.receipt_token,
        RecoveryReason::ExternalEffectStatusUnknown,
        secs_to_ns(1_002),
    );
    RootResponseWorkflow::abort_replay(pending);

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

    let err = RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect_err("recovery-required duplicate must not run fresh");
    assert!(
        err.to_string().contains("duplicate replay request"),
        "expected duplicate replay error, got: {err}"
    );
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
        metadata: Some(meta(8, 60)),
    });
    let conflict = RootCapability::RequestCycles(CyclesRequest {
        cycles: 11,
        metadata: Some(meta(8, 60)),
    });

    let pending = match RootResponseWorkflow::check_replay(&ctx, &base).expect("first replay") {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };
    RootResponseWorkflow::commit_replay(
        &pending,
        &Response::Cycles(CyclesResponse {
            cycles_transferred: 10,
        }),
    )
    .expect("commit");

    let err = RootResponseWorkflow::check_replay(&ctx, &conflict).expect_err("must conflict");
    assert!(
        err.to_string().contains("replay conflict"),
        "expected replay conflict error, got: {err}"
    );
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
    let upgrade = RootCapability::Upgrade(UpgradeCanisterRequest {
        canister_pid: p(9),
        metadata: Some(meta(8, 60)),
    });
    let cycles = RootCapability::RequestCycles(CyclesRequest {
        cycles: 11,
        metadata: Some(meta(8, 60)),
    });

    let pending = match RootResponseWorkflow::check_replay(&ctx, &upgrade).expect("first replay") {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };
    RootResponseWorkflow::commit_replay(
        &pending,
        &Response::UpgradeCanister(UpgradeCanisterResponse {}),
    )
    .expect("commit");

    let err = RootResponseWorkflow::check_replay(&ctx, &cycles).expect_err("must conflict");
    assert!(
        err.to_string().contains("replay conflict"),
        "expected replay conflict error, got: {err}"
    );
}

#[test]
fn preflight_authorize_then_replay_reports_existing_cross_variant_conflict_before_policy() {
    ReplayReceiptOps::reset_for_tests();

    let ctx = RootContext {
        caller: p(3),
        self_pid: p(42),
        is_root_env: true,
        subnet_id: p(4),
        now: 2_000,
    };
    let upgrade = RootCapability::Upgrade(UpgradeCanisterRequest {
        canister_pid: p(9),
        metadata: Some(meta(8, 60)),
    });
    let cycles = RootCapability::RequestCycles(CyclesRequest {
        cycles: 11,
        metadata: Some(meta(8, 60)),
    });

    let pending = match RootResponseWorkflow::check_replay(&ctx, &upgrade).expect("first replay") {
        replay::ReplayPreflight::Fresh(pending) => pending,
        replay::ReplayPreflight::Cached(_) => panic!("first replay must be fresh"),
    };
    RootResponseWorkflow::commit_replay(
        &pending,
        &Response::UpgradeCanister(UpgradeCanisterResponse {}),
    )
    .expect("commit");

    let err = RootResponseWorkflow::preflight(
        &ctx,
        &cycles,
        AuthorizationPipelineOrder::AuthorizeThenReplay,
    )
    .expect_err("existing replay conflict must win before policy");
    assert!(
        err.to_string().contains("replay conflict"),
        "expected replay conflict error, got: {err}"
    );
}

#[test]
fn replay_purge_respects_limit_and_keeps_unexpired_entries() {
    ReplayReceiptOps::reset_for_tests();

    let ok = encode_one(Response::UpgradeCanister(UpgradeCanisterResponse {})).expect("encode");

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
        metadata: Some(meta(7, 60)),
    });
    let err = RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect_err("reservation must fail when store is at capacity");

    assert!(
        err.to_string().contains("replay store capacity reached"),
        "expected capacity error, got: {err}"
    );
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
        metadata: Some(meta(7, 60)),
    });
    let err = RootResponseWorkflow::check_replay(&ctx, &capability)
        .expect_err("reservation must fail when caller is at capacity");

    assert!(
        err.to_string()
            .contains("replay store caller capacity reached"),
        "expected caller capacity error, got: {err}"
    );
}
