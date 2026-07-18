//! Module: workflow::runtime::auth::prepare
//!
//! Responsibility: prepare replay-protected auth proofs and delegated tokens.
//! Does not own: endpoint authorization, auth stable records, or crypto primitives.
//! Boundary: runtime auth workflow delegates proof creation to auth ops and replay ops.

mod admission;
mod replay;

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::{
        auth::{
            DelegatedTokenPrepareRequest, DelegatedTokenPrepareResponse,
            RoleAttestationPrepareResponse, RoleAttestationRequest, RootDelegationProofBatchProof,
        },
        error::{Error, ErrorCode},
    },
    model::replay::{RecoveryReason, ReplayActor},
    ops::{
        auth::{AuthOps, PrepareDelegatedTokenIssuerProofInput, PrepareRootRoleAttestationInput},
        ic::{
            IcOps,
            call::{CallOps, CallResult},
        },
        replay::{
            DELEGATED_TOKEN_PREPARE_REPLAY_RESPONSE_SCHEMA_VERSION,
            ROLE_ATTESTATION_PREPARE_REPLAY_RESPONSE_SCHEMA_VERSION,
            receipt::{
                ReplayReceiptDecision, ReplayReceiptStoreError, ReplayReceiptToken,
                commit_staged_receipt_response, mark_recovery_required, reserve_or_replay_receipt,
                stage_receipt_response, validate_receipt_token,
            },
        },
        runtime::env::EnvOps,
    },
    protocol,
    workflow::{replay::abort_reserved_receipt_after_failure, runtime::auth::RuntimeAuthWorkflow},
};
use admission::{validate_role_attestation_request, validate_token_prepare_public_request};
use replay::{
    encode_role_attestation_prepare_response, encode_token_prepare_response,
    map_role_attestation_replay_decision, map_role_attestation_replay_store_error,
    map_token_prepare_replay_decision, map_token_prepare_replay_store_error, replay_reserve_input,
    role_attestation_replay_command_kind, role_attestation_replay_metadata,
    role_attestation_replay_payload_hash, token_prepare_replay_command_kind,
    token_prepare_replay_payload_hash, token_replay_metadata,
};
use std::future::Future;

impl RuntimeAuthWorkflow {
    /// Prepare a delegated token from issuer-local root-certified delegation material.
    pub async fn prepare_delegated_token(
        request: DelegatedTokenPrepareRequest,
    ) -> Result<DelegatedTokenPrepareResponse, InternalError> {
        let label = "delegated token prepare";
        let metadata = token_replay_metadata(request.metadata, label)?;
        let caller = IcOps::msg_caller();
        validate_token_prepare_public_request(caller, &request)?;
        let command_kind = token_prepare_replay_command_kind();
        let actor = ReplayActor::direct_caller(caller);
        let payload_hash = token_prepare_replay_payload_hash(&command_kind, &actor, &request);
        let now_ns = IcOps::now_nanos();
        let replay_input = replay_reserve_input(
            command_kind,
            metadata.request_id,
            actor,
            payload_hash,
            now_ns,
            metadata.ttl_ns,
            "delegated token prepare replay metadata ttl_ns overflows nanoseconds",
        )?;
        crate::perf!("delegated_token_validate_request");

        let token = match reserve_or_replay_receipt(replay_input)
            .map_err(map_token_prepare_replay_store_error)?
        {
            ReplayReceiptDecision::Fresh(token) => token,
            decision => return map_token_prepare_replay_decision(decision),
        };
        crate::perf!("delegated_token_reserve_replay");

        let prepare_input = PrepareDelegatedTokenIssuerProofInput {
            subject: request.subject,
            audience: request.aud,
            grants: request.grants,
            ttl_ns: request.ttl_ns,
            ext: request.ext,
        };
        let prepared = match prepare_delegated_token_with_lazy_repair(
            prepare_input,
            metadata.request_id,
            caller,
            &token,
        )
        .await
        {
            Ok(prepared) => prepared,
            Err(err) => {
                return Err(abort_reserved_receipt_after_failure(
                    &token,
                    err,
                    "delegated token replay reservation cleanup failed",
                ));
            }
        };
        crate::perf!("delegated_token_prepare_proof");

        let response = DelegatedTokenPrepareResponse {
            claims: prepared.prepared.claims,
            claims_hash: prepared.claims_hash,
            retrieval_expires_at_ns: prepared.retrieval_expires_at_ns,
        };

        let response_bytes = match encode_token_prepare_response(&response) {
            Ok(response_bytes) => response_bytes,
            Err(err) => {
                return Err(preserve_auth_response_failure(
                    &token,
                    err,
                    map_token_prepare_replay_store_error,
                    "delegated token prepare",
                ));
            }
        };
        crate::perf!("delegated_token_encode_response");

        commit_auth_prepare_response(
            &token,
            DELEGATED_TOKEN_PREPARE_REPLAY_RESPONSE_SCHEMA_VERSION,
            response_bytes,
            map_token_prepare_replay_store_error,
            "delegated token prepare",
        )?;
        crate::perf!("delegated_token_commit_replay");
        Ok(response)
    }

    /// Prepare a root-certified role attestation from the local root update path.
    pub fn prepare_role_attestation_root(
        request: RoleAttestationRequest,
    ) -> Result<RoleAttestationPrepareResponse, InternalError> {
        EnvOps::require_root()?;
        let caller = IcOps::msg_caller();
        validate_role_attestation_request(caller, &request)?;
        let metadata = role_attestation_replay_metadata(request.metadata)?;
        let command_kind = role_attestation_replay_command_kind();
        let actor = ReplayActor::direct_caller(caller);
        let payload_hash = role_attestation_replay_payload_hash(&command_kind, &actor, &request);
        let now_ns = IcOps::now_nanos();
        let replay_input = replay_reserve_input(
            command_kind,
            metadata.request_id,
            actor,
            payload_hash,
            now_ns,
            metadata.ttl_ns,
            "role attestation replay metadata ttl_ns overflows nanoseconds",
        )?;

        let token = match reserve_or_replay_receipt(replay_input)
            .map_err(map_role_attestation_replay_store_error)?
        {
            ReplayReceiptDecision::Fresh(token) => token,
            decision => return map_role_attestation_replay_decision(decision),
        };

        let prepared = match AuthOps::prepare_role_attestation(PrepareRootRoleAttestationInput {
            operation_id: token.receipt().operation_id.into_bytes(),
            subject: request.subject,
            role: request.role,
            subnet_id: request.subnet_id,
            audience: request.audience,
            ttl_ns: request.ttl_ns,
            epoch: request.epoch,
            issued_at_ns: now_ns,
        }) {
            Ok(prepared) => prepared,
            Err(err) => {
                return Err(abort_reserved_receipt_after_failure(
                    &token,
                    err,
                    "role attestation replay reservation cleanup failed",
                ));
            }
        };

        let response = RoleAttestationPrepareResponse {
            payload: prepared.payload,
            payload_hash: prepared.payload_hash,
            retrieval_expires_at_ns: prepared.retrieval_expires_at_ns,
        };

        let response_bytes = match encode_role_attestation_prepare_response(&response) {
            Ok(response_bytes) => response_bytes,
            Err(err) => {
                return Err(preserve_auth_response_failure(
                    &token,
                    err,
                    map_role_attestation_replay_store_error,
                    "role attestation prepare",
                ));
            }
        };

        commit_auth_prepare_response(
            &token,
            ROLE_ATTESTATION_PREPARE_REPLAY_RESPONSE_SCHEMA_VERSION,
            response_bytes,
            map_role_attestation_replay_store_error,
            "role attestation prepare",
        )?;
        Ok(response)
    }
}

fn commit_auth_prepare_response(
    token: &ReplayReceiptToken,
    response_schema_version: u32,
    response_bytes: Vec<u8>,
    map_store_error: fn(ReplayReceiptStoreError) -> InternalError,
    label: &'static str,
) -> Result<(), InternalError> {
    if let Err(err) = stage_receipt_response(
        token,
        response_schema_version,
        response_bytes,
        IcOps::now_nanos(),
    ) {
        return Err(preserve_auth_response_failure(
            token,
            map_store_error(err),
            map_store_error,
            label,
        ));
    }
    if let Err(err) = commit_staged_receipt_response(token, IcOps::now_nanos()) {
        return Err(preserve_auth_response_failure(
            token,
            map_store_error(err),
            map_store_error,
            label,
        ));
    }
    Ok(())
}

fn preserve_auth_response_failure(
    token: &ReplayReceiptToken,
    mut err: InternalError,
    map_store_error: fn(ReplayReceiptStoreError) -> InternalError,
    label: &'static str,
) -> InternalError {
    if let Err(recovery_err) = mark_recovery_required(
        token,
        RecoveryReason::ResponseCommitFailed,
        IcOps::now_nanos(),
    )
    .map_err(map_store_error)
    {
        err = err.with_diagnostic_context(format!(
            "{label} replay recovery marker failed: {recovery_err}"
        ));
    }
    err
}

async fn prepare_delegated_token_with_lazy_repair(
    input: PrepareDelegatedTokenIssuerProofInput,
    operation_id: [u8; 32],
    prepared_by: Principal,
    token: &ReplayReceiptToken,
) -> Result<crate::ops::auth::PreparedDelegatedTokenIssuerProof, InternalError> {
    prepare_delegated_token_with_lazy_repair_using(
        input,
        operation_id,
        prepared_by,
        AuthOps::prepare_delegated_token_issuer_proof,
        repair_active_delegation_proof_from_root,
        || validate_receipt_token(token).map_err(map_token_prepare_replay_store_error),
    )
    .await
}

async fn prepare_delegated_token_with_lazy_repair_using<T, P, R, V, Fut>(
    input: PrepareDelegatedTokenIssuerProofInput,
    operation_id: [u8; 32],
    prepared_by: Principal,
    mut prepare: P,
    repair: R,
    validate_replay_owner: V,
) -> Result<T, InternalError>
where
    P: FnMut(
        PrepareDelegatedTokenIssuerProofInput,
        [u8; 32],
        Principal,
    ) -> Result<T, InternalError>,
    R: FnOnce() -> Fut,
    V: FnOnce() -> Result<(), InternalError>,
    Fut: Future<Output = Result<(), InternalError>>,
{
    match prepare(input.clone(), operation_id, prepared_by) {
        Ok(prepared) => Ok(prepared),
        Err(err) if delegated_token_prepare_error_allows_lazy_repair(&err) => {
            repair().await?;
            validate_replay_owner()?;
            prepare(input, operation_id, prepared_by)
        }
        Err(err) => Err(err),
    }
}

fn delegated_token_prepare_error_allows_lazy_repair(err: &InternalError) -> bool {
    err.public_error().is_some_and(|public| {
        matches!(
            public.code,
            ErrorCode::AuthMaterialStale | ErrorCode::AuthProofExpired
        )
    })
}

async fn repair_active_delegation_proof_from_root() -> Result<(), InternalError> {
    let verifier = AuthOps::auth_proof_verifier_config()?;
    let root_canister_id = verifier.root_canister_id;
    crate::perf!("delegated_token_resolve_root");
    let call = CallOps::unbounded_wait(
        root_canister_id,
        protocol::CANIC_GET_OR_CREATE_CHAIN_KEY_DELEGATION_PROOF,
    )
    .execute()
    .await?;
    let proof = chain_key_delegation_proof_from_root_call(call)?;
    crate::perf!("delegated_token_fetch_root_proof");
    AuthOps::install_active_delegation_proof(proof.proof, root_canister_id)?;
    crate::perf!("delegated_token_install_root_proof");
    Ok(())
}

fn chain_key_delegation_proof_from_root_call(
    call: CallResult,
) -> Result<RootDelegationProofBatchProof, InternalError> {
    let result: Result<RootDelegationProofBatchProof, Error> = call.candid()?;
    result.map_err(InternalError::public)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::{
            auth::{AuthRequestMetadata, DelegatedRoleGrant, DelegationAudience},
            error::ErrorCode,
        },
        ids::{CanisterRole, cap},
    };
    use futures::executor::block_on;
    use std::cell::Cell;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn grant(role: &str, scopes: &[&str]) -> DelegatedRoleGrant {
        DelegatedRoleGrant {
            target: CanisterRole::owned(role.to_string()),
            scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
        }
    }

    fn meta(id: u8, ttl_ns: u64) -> AuthRequestMetadata {
        AuthRequestMetadata {
            request_id: [id; 32],
            ttl_ns,
        }
    }

    fn token_prepare_request(metadata_id: u8) -> DelegatedTokenPrepareRequest {
        DelegatedTokenPrepareRequest {
            metadata: Some(meta(metadata_id, 60_000_000_000)),
            subject: p(8),
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![grant("project_instance", &["canic.verify"])],
            ttl_ns: 30_000_000_000,
            ext: None,
        }
    }

    fn token_prepare_input() -> PrepareDelegatedTokenIssuerProofInput {
        let request = token_prepare_request(1);
        PrepareDelegatedTokenIssuerProofInput {
            subject: request.subject,
            audience: request.aud,
            grants: request.grants,
            ttl_ns: request.ttl_ns,
            ext: request.ext,
        }
    }

    #[test]
    fn delegated_token_lazy_repair_retries_once_after_stale_material() {
        let input = token_prepare_input();
        let operation_id = [7; 32];
        let prepared_by = p(8);
        let prepare_calls = Cell::new(0);
        let repair_calls = Cell::new(0);

        let prepared = block_on(prepare_delegated_token_with_lazy_repair_using(
            input.clone(),
            operation_id,
            prepared_by,
            |observed, observed_operation_id, observed_prepared_by| {
                assert_eq!(observed.subject, input.subject);
                assert_eq!(observed.ttl_ns, input.ttl_ns);
                assert_eq!(observed_operation_id, operation_id);
                assert_eq!(observed_prepared_by, prepared_by);
                let call = prepare_calls.get();
                prepare_calls.set(call + 1);
                if call == 0 {
                    Err(InternalError::auth_material_stale(
                        "active delegation proof is stale",
                    ))
                } else {
                    Ok(42_u8)
                }
            },
            || async {
                repair_calls.set(repair_calls.get() + 1);
                Ok(())
            },
            || Ok(()),
        ))
        .expect("lazy repair should retry and return prepared material");

        assert_eq!(prepared, 42);
        assert_eq!(prepare_calls.get(), 2);
        assert_eq!(repair_calls.get(), 1);
    }

    #[test]
    fn delegated_token_lazy_repair_revalidates_replay_owner_before_retry() {
        let prepare_calls = Cell::new(0);
        let repair_calls = Cell::new(0);
        let validation_calls = Cell::new(0);

        let err = block_on(prepare_delegated_token_with_lazy_repair_using(
            token_prepare_input(),
            [11; 32],
            p(8),
            |_, _, _| -> Result<u8, InternalError> {
                prepare_calls.set(prepare_calls.get() + 1);
                Err(InternalError::auth_material_stale(
                    "active delegation proof is stale",
                ))
            },
            || async {
                repair_calls.set(repair_calls.get() + 1);
                Ok(())
            },
            || {
                validation_calls.set(validation_calls.get() + 1);
                Err(InternalError::public(Error::conflict(
                    "replay ownership changed",
                )))
            },
        ))
        .expect_err("stale replay ownership must prevent the second prepare");

        assert_eq!(
            err.public_error().expect("public error").code,
            ErrorCode::Conflict
        );
        assert_eq!(prepare_calls.get(), 1);
        assert_eq!(repair_calls.get(), 1);
        assert_eq!(validation_calls.get(), 1);
    }

    #[test]
    fn delegated_token_lazy_repair_does_not_call_root_when_prepare_succeeds() {
        let input = token_prepare_input();
        let prepare_calls = Cell::new(0);
        let repair_calls = Cell::new(0);

        let prepared = block_on(prepare_delegated_token_with_lazy_repair_using(
            input,
            [10; 32],
            p(8),
            |_, _, _| {
                prepare_calls.set(prepare_calls.get() + 1);
                Ok(7_u8)
            },
            || async {
                repair_calls.set(repair_calls.get() + 1);
                Ok(())
            },
            || Ok(()),
        ))
        .expect("fresh local active proof should prepare without root repair");

        assert_eq!(prepared, 7);
        assert_eq!(prepare_calls.get(), 1);
        assert_eq!(repair_calls.get(), 0);
    }

    #[test]
    fn delegated_token_lazy_repair_returns_pending_without_second_prepare_attempt() {
        let input = token_prepare_input();
        let prepare_calls = Cell::new(0);
        let repair_calls = Cell::new(0);

        let err = block_on(prepare_delegated_token_with_lazy_repair_using(
            input,
            [8; 32],
            p(8),
            |_, _, _| -> Result<u8, InternalError> {
                prepare_calls.set(prepare_calls.get() + 1);
                Err(InternalError::auth_proof_expired(
                    "active delegation proof expired",
                ))
            },
            || async {
                repair_calls.set(repair_calls.get() + 1);
                Err(InternalError::auth_proof_pending(
                    "chain-key root delegation proof is not available yet; retry",
                ))
            },
            || Ok(()),
        ))
        .expect_err("pending root repair must not issue a token");

        assert_eq!(
            err.public_error().expect("public error").code,
            ErrorCode::AuthProofPending
        );
        assert_eq!(prepare_calls.get(), 1);
        assert_eq!(repair_calls.get(), 1);
    }

    #[test]
    fn delegated_token_lazy_repair_does_not_run_for_non_repairable_errors() {
        let input = token_prepare_input();
        let prepare_calls = Cell::new(0);
        let repair_calls = Cell::new(0);

        let err = block_on(prepare_delegated_token_with_lazy_repair_using(
            input,
            [9; 32],
            p(8),
            |_, _, _| -> Result<u8, InternalError> {
                prepare_calls.set(prepare_calls.get() + 1);
                Err(InternalError::public(Error::forbidden(
                    "delegated token prepare subject must match caller",
                )))
            },
            || async {
                repair_calls.set(repair_calls.get() + 1);
                Ok(())
            },
            || Ok(()),
        ))
        .expect_err("non-repairable error should pass through");

        assert_eq!(
            err.public_error().expect("public error").code,
            ErrorCode::Forbidden
        );
        assert_eq!(prepare_calls.get(), 1);
        assert_eq!(repair_calls.get(), 0);
    }

    #[test]
    fn delegated_token_replay_metadata_rejects_missing_or_invalid_ttl() {
        let missing = token_replay_metadata(None, "delegated token prepare").expect_err("required");
        assert_eq!(
            missing.public_error().expect("public error").code,
            ErrorCode::OperationIdRequired
        );

        let zero = token_replay_metadata(Some(meta(1, 0)), "delegated token prepare")
            .expect_err("zero ttl is invalid");
        assert_eq!(
            zero.public_error().expect("public error").code,
            ErrorCode::InvalidInput
        );

        let too_large = token_replay_metadata(
            Some(meta(1, replay::MAX_TOKEN_REPLAY_TTL_NS + 1)),
            "delegated token prepare",
        )
        .expect_err("oversized ttl is invalid");
        assert_eq!(
            too_large.public_error().expect("public error").code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn delegated_token_public_prepare_rejects_subject_mismatch_before_replay() {
        let mut request = token_prepare_request(1);
        request.subject = p(9);
        request.grants = vec![grant("project_instance", &[cap::SESSION])];

        let err = validate_token_prepare_public_request(p(8), &request)
            .expect_err("subject mismatch must fail");

        assert_eq!(
            err.public_error().expect("public error").code,
            ErrorCode::Forbidden
        );
    }

    #[test]
    fn delegated_token_public_prepare_rejects_privileged_self_grants_before_replay() {
        let mut request = token_prepare_request(1);
        request.grants = vec![grant("project_instance", &[cap::WRITE])];

        let err = validate_token_prepare_public_request(p(8), &request)
            .expect_err("privileged self-grant must fail");

        assert_eq!(
            err.public_error().expect("public error").code,
            ErrorCode::Forbidden
        );
    }

    #[test]
    fn delegated_token_public_prepare_accepts_login_scopes_before_replay() {
        let mut request = token_prepare_request(1);
        request.grants = vec![
            grant("project_hub", &[cap::SESSION]),
            grant("project_instance", &[cap::VERIFY]),
        ];

        validate_token_prepare_public_request(p(8), &request).expect("login scopes");
    }

    #[test]
    fn delegated_token_prepare_payload_hash_ignores_metadata() {
        let command_kind = token_prepare_replay_command_kind();
        let actor = ReplayActor::direct_caller(p(2));
        let a = token_prepare_request(1);
        let b = token_prepare_request(9);

        assert_eq!(
            token_prepare_replay_payload_hash(&command_kind, &actor, &a),
            token_prepare_replay_payload_hash(&command_kind, &actor, &b)
        );
    }

    #[test]
    fn delegated_token_prepare_payload_hash_binds_authoritative_payload() {
        let command_kind = token_prepare_replay_command_kind();
        let actor = ReplayActor::direct_caller(p(2));
        let a = token_prepare_request(1);
        let mut b = a.clone();
        b.ttl_ns += 1;

        assert_ne!(
            token_prepare_replay_payload_hash(&command_kind, &actor, &a),
            token_prepare_replay_payload_hash(&command_kind, &actor, &b)
        );
    }

    #[test]
    fn delegated_token_prepare_payload_hash_binds_ext() {
        let command_kind = token_prepare_replay_command_kind();
        let actor = ReplayActor::direct_caller(p(2));
        let a = token_prepare_request(1);
        let mut b = a.clone();
        b.ext = Some(b"app-context".to_vec());

        assert_ne!(
            token_prepare_replay_payload_hash(&command_kind, &actor, &a),
            token_prepare_replay_payload_hash(&command_kind, &actor, &b)
        );
    }
}
