use super::proof_cache::{
    cache_internal_invocation_proof, cached_internal_invocation_proof,
    clear_internal_invocation_proof_cache, internal_invocation_proof_refresh_margin_ns,
};
use super::*;
use crate::{
    config::schema::RoleAttestationConfig,
    dto::auth::{InternalInvocationProofPayloadV1, SignedInternalInvocationProofV1},
};
use candid::{decode_args, decode_one};
use std::collections::BTreeMap;

fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

fn proof() -> SignedInternalInvocationProofV1 {
    SignedInternalInvocationProofV1 {
        payload: InternalInvocationProofPayloadV1 {
            subject: p(1),
            role: CanisterRole::new("project_hub"),
            subnet_id: None,
            audience: p(2),
            audience_method: "system_add_project_to_user".to_string(),
            issued_at_ns: 10 * NS_PER_SEC,
            expires_at_ns: 20 * NS_PER_SEC,
            epoch: 3,
        },
        signature: vec![1, 2, 3],
        key_id: 1,
    }
}

fn request() -> InternalInvocationProofRequest {
    InternalInvocationProofRequest {
        subject: p(1),
        role: CanisterRole::new("project_hub"),
        subnet_id: Some(p(9)),
        audience: p(2),
        audience_method: "system_add_project_to_user".to_string(),
        ttl_ns: 120 * NS_PER_SEC,
        metadata: None,
    }
}

fn cfg(min_epoch: u64) -> RoleAttestationConfig {
    let mut min_accepted_epoch_by_role = BTreeMap::new();
    min_accepted_epoch_by_role.insert("project_hub".to_string(), min_epoch);
    RoleAttestationConfig {
        ecdsa_key_name: "key_1".to_string(),
        max_ttl_secs: 900,
        min_accepted_epoch_by_role,
    }
}

#[test]
fn canic_call_envelope_binds_target_method_and_original_args() {
    let args = encode_args((7_u64, "project")).expect("args encode");
    let envelope = build_internal_call_envelope(p(2), "system_add_project_to_user", proof(), args);

    assert_eq!(envelope.version, 1);
    assert_eq!(envelope.header.target_canister, p(2));
    assert_eq!(envelope.header.target_method, "system_add_project_to_user");
    assert_eq!(
        envelope.proof.payload.audience_method,
        "system_add_project_to_user"
    );

    let decoded: (u64, String) = decode_args(&envelope.args).expect("decode original args");
    assert_eq!(decoded, (7, "project".to_string()));
}

#[test]
fn canic_call_encodes_envelope_as_raw_ingress_bytes() {
    let args = encode_args((7_u64, "project")).expect("args encode");
    let envelope = build_internal_call_envelope(p(2), "system_add_project_to_user", proof(), args);
    let raw = encode_internal_call_envelope_raw(envelope.clone()).expect("envelope should encode");

    let decoded: CanicInternalCallEnvelopeV1 =
        decode_one(&raw).expect("raw ingress bytes decode as envelope");

    assert_eq!(decoded, envelope);
}

#[test]
fn canic_call_builder_records_role_and_raw_args() {
    let raw = vec![9_u8, 8, 7];
    let builder = CanicCall::unbounded_wait(p(3), "target")
        .with_caller_role(CanisterRole::new("project_hub"))
        .with_proof_ttl_secs(30)
        .with_cycles(10)
        .with_raw_args(raw.clone());

    assert_eq!(builder.wait, WaitMode::Unbounded);
    assert_eq!(builder.canister_id, p(3));
    assert_eq!(builder.method, "target");
    assert_eq!(builder.caller_role, Some(CanisterRole::new("project_hub")));
    assert_eq!(builder.ttl_secs, Some(30));
    assert_eq!(builder.cycles, 10);
    assert_eq!(builder.args.as_ref(), raw.as_slice());
}

#[test]
fn canic_call_rejects_empty_target_method_locally() {
    let err = validate_internal_call_target_method("   ")
        .expect_err("empty protected call method should fail locally");

    assert_eq!(err.code, ErrorCode::InvalidInput);
}

#[test]
fn canic_call_rejects_empty_caller_role_locally() {
    let err = validate_internal_call_caller_role(&CanisterRole::new("   "))
        .expect_err("empty protected call role should fail locally");

    assert_eq!(err.code, ErrorCode::InvalidInput);
}

#[test]
fn canic_call_rejects_zero_effective_proof_ttl_locally() {
    let zero_requested = effective_internal_call_proof_ttl_secs(0, 900)
        .expect_err("zero requested proof ttl should fail locally");
    assert_eq!(zero_requested.code, ErrorCode::InvalidInput);

    let zero_max = effective_internal_call_proof_ttl_secs(120, 0)
        .expect_err("zero configured max proof ttl should fail locally");
    assert_eq!(zero_max.code, ErrorCode::InvalidInput);
}

#[test]
fn canic_call_clamps_requested_proof_ttl_to_config_max() {
    assert_eq!(
        effective_internal_call_proof_ttl_secs(120, 900).expect("ttl"),
        120
    );
    assert_eq!(
        effective_internal_call_proof_ttl_secs(1200, 900).expect("ttl"),
        900
    );
}

#[test]
fn canic_call_ttl_secs_to_ns_rejects_overflow() {
    let err = secs_to_ns(u64::MAX).expect_err("ttl seconds overflow must reject");

    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(err.message.contains("overflows nanoseconds"));
}

#[test]
fn protected_internal_endpoint_descriptor_matches_roles() {
    let endpoint = ProtectedInternalEndpoint::new(
        "system_add_project_to_user",
        [
            CanisterRole::new("project_hub"),
            CanisterRole::new("admin_hub"),
        ],
    );

    assert_eq!(endpoint.method(), "system_add_project_to_user");
    assert_eq!(endpoint.accepted_roles_label(), "project_hub, admin_hub");
    assert!(endpoint.accepts_role(&CanisterRole::new("project_hub")));
    assert!(endpoint.accepts_role(&CanisterRole::new("admin_hub")));
    assert!(!endpoint.accepts_role(&CanisterRole::new("user_hub")));
    assert!(endpoint.single_role().is_none());
}

#[test]
fn protected_internal_endpoint_single_role_is_available_to_generated_clients() {
    let endpoint = ProtectedInternalEndpoint::new(
        "system_add_project_to_user",
        [CanisterRole::new("project_hub")],
    );

    assert_eq!(
        endpoint.single_role(),
        Some(&CanisterRole::new("project_hub"))
    );
    assert_eq!(
        endpoint.required_single_role().expect("single role"),
        CanisterRole::new("project_hub")
    );
}

#[test]
fn protected_internal_endpoint_requires_explicit_role_when_ambiguous() {
    let endpoint = ProtectedInternalEndpoint::new(
        "system_add_project_to_user",
        [
            CanisterRole::new("project_hub"),
            CanisterRole::new("admin_hub"),
        ],
    );

    let err = endpoint
        .required_single_role()
        .expect_err("multi-role endpoint should require explicit caller role");
    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(err.message.contains("project_hub, admin_hub"));
    assert!(err.message.contains("call_update(..., caller_role, args)"));
}

#[test]
fn protected_internal_endpoint_descriptor_rejects_missing_method() {
    let result =
        std::panic::catch_unwind(|| ProtectedInternalEndpoint::new("", [CanisterRole::ROOT]));

    assert!(result.is_err());
}

#[test]
fn protected_internal_endpoint_descriptor_rejects_blank_method() {
    let result =
        std::panic::catch_unwind(|| ProtectedInternalEndpoint::new("   ", [CanisterRole::ROOT]));

    assert!(result.is_err());
}

#[test]
fn protected_internal_endpoint_descriptor_rejects_missing_roles() {
    let result = std::panic::catch_unwind(|| {
        ProtectedInternalEndpoint::new("system_add_project_to_user", [])
    });

    assert!(result.is_err());
}

#[test]
fn protected_internal_endpoint_descriptor_rejects_empty_role() {
    let result = std::panic::catch_unwind(|| {
        ProtectedInternalEndpoint::new("system_add_project_to_user", [CanisterRole::new("")])
    });

    assert!(result.is_err());
}

#[test]
fn protected_internal_endpoint_descriptor_rejects_blank_role() {
    let result = std::panic::catch_unwind(|| {
        ProtectedInternalEndpoint::new("system_add_project_to_user", [CanisterRole::new("   ")])
    });

    assert!(result.is_err());
}

#[test]
fn protected_internal_endpoint_descriptor_rejects_duplicate_roles() {
    let result = std::panic::catch_unwind(|| {
        ProtectedInternalEndpoint::new(
            "system_add_project_to_user",
            [
                CanisterRole::new("project_hub"),
                CanisterRole::new("project_hub"),
            ],
        )
    });

    assert!(result.is_err());
}

#[test]
fn internal_client_options_are_chainable() {
    let client = CanicInternalClient::new(p(3))
        .with_bounded_wait()
        .with_cycles(10)
        .with_proof_ttl_secs(30);

    assert_eq!(client.canister_id, p(3));
    assert_eq!(client.options.wait, CanicInternalWaitMode::Bounded);
    assert_eq!(client.options.cycles, 10);
    assert_eq!(client.options.proof_ttl_secs, Some(30));
}

#[test]
fn internal_client_rejects_unaccepted_explicit_role_locally() {
    let client = CanicInternalClient::new(p(3));
    let endpoint = ProtectedInternalEndpoint::new(
        "system_add_project_to_user",
        [CanisterRole::new("project_hub")],
    );
    let result = futures::executor::block_on(client.call_update(
        &endpoint,
        CanisterRole::new("admin_hub"),
        (),
    ));

    match result {
        Err(err) => {
            assert_eq!(err.code, ErrorCode::InvalidInput);
            assert!(err.message.contains("accepted caller roles: [project_hub]"));
            assert!(
                err.message
                    .contains("call_update(..., accepted_role, args)")
            );
        }
        Ok(_) => panic!("unaccepted caller role should fail before transport"),
    }
}

#[test]
fn internal_invocation_proof_cache_reuses_exact_fresh_edge() {
    clear_internal_invocation_proof_cache();
    let request = request();
    let mut proof = proof();
    proof.payload.subnet_id = request.subnet_id;
    cache_internal_invocation_proof(&request, &cfg(0), p(7), 12 * NS_PER_SEC, proof.clone());

    let cached = cached_internal_invocation_proof(&request, &cfg(0), p(7), 12 * NS_PER_SEC)
        .expect("fresh matching proof should cache-hit");

    assert_eq!(cached, proof);
}

#[test]
fn internal_invocation_proof_cache_rejects_near_expiry_entry() {
    clear_internal_invocation_proof_cache();
    let request = request();
    let mut proof = proof();
    proof.payload.subnet_id = request.subnet_id;
    proof.payload.issued_at_ns = 10 * NS_PER_SEC;
    proof.payload.expires_at_ns = 20 * NS_PER_SEC;
    cache_internal_invocation_proof(&request, &cfg(0), p(7), 18 * NS_PER_SEC, proof);

    assert!(cached_internal_invocation_proof(&request, &cfg(0), p(7), 18 * NS_PER_SEC).is_none());
}

#[test]
fn internal_invocation_proof_cache_rejects_future_issued_at_entry() {
    clear_internal_invocation_proof_cache();
    let request = request();
    let mut proof = proof();
    proof.payload.subnet_id = request.subnet_id;
    proof.payload.issued_at_ns = 20 * NS_PER_SEC;
    proof.payload.expires_at_ns = 40 * NS_PER_SEC;
    cache_internal_invocation_proof(&request, &cfg(0), p(7), 12 * NS_PER_SEC, proof);

    assert!(cached_internal_invocation_proof(&request, &cfg(0), p(7), 12 * NS_PER_SEC).is_none());
}

#[test]
fn internal_invocation_proof_cache_rejects_invalid_time_window() {
    clear_internal_invocation_proof_cache();
    let request = request();
    let mut proof = proof();
    proof.payload.subnet_id = request.subnet_id;
    proof.payload.issued_at_ns = 20 * NS_PER_SEC;
    proof.payload.expires_at_ns = 20 * NS_PER_SEC;
    cache_internal_invocation_proof(&request, &cfg(0), p(7), 20 * NS_PER_SEC, proof);

    assert!(cached_internal_invocation_proof(&request, &cfg(0), p(7), 20 * NS_PER_SEC).is_none());
}

#[test]
fn internal_invocation_proof_cache_rejects_epoch_below_local_floor() {
    clear_internal_invocation_proof_cache();
    let request = request();
    let mut proof = proof();
    proof.payload.subnet_id = request.subnet_id;
    proof.payload.epoch = 3;
    cache_internal_invocation_proof(&request, &cfg(0), p(7), 12 * NS_PER_SEC, proof);

    assert!(cached_internal_invocation_proof(&request, &cfg(4), p(7), 12 * NS_PER_SEC).is_none());
}

#[test]
fn internal_invocation_proof_refresh_margin_has_ns_min_and_max() {
    let mut proof = proof();
    proof.payload.issued_at_ns = 10 * NS_PER_SEC;
    proof.payload.expires_at_ns = 11 * NS_PER_SEC;
    assert_eq!(
        internal_invocation_proof_refresh_margin_ns(&proof),
        NS_PER_SEC,
        "one-second proof windows should keep a one-second refresh floor"
    );

    proof.payload.expires_at_ns = 20 * NS_PER_SEC;
    assert_eq!(
        internal_invocation_proof_refresh_margin_ns(&proof),
        2 * NS_PER_SEC,
        "ten-second proof windows should use the one-fifth refresh margin"
    );

    proof.payload.expires_at_ns = 1_000 * NS_PER_SEC;
    assert_eq!(
        internal_invocation_proof_refresh_margin_ns(&proof),
        30 * NS_PER_SEC,
        "long proof windows should clamp to the configured thirty-second max"
    );
}

#[test]
fn internal_call_retry_classifier_is_limited_to_repairable_auth_material() {
    assert!(internal_call_error_is_retryable(&Error::new(
        ErrorCode::AuthKeyUnknown,
        "unknown key".to_string(),
    )));
    assert!(internal_call_error_is_retryable(&Error::new(
        ErrorCode::AuthMaterialStale,
        "stale epoch".to_string(),
    )));
    assert!(!internal_call_error_is_retryable(&Error::new(
        ErrorCode::AuthProofExpired,
        "expired".to_string(),
    )));
    assert!(!internal_call_error_is_retryable(&Error::unauthorized(
        "role mismatch"
    )));
}
