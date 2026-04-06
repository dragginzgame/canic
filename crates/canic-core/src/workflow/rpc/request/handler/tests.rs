use super::*;
use crate::{
    cdk::types::Principal,
    dto::{
        auth::{DelegationCert, DelegationRequest, RoleAttestationRequest},
        rpc::{
            CreateCanisterParent, CreateCanisterRequest, CyclesRequest, CyclesResponse,
            RootRequestMetadata, UpgradeCanisterRequest, UpgradeCanisterResponse,
        },
    },
    ids::CanisterRole,
    ops::{
        runtime::metrics::cycles_funding::{
            CyclesFundingDeniedReason, CyclesFundingMetricKey, CyclesFundingMetrics,
        },
        storage::{
            registry::subnet::SubnetRegistryOps, replay::RootReplayOps, state::app::AppStateOps,
        },
    },
    storage::stable::env::{Env, EnvRecord},
    storage::stable::replay::{ReplaySlotKey, RootReplayRecord},
    storage::stable::state::app::{AppMode, AppStateRecord},
};
use candid::encode_one;
use sha2::{Digest, Sha256};
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
        ..EnvRecord::default()
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

fn sample_delegation_cert(root_pid: Principal) -> DelegationCert {
    DelegationCert {
        root_pid,
        shard_pid: p(2),
        issued_at: 100,
        expires_at: 200,
        scopes: vec!["rpc:verify".to_string()],
        aud: vec![p(3)],
    }
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
fn map_request_maps_cycles() {
    let req = Request::Cycles(CyclesRequest {
        cycles: 42,
        metadata: None,
    });

    let mapped = RootResponseWorkflow::map_request(req);
    assert_eq!(mapped.capability_name(), "RequestCycles");
}

#[test]
fn map_request_maps_issue_delegation() {
    let req = Request::IssueDelegation(DelegationRequest {
        shard_pid: p(2),
        scopes: vec!["rpc:call".to_string()],
        aud: vec![p(3)],
        ttl_secs: 60,
        verifier_targets: vec![p(4)],
        include_root_verifier: true,
        shard_public_key_sec1: None,
        metadata: None,
    });

    let mapped = RootResponseWorkflow::map_request(req);
    assert_eq!(mapped.capability_name(), "IssueDelegation");
}

#[test]
fn map_request_maps_issue_role_attestation() {
    let req = Request::IssueRoleAttestation(RoleAttestationRequest {
        subject: p(2),
        role: CanisterRole::new("test"),
        subnet_id: Some(p(7)),
        audience: Some(p(8)),
        ttl_secs: 120,
        epoch: 1,
        metadata: None,
    });

    let mapped = RootResponseWorkflow::map_request(req);
    assert_eq!(mapped.capability_name(), "IssueRoleAttestation");
}

#[test]
fn validate_delegation_cert_policy_rejects_invalid_window() {
    let root_pid = p(1);
    let _restore = configure_root_env(root_pid);

    let mut cert = sample_delegation_cert(root_pid);
    cert.expires_at = cert.issued_at;

    let err =
        delegation::validate_delegation_cert_policy(&cert).expect_err("invalid window must fail");
    assert!(
        err.to_string()
            .contains("expires_at must be greater than issued_at")
    );
}

#[test]
fn validate_delegation_cert_policy_rejects_empty_audience() {
    let root_pid = p(1);
    let _restore = configure_root_env(root_pid);

    let mut cert = sample_delegation_cert(root_pid);
    cert.aud.clear();

    let err =
        delegation::validate_delegation_cert_policy(&cert).expect_err("empty audience must fail");
    assert!(
        err.to_string()
            .contains("delegation audience must not be empty")
    );
}

#[test]
fn validate_delegation_cert_policy_rejects_empty_scopes() {
    let root_pid = p(1);
    let _restore = configure_root_env(root_pid);

    let mut cert = sample_delegation_cert(root_pid);
    cert.scopes.clear();

    let err =
        delegation::validate_delegation_cert_policy(&cert).expect_err("empty scopes must fail");
    assert!(
        err.to_string()
            .contains("delegation scopes must not be empty")
    );
}

#[test]
fn validate_delegation_cert_policy_rejects_empty_scope_values() {
    let root_pid = p(1);
    let _restore = configure_root_env(root_pid);

    let mut cert = sample_delegation_cert(root_pid);
    cert.scopes = vec![String::new()];

    let err = delegation::validate_delegation_cert_policy(&cert)
        .expect_err("empty scope value must fail");
    assert!(err.to_string().contains("must not contain empty strings"));
}

#[test]
fn validate_delegation_cert_policy_rejects_root_pid_mismatch() {
    let root_pid = p(1);
    let _restore = configure_root_env(root_pid);

    let cert = sample_delegation_cert(p(9));
    let err = delegation::validate_delegation_cert_policy(&cert)
        .expect_err("root pid mismatch must fail");
    assert!(err.to_string().contains("delegation root pid mismatch"));
}

#[test]
fn validate_delegation_cert_policy_rejects_shard_equal_to_root() {
    let root_pid = p(1);
    let _restore = configure_root_env(root_pid);

    let mut cert = sample_delegation_cert(root_pid);
    cert.shard_pid = root_pid;

    let err = delegation::validate_delegation_cert_policy(&cert).expect_err("root shard must fail");
    assert!(
        err.to_string()
            .contains("delegation shard_pid must not equal root pid")
    );
}

#[test]
fn authorize_rejects_issue_delegation_when_verifier_target_is_root() {
    let root_pid = p(30);
    let _restore = configure_root_env(root_pid);
    SubnetRegistryOps::register_root(root_pid, 1);

    let caller = p(31);
    let ctx = RootContext {
        caller,
        self_pid: root_pid,
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::IssueDelegation(DelegationRequest {
        shard_pid: caller,
        scopes: vec!["rpc:verify".to_string()],
        aud: vec![p(33)],
        ttl_secs: 60,
        verifier_targets: vec![root_pid],
        include_root_verifier: true,
        shard_public_key_sec1: None,
        metadata: None,
    });

    let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
    assert!(
        err.to_string().contains("must not equal root pid"),
        "expected root-target denial, got: {err}"
    );
}

#[test]
fn authorize_rejects_issue_delegation_when_verifier_target_is_shard() {
    let root_pid = p(40);
    let _restore = configure_root_env(root_pid);
    SubnetRegistryOps::register_root(root_pid, 1);

    let caller = p(34);
    let ctx = RootContext {
        caller,
        self_pid: root_pid,
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::IssueDelegation(DelegationRequest {
        shard_pid: caller,
        scopes: vec!["rpc:verify".to_string()],
        aud: vec![p(35)],
        ttl_secs: 60,
        verifier_targets: vec![caller],
        include_root_verifier: true,
        shard_public_key_sec1: None,
        metadata: None,
    });

    let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
    assert!(
        err.to_string().contains("must not equal shard_pid"),
        "expected shard-target denial, got: {err}"
    );
}

#[test]
fn authorize_rejects_issue_delegation_when_verifier_target_is_unregistered() {
    let root_pid = p(50);
    let _restore = configure_root_env(root_pid);
    SubnetRegistryOps::register_root(root_pid, 1);

    let caller = p(36);
    let unregistered = p(37);
    let ctx = RootContext {
        caller,
        self_pid: root_pid,
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::IssueDelegation(DelegationRequest {
        shard_pid: caller,
        scopes: vec!["rpc:verify".to_string()],
        aud: vec![unregistered],
        ttl_secs: 60,
        verifier_targets: vec![unregistered],
        include_root_verifier: true,
        shard_public_key_sec1: None,
        metadata: None,
    });

    let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
    assert!(
        err.to_string().contains("must be registered"),
        "expected unregistered-target denial, got: {err}"
    );
}

#[test]
fn authorize_allows_issue_delegation_when_verifier_target_is_registered() {
    let root_pid = p(60);
    let _restore = configure_root_env(root_pid);
    SubnetRegistryOps::register_root(root_pid, 1);

    let verifier = p(38);
    SubnetRegistryOps::register_unchecked(
        verifier,
        &CanisterRole::new("project_hub"),
        root_pid,
        vec![],
        2,
    )
    .expect("register verifier canister");

    let caller = p(39);
    let ctx = RootContext {
        caller,
        self_pid: root_pid,
        is_root_env: true,
        subnet_id: p(2),
        now: 5,
    };
    let capability = RootCapability::IssueDelegation(DelegationRequest {
        shard_pid: caller,
        scopes: vec!["rpc:verify".to_string()],
        aud: vec![verifier],
        ttl_secs: 60,
        verifier_targets: vec![verifier],
        include_root_verifier: true,
        shard_public_key_sec1: None,
        metadata: None,
    });

    RootResponseWorkflow::authorize(&ctx, &capability).expect("registered verifier must authorize");
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
fn preflight_authorize_then_replay_denies_before_replay_validation() {
    RootReplayOps::reset_for_tests();

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
    RootReplayOps::reset_for_tests();

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
    RootReplayOps::reset_for_tests();

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
    funding::reset_for_tests();

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
    funding::reset_for_tests();

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
        audience: None,
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
        audience: Some(p(8)),
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
        audience: Some(p(8)),
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
fn authorize_rejects_role_attestation_when_audience_missing() {
    let subject = p(43);
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
        audience: None,
        ttl_secs: 60,
        epoch: 0,
        metadata: None,
    });

    let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
    assert!(
        err.to_string().contains("audience is required"),
        "expected audience-required error, got: {err}"
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
        audience: Some(p(8)),
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
fn build_role_attestation_uses_root_generated_time_window() {
    let ctx = RootContext {
        caller: p(1),
        self_pid: p(9),
        is_root_env: true,
        subnet_id: p(2),
        now: 1_000,
    };
    let req = RoleAttestationRequest {
        subject: p(1),
        role: CanisterRole::new("test"),
        subnet_id: Some(p(7)),
        audience: Some(p(8)),
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
    assert_eq!(payload.epoch, 5);
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
        audience: Some(p(8)),
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
fn payload_hash_includes_capability_variant_discriminant() {
    let capability_hash = RootCapability::RequestCycles(CyclesRequest {
        cycles: 42,
        metadata: None,
    })
    .payload_hash();

    let legacy_struct_hash = {
        let bytes = encode_one(&CyclesRequest {
            cycles: 42,
            metadata: None,
        })
        .expect("encode");
        hash_domain_separated(REPLAY_PAYLOAD_HASH_DOMAIN, &bytes)
    };

    assert_ne!(
        capability_hash, legacy_struct_hash,
        "capability payload hash must include variant discriminant"
    );
}

#[test]
fn replay_slot_key_binds_caller_target_and_request_id() {
    let request_id = [9u8; 32];
    let key = replay::replay_slot_key(p(1), p(2), request_id);

    assert_ne!(
        key,
        replay::replay_slot_key(p(3), p(2), request_id),
        "caller must affect replay key"
    );
    assert_ne!(
        key,
        replay::replay_slot_key(p(1), p(4), request_id),
        "target must affect replay key"
    );
    assert_ne!(
        key,
        replay::replay_slot_key(p(1), p(2), [8u8; 32]),
        "request_id must affect replay key"
    );
}

#[test]
fn check_replay_rejects_invalid_ttl() {
    RootReplayOps::reset_for_tests();

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
fn check_replay_rejects_expired_entry_when_purge_limit_exceeded() {
    RootReplayOps::reset_for_tests();

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
    let payload_hash = capability.payload_hash();
    let request_id = capability.metadata().expect("metadata").request_id;
    let target_key = replay::replay_slot_key(ctx.caller, ctx.self_pid, request_id);
    let response_bytes = encode_one(Response::Cycles(CyclesResponse {
        cycles_transferred: 500,
    }))
    .expect("encode");

    RootReplayOps::upsert(
        target_key,
        RootReplayRecord {
            payload_hash,
            issued_at: 9_900,
            expires_at: 9_999,
            response_bytes: response_bytes.clone(),
        },
    );

    // Force purge limit exhaustion before reaching target_key by seeding
    // 256 lexicographically smaller expired entries.
    let mut seeded = 0usize;
    let mut nonce = 0u64;
    while seeded < REPLAY_PURGE_SCAN_LIMIT {
        let mut hasher = Sha256::new();
        hasher.update(nonce.to_be_bytes());
        let candidate: [u8; 32] = hasher.finalize().into();
        nonce = nonce.saturating_add(1);
        assert!(
            nonce < 1_000_000,
            "failed to seed replay filler keys before nonce overflow"
        );

        if candidate >= target_key.0 {
            continue;
        }

        RootReplayOps::upsert(
            ReplaySlotKey(candidate),
            RootReplayRecord {
                payload_hash: [0u8; 32],
                issued_at: 9_000,
                expires_at: 9_100,
                response_bytes: response_bytes.clone(),
            },
        );
        seeded += 1;
    }

    let err = RootResponseWorkflow::check_replay(&ctx, &capability).expect_err("must expire");
    assert!(
        err.to_string().contains("replay request expired"),
        "expected replay expiration error, got: {err}"
    );
}

#[test]
fn check_replay_returns_cached_response_for_duplicate_same_payload() {
    RootReplayOps::reset_for_tests();

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
    RootResponseWorkflow::commit_replay(pending, &response).expect("commit");

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
fn check_replay_rejects_conflicting_payload_for_same_request_id() {
    RootReplayOps::reset_for_tests();

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
        pending,
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
fn replay_purge_respects_limit_and_keeps_unexpired_entries() {
    RootReplayOps::reset_for_tests();

    let ok = encode_one(Response::UpgradeCanister(UpgradeCanisterResponse {})).expect("encode");

    for i in 0..5u8 {
        RootReplayOps::upsert(
            ReplaySlotKey([i; 32]),
            RootReplayRecord {
                payload_hash: [i; 32],
                issued_at: 0,
                expires_at: 10,
                response_bytes: ok.clone(),
            },
        );
    }

    for i in 200..202u8 {
        RootReplayOps::upsert(
            ReplaySlotKey([i; 32]),
            RootReplayRecord {
                payload_hash: [i; 32],
                issued_at: 0,
                expires_at: 999,
                response_bytes: ok.clone(),
            },
        );
    }

    let purged = RootReplayOps::purge_expired(100, 3);
    assert_eq!(purged, 3, "purge must stop at the configured limit");
    assert_eq!(
        RootReplayOps::len(),
        4,
        "expected 4 entries after first purge"
    );

    let purged = RootReplayOps::purge_expired(100, 10);
    assert_eq!(purged, 2, "remaining expired entries must be purged");
    assert_eq!(
        RootReplayOps::len(),
        2,
        "only unexpired entries should remain"
    );
}

#[test]
fn check_replay_rejects_when_capacity_reached() {
    RootReplayOps::reset_for_tests();

    let response_bytes = encode_one(Response::Cycles(CyclesResponse {
        cycles_transferred: 1,
    }))
    .expect("encode");

    for i in 0..MAX_ROOT_REPLAY_ENTRIES {
        let mut key = [0u8; 32];
        key[..8].copy_from_slice(&(i as u64).to_be_bytes());

        RootReplayOps::upsert(
            ReplaySlotKey(key),
            RootReplayRecord {
                payload_hash: [0u8; 32],
                issued_at: 0,
                expires_at: 5_000,
                response_bytes: response_bytes.clone(),
            },
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
