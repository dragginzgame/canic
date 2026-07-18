//! Focused tests for delegated-token preparation retention.

use super::*;
use crate::{
    dto::auth::{
        DelegatedRoleGrant, DelegationAudience, DelegationCert, DelegationProof,
        IssuerProofAlgorithm, IssuerProofBinding,
    },
    ids::CanisterRole,
    ops::auth::{
        delegated::prepare::{PrepareDelegatedTokenInput, prepare_delegated_token},
        issuer_canister_sig::{IssuerPayloadKind, issuer_canister_sig_seed_hash},
        test_fixtures::chain_key_root_proof,
    },
};

fn p(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn prepared_token(prepared_by: Principal, operation: u8) -> PreparedDelegatedToken {
    let grant = DelegatedRoleGrant {
        target: CanisterRole::new("project_instance"),
        scopes: vec!["read".to_string()],
    };
    let proof = DelegationProof {
        cert: DelegationCert {
            root_pid: p(1),
            issuer_pid: p(2),
            issuer_proof_alg: IssuerProofAlgorithm::IcCanisterSignatureV1,
            issuer_proof_binding_hash: [3; 32],
            issuer_proof_binding: IssuerProofBinding::IcCanisterSignatureV1 {
                seed_hash: issuer_canister_sig_seed_hash(IssuerPayloadKind::DelegatedTokenClaims),
            },
            issued_at_ns: 10,
            not_before_ns: 10,
            expires_at_ns: 1_000,
            max_token_ttl_ns: 100,
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![grant.clone()],
        },
        root_proof: chain_key_root_proof(4),
    };

    prepare_delegated_token(PrepareDelegatedTokenInput {
        proof: &proof,
        operation_id: [operation; 32],
        prepared_by,
        subject: prepared_by,
        audience: DelegationAudience::Project("test".to_string()),
        grants: vec![grant],
        ttl_ns: 50,
        ext: None,
        now_ns: 100,
    })
    .expect("retention fixture should prepare")
}

fn reset() {
    RETAINED_DELEGATED_TOKENS.with_borrow_mut(BTreeMap::clear);
}

fn retain(prepared_by: Principal, operation: u8, retrieval_expires_at_ns: u64) {
    let prepared = prepared_token(prepared_by, operation);
    insert(
        RetainedDelegatedTokenKey::new(prepared.claims_hash, prepared_by),
        RetainedDelegatedToken {
            prepared,
            retrieval_expires_at_ns,
        },
    );
}

#[test]
fn caller_capacity_rejects_without_pruning_live_entries() {
    let _guard = crate::test::seams::lock();
    reset();
    let caller = p(9);
    for operation in 0..DELEGATED_TOKEN_REPLAY_RETENTION_LIMITS.max_active_per_actor {
        retain(
            caller,
            u8::try_from(operation).expect("operation fixture fits u8"),
            200,
        );
    }

    let error = prune_and_admit(caller, 100).expect_err("caller capacity must reject");

    assert!(error.is_public_resource_exhausted());
    RETAINED_DELEGATED_TOKENS.with_borrow(|retained| assert_eq!(retained.len(), 64));
    reset();
}

#[test]
fn global_capacity_rejects_distinct_callers() {
    let _guard = crate::test::seams::lock();
    reset();
    for index in 0..DELEGATED_TOKEN_REPLAY_RETENTION_LIMITS.max_active_per_command_kind {
        let caller = p(u8::try_from(index / 64 + 1).expect("caller fixture"));
        retain(
            caller,
            u8::try_from(index % 64).expect("operation fixture fits u8"),
            200,
        );
    }

    let error = prune_and_admit(p(20), 100).expect_err("global capacity must reject");

    assert!(error.is_public_resource_exhausted());
    RETAINED_DELEGATED_TOKENS.with_borrow(|retained| assert_eq!(retained.len(), 512));
    reset();
}

#[test]
fn expiry_boundary_prunes_before_capacity_admission() {
    let _guard = crate::test::seams::lock();
    reset();
    let caller = p(9);
    for operation in 0..DELEGATED_TOKEN_REPLAY_RETENTION_LIMITS.max_active_per_actor {
        retain(
            caller,
            u8::try_from(operation).expect("operation fixture fits u8"),
            100,
        );
    }

    prune_and_admit(caller, 100).expect("expiry boundary must release capacity");

    RETAINED_DELEGATED_TOKENS.with_borrow(|retained| assert!(retained.is_empty()));
    reset();
}

#[test]
fn retrieval_rejects_at_exact_expiry_boundary() {
    let _guard = crate::test::seams::lock();
    reset();
    let caller = p(9);
    let prepared = prepared_token(caller, 1);
    let key = RetainedDelegatedTokenKey::new(prepared.claims_hash, caller);
    insert(
        key.clone(),
        RetainedDelegatedToken {
            prepared,
            retrieval_expires_at_ns: 100,
        },
    );

    get(&key, 99).expect("token remains available before expiry");
    get(&key, 100).expect_err("token must reject at expiry boundary");
    reset();
}
