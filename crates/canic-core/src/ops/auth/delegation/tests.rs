use super::{
    active::active_delegation_proof_status_response,
    batch::{
        RootDelegationProofBatchPrepareContext, get_delegation_proof_batch_with_root_proof,
        preflight_delegation_proof_batch_install_proof,
        preflight_delegation_proof_batch_prepare_request,
        prepare_delegation_proof_batch_with_root_proof,
        prepare_delegation_proof_batch_with_root_proof_replay,
    },
    pending::{
        MAX_PENDING_ROOT_DELEGATION_PROOFS_PER_ISSUER, MAX_ROOT_DELEGATION_PROOF_BATCH_ISSUERS,
        cache_prepared_delegation_proof_batch_replay, mark_delegation_proof_batch_installed,
        pending_delegation_proof_batch_entry, pending_delegation_proof_batch_replay_response,
        prune_expired_pending_delegation_proof_batch_metadata,
    },
};
use crate::{
    InternalError, InternalErrorClass,
    cdk::types::Principal,
    domain::policy::auth::{
        RootDelegatedRoleGrantPolicy, RootDelegationAudiencePolicy,
        RootDelegationProofPreparePolicyDecision, RootIssuerPolicy,
    },
    dto::auth::{
        ActiveDelegationProof, ActiveDelegationProofStatus, AuthRequestMetadata,
        DelegatedRoleGrant, DelegationAudience, DelegationCert, DelegationProof,
        IcCanisterSignatureProofV1, IssuerProofAlgorithm, IssuerProofBinding,
        RootDelegationProofBatchGetRequest, RootDelegationProofBatchPrepareEntry,
        RootDelegationProofBatchPrepareRequest, RootDelegationProofBatchPrepareResponse,
        RootDelegationProofBatchProofRef, RootDelegationProofInstallOutcome, RootProof,
    },
    dto::error::ErrorCode,
    ids::{CanisterRole, cap},
    ops::{auth::AuthOps, storage::auth::AuthStateOps},
};

fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

fn active_proof() -> ActiveDelegationProof {
    ActiveDelegationProof {
        proof: DelegationProof {
            cert: DelegationCert {
                root_pid: p(1),
                issuer_pid: p(2),
                issuer_proof_alg: IssuerProofAlgorithm::IcCanisterSignatureV1,
                issuer_proof_binding_hash: [3; 32],
                issuer_proof_binding: IssuerProofBinding::IcCanisterSignatureV1 {
                    seed_hash: [4; 32],
                },
                issued_at_ns: 10,
                not_before_ns: 20,
                expires_at_ns: 100,
                max_token_ttl_ns: 30,
                aud: DelegationAudience::CanicSubnet(p(7)),
                grants: vec![DelegatedRoleGrant {
                    target: CanisterRole::owned("project_instance".to_string()),
                    scopes: vec!["canic.issue".to_string()],
                }],
            },
            root_proof: RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
                signature_cbor: vec![8; 64],
                public_key_der: vec![9; 32],
            }),
        },
        cert_hash: [10; 32],
        not_before_ns: 20,
        expires_at_ns: 100,
        refresh_after_ns: 80,
        installed_at_ns: 20,
        installed_by: p(11),
    }
}

fn assert_public_error_code(err: &InternalError, expected: ErrorCode) {
    let public = err.public_error().expect("expected public error");
    assert_eq!(public.code, expected);
}

fn batch_prepare_entry(
    issuer_pid: Principal,
    cert_ttl_ns: u64,
) -> RootDelegationProofBatchPrepareEntry {
    RootDelegationProofBatchPrepareEntry {
        issuer_pid,
        aud: DelegationAudience::Project("test".to_string()),
        grants: vec![DelegatedRoleGrant {
            target: CanisterRole::owned("project_instance".to_string()),
            scopes: vec![cap::READ.to_string()],
        }],
        cert_ttl_ns,
    }
}

fn batch_prepare_request(
    issuer_pid: Principal,
    cert_ttl_ns: u64,
) -> RootDelegationProofBatchPrepareRequest {
    RootDelegationProofBatchPrepareRequest {
        metadata: None,
        entries: vec![batch_prepare_entry(issuer_pid, cert_ttl_ns)],
    }
}

fn metadata(id: u8, ttl_ns: u64) -> AuthRequestMetadata {
    AuthRequestMetadata {
        request_id: [id; 32],
        ttl_ns,
    }
}

fn root_issuer_policy(issuer_pid: Principal) -> RootIssuerPolicy {
    RootIssuerPolicy {
        issuer_pid,
        enabled: true,
        allowed_audiences: vec![RootDelegationAudiencePolicy::Project("test".to_string())],
        allowed_grants: vec![RootDelegatedRoleGrantPolicy {
            target: CanisterRole::owned("project_instance".to_string()),
            scopes: vec![cap::READ.to_string()],
        }],
        max_cert_ttl_ns: 120_000_000_000,
        refresh_after_ratio_bps: 8_000,
    }
}

fn root_proof(byte: u8) -> RootProof {
    RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
        signature_cbor: vec![byte; 8],
        public_key_der: vec![byte; 4],
    })
}

fn prepared_batch(
    issuer_pid: Principal,
    metadata_id: u8,
    retrieval_expires_at_ns: u64,
) -> RootDelegationProofBatchPrepareResponse {
    AuthStateOps::upsert_root_issuer_policy(root_issuer_policy(issuer_pid));
    let mut request = batch_prepare_request(issuer_pid, 60_000_000_000);
    request.metadata = Some(metadata(metadata_id, 60_000_000_000));
    let decisions = preflight_delegation_proof_batch_prepare_request(&request, 10).unwrap();

    prepare_delegation_proof_batch_with_root_proof(
        request,
        [metadata_id; 32],
        decisions,
        120_000_000_000,
        10,
        p(1),
        |batch_id, _cert_hash| {
            assert_eq!(batch_id, [metadata_id; 32]);
            Ok(retrieval_expires_at_ns)
        },
    )
    .expect("batch prepare should produce metadata")
}

fn batch_get_request(
    response: &RootDelegationProofBatchPrepareResponse,
) -> RootDelegationProofBatchGetRequest {
    RootDelegationProofBatchGetRequest {
        batch_id: response.batch_id,
        entries: response
            .entries
            .iter()
            .map(|entry| RootDelegationProofBatchProofRef {
                issuer_pid: entry.issuer_pid,
                cert_hash: entry.cert_hash,
            })
            .collect(),
    }
}

#[test]
fn active_delegation_proof_status_reports_missing() {
    let status = active_delegation_proof_status_response(50, None);

    assert_eq!(status.status, ActiveDelegationProofStatus::Missing);
    assert_eq!(status.root_pid, None);
    assert_eq!(status.cert_hash, None);
}

#[test]
fn active_delegation_proof_status_reports_lifecycle_states() {
    let valid = active_delegation_proof_status_response(79, Some(active_proof()));
    assert_eq!(valid.status, ActiveDelegationProofStatus::Valid);
    assert_eq!(valid.root_pid, Some(p(1)));
    assert_eq!(valid.issuer_pid, Some(p(2)));
    assert_eq!(valid.cert_hash, Some([10; 32]));
    assert_eq!(valid.expires_at_ns, Some(100));
    assert_eq!(valid.refresh_after_ns, Some(80));

    let refresh = active_delegation_proof_status_response(80, Some(active_proof()));
    assert_eq!(refresh.status, ActiveDelegationProofStatus::RefreshNeeded);

    let expired = active_delegation_proof_status_response(100, Some(active_proof()));
    assert_eq!(expired.status, ActiveDelegationProofStatus::Expired);
}

#[test]
fn batch_prepare_preflight_accepts_registered_issuer_policy() {
    AuthStateOps::upsert_root_issuer_policy(root_issuer_policy(p(21)));

    let decisions = preflight_delegation_proof_batch_prepare_request(
        &batch_prepare_request(p(21), 60_000_000_000),
        10,
    )
    .expect("valid batch prepare shape");

    assert_eq!(
        decisions,
        vec![RootDelegationProofPreparePolicyDecision {
            expires_at_ns: 60_000_000_010,
            refresh_after_ns: 48_000_000_010,
        }]
    );
}

#[test]
fn batch_prepare_preflight_rejects_ttl_above_max() {
    AuthStateOps::upsert_root_issuer_policy(root_issuer_policy(p(22)));

    let err = preflight_delegation_proof_batch_prepare_request(
        &batch_prepare_request(p(22), 121_000_000_000),
        10,
    )
    .expect_err("ttl above max must fail preflight");
    assert_public_error_code(&err, ErrorCode::Forbidden);
}

#[test]
fn batch_prepare_preflight_rejects_unregistered_issuer() {
    let err = preflight_delegation_proof_batch_prepare_request(
        &batch_prepare_request(p(23), 60_000_000_000),
        10,
    )
    .expect_err("unregistered issuer must fail preflight");
    assert_public_error_code(&err, ErrorCode::Forbidden);
}

#[test]
fn batch_prepare_preflight_rejects_disabled_issuer() {
    let mut policy = root_issuer_policy(p(24));
    policy.enabled = false;
    AuthStateOps::upsert_root_issuer_policy(policy);

    let err = preflight_delegation_proof_batch_prepare_request(
        &batch_prepare_request(p(24), 60_000_000_000),
        10,
    )
    .expect_err("disabled issuer must fail preflight");
    assert_public_error_code(&err, ErrorCode::Forbidden);
}

#[test]
fn batch_prepare_preflight_rejects_grant_outside_issuer_policy() {
    let mut policy = root_issuer_policy(p(25));
    policy.allowed_grants[0].scopes = vec!["canic.read".to_string()];
    AuthStateOps::upsert_root_issuer_policy(policy);

    let err = preflight_delegation_proof_batch_prepare_request(
        &batch_prepare_request(p(25), 60_000_000_000),
        10,
    )
    .expect_err("grant outside issuer policy must fail preflight");
    assert_public_error_code(&err, ErrorCode::Forbidden);
}

#[test]
fn batch_prepare_rejects_missing_metadata() {
    AuthStateOps::upsert_root_issuer_policy(root_issuer_policy(p(26)));

    let err = AuthOps::prepare_delegation_proof_batch(
        batch_prepare_request(p(26), 60_000_000_000),
        120_000_000_000,
        10,
    )
    .expect_err("batch prepare requires request id metadata");
    assert_public_error_code(&err, ErrorCode::OperationIdRequired);
}

#[test]
fn batch_prepare_rejects_empty_entries() {
    let request = RootDelegationProofBatchPrepareRequest {
        metadata: Some(metadata(27, 60_000_000_000)),
        entries: vec![],
    };

    let err = AuthOps::prepare_delegation_proof_batch(request, 120_000_000_000, 10)
        .expect_err("empty batch must fail");
    assert_public_error_code(&err, ErrorCode::InvalidInput);
}

#[test]
fn batch_prepare_rejects_invalid_metadata_ttl() {
    let request = RootDelegationProofBatchPrepareRequest {
        metadata: Some(metadata(28, 0)),
        entries: vec![batch_prepare_entry(p(28), 60_000_000_000)],
    };

    let err = AuthOps::prepare_delegation_proof_batch(request, 120_000_000_000, 10)
        .expect_err("zero metadata ttl must fail");
    assert_public_error_code(&err, ErrorCode::InvalidInput);
}

#[test]
fn batch_prepare_rejects_batch_size_above_mvp_limit() {
    let request = RootDelegationProofBatchPrepareRequest {
        metadata: Some(metadata(52, 60_000_000_000)),
        entries: (0..=MAX_ROOT_DELEGATION_PROOF_BATCH_ISSUERS)
            .map(|index| {
                let issuer_id = u8::try_from(index).expect("test issuer index must fit in u8");
                batch_prepare_entry(p(issuer_id), 60_000_000_000)
            })
            .collect(),
    };

    let err = AuthOps::prepare_delegation_proof_batch(request, 120_000_000_000, 10)
        .expect_err("oversized batch must fail before policy validation");
    assert_public_error_code(&err, ErrorCode::ResourceExhausted);
}

#[test]
fn batch_prepare_rejects_pending_issuer_quota_exhaustion() {
    let issuer_pid = p(53);
    for offset in 0..MAX_PENDING_ROOT_DELEGATION_PROOFS_PER_ISSUER {
        let metadata_id = 80 + u8::try_from(offset).expect("test offset must fit in u8");
        prepared_batch(issuer_pid, metadata_id, 120_000_000_000);
    }
    let mut request = batch_prepare_request(issuer_pid, 60_000_000_000);
    request.metadata = Some(metadata(97, 60_000_000_000));

    let err = AuthOps::prepare_delegation_proof_batch(request, 120_000_000_000, 10)
        .expect_err("pending issuer quota exhaustion must fail before preparing proof leaves");
    assert_public_error_code(&err, ErrorCode::ResourceExhausted);
}

#[test]
fn batch_prepare_returns_metadata_for_registered_issuer() {
    AuthStateOps::upsert_root_issuer_policy(root_issuer_policy(p(29)));
    let mut request = batch_prepare_request(p(29), 60_000_000_000);
    request.metadata = Some(metadata(29, 60_000_000_000));
    let decisions = preflight_delegation_proof_batch_prepare_request(&request, 10).unwrap();
    let mut prepared_hashes = Vec::new();

    let response = prepare_delegation_proof_batch_with_root_proof(
        request,
        [29; 32],
        decisions,
        120_000_000_000,
        10,
        p(1),
        |batch_id, cert_hash| {
            assert_eq!(batch_id, [29; 32]);
            prepared_hashes.push(cert_hash);
            Ok(70)
        },
    )
    .expect("batch prepare should produce metadata");

    assert_eq!(response.batch_id, [29; 32]);
    assert_eq!(response.entries.len(), 1);
    assert_eq!(response.entries[0].issuer_pid, p(29));
    assert_eq!(response.entries[0].expires_at_ns, 60_000_000_010);
    assert_eq!(response.entries[0].refresh_after_ns, 48_000_000_010);
    assert_eq!(response.retrieval_expires_at_ns, 70);
    assert_eq!(prepared_hashes, vec![response.entries[0].cert_hash]);
}

#[test]
fn batch_prepare_replays_same_request_id_without_resigning() {
    AuthStateOps::upsert_root_issuer_policy(root_issuer_policy(p(30)));
    let mut request = batch_prepare_request(p(30), 60_000_000_000);
    let metadata = metadata(30, 60_000_000_000);
    request.metadata = Some(metadata);
    let context = RootDelegationProofBatchPrepareContext {
        metadata,
        max_cert_ttl_ns: 120_000_000_000,
        issued_at_ns: 10,
    };
    let mut prepare_count = 0;

    let first = prepare_delegation_proof_batch_with_root_proof_replay(
        request.clone(),
        context,
        |request| preflight_delegation_proof_batch_prepare_request(request, 10),
        || p(1),
        |batch_id, _cert_hash| {
            assert_eq!(batch_id, [30; 32]);
            prepare_count += 1;
            Ok(70)
        },
    )
    .expect("first batch prepare should produce metadata");

    let replay = prepare_delegation_proof_batch_with_root_proof_replay(
        request,
        RootDelegationProofBatchPrepareContext {
            issued_at_ns: 20,
            ..context
        },
        |_request| -> Result<Vec<RootDelegationProofPreparePolicyDecision>, InternalError> {
            panic!("cached replay must not rerun policy preflight")
        },
        || p(1),
        |_batch_id, _cert_hash| -> Result<u64, InternalError> {
            panic!("cached replay must not prepare a new root proof")
        },
    )
    .expect("same request id and payload should replay original metadata");

    assert_eq!(replay, first);
    assert_eq!(prepare_count, 1);
}

#[test]
fn batch_prepare_rejects_conflicting_request_id_reuse() {
    AuthStateOps::upsert_root_issuer_policy(root_issuer_policy(p(31)));
    let mut request = batch_prepare_request(p(31), 60_000_000_000);
    let metadata = metadata(31, 60_000_000_000);
    request.metadata = Some(metadata);
    let context = RootDelegationProofBatchPrepareContext {
        metadata,
        max_cert_ttl_ns: 120_000_000_000,
        issued_at_ns: 10,
    };

    prepare_delegation_proof_batch_with_root_proof_replay(
        request.clone(),
        context,
        |request| preflight_delegation_proof_batch_prepare_request(request, 10),
        || p(1),
        |_batch_id, _cert_hash| Ok(70),
    )
    .expect("first batch prepare should produce metadata");

    let mut conflicting = request;
    conflicting.entries[0].cert_ttl_ns = 30_000_000_000;
    let err = prepare_delegation_proof_batch_with_root_proof_replay(
        conflicting,
        RootDelegationProofBatchPrepareContext {
            issued_at_ns: 20,
            ..context
        },
        |_request| -> Result<Vec<RootDelegationProofPreparePolicyDecision>, InternalError> {
            panic!("conflicting replay must fail before policy preflight")
        },
        || p(1),
        |_batch_id, _cert_hash| -> Result<u64, InternalError> {
            panic!("conflicting replay must fail before root proof preparation")
        },
    )
    .expect_err("request id reuse with a different payload must fail");
    assert_public_error_code(&err, ErrorCode::InvalidInput);
}

#[test]
fn batch_get_rejects_empty_entries() {
    let err = get_delegation_proof_batch_with_root_proof(
        RootDelegationProofBatchGetRequest {
            batch_id: [40; 32],
            entries: vec![],
        },
        p(1),
        10,
        |_cert_hash| panic!("empty get request must not request a root proof"),
    )
    .expect_err("empty get request must fail");
    assert_public_error_code(&err, ErrorCode::InvalidInput);
}

#[test]
fn batch_get_rejects_missing_pending_metadata() {
    let err = get_delegation_proof_batch_with_root_proof(
        RootDelegationProofBatchGetRequest {
            batch_id: [41; 32],
            entries: vec![RootDelegationProofBatchProofRef {
                issuer_pid: p(41),
                cert_hash: [1; 32],
            }],
        },
        p(1),
        10,
        |_cert_hash| panic!("missing pending entry must not request a root proof"),
    )
    .expect_err("missing pending entry must fail");

    assert_eq!(err.class(), InternalErrorClass::Ops);
}

#[test]
fn batch_get_rejects_expired_pending_metadata() {
    let response = prepared_batch(p(42), 42, 70);

    let err = get_delegation_proof_batch_with_root_proof(
        batch_get_request(&response),
        p(1),
        70,
        |_cert_hash| panic!("expired pending entry must not request a root proof"),
    )
    .expect_err("expired pending entry must fail");

    assert_eq!(err.class(), InternalErrorClass::Ops);
}

#[test]
fn batch_get_returns_prepared_proofs() {
    let response = prepared_batch(p(43), 43, 90);
    let mut requested_hashes = Vec::new();

    let retrieved = get_delegation_proof_batch_with_root_proof(
        batch_get_request(&response),
        p(1),
        80,
        |cert_hash| {
            requested_hashes.push(cert_hash);
            Ok(root_proof(9))
        },
    )
    .expect("prepared batch should retrieve proofs");

    assert_eq!(retrieved.batch_id, response.batch_id);
    assert_eq!(retrieved.proofs.len(), 1);
    assert_eq!(retrieved.proofs[0].issuer_pid, p(43));
    assert_eq!(retrieved.proofs[0].cert_hash, response.entries[0].cert_hash);
    assert_eq!(retrieved.proofs[0].proof.cert.root_pid, p(1));
    assert_eq!(retrieved.proofs[0].proof.cert.issuer_pid, p(43));
    assert_eq!(requested_hashes, vec![response.entries[0].cert_hash]);
}

#[test]
fn batch_get_preserves_requested_entry_order() {
    AuthStateOps::upsert_root_issuer_policy(root_issuer_policy(p(44)));
    AuthStateOps::upsert_root_issuer_policy(root_issuer_policy(p(45)));
    let request = RootDelegationProofBatchPrepareRequest {
        metadata: Some(metadata(44, 60_000_000_000)),
        entries: vec![
            batch_prepare_entry(p(44), 60_000_000_000),
            batch_prepare_entry(p(45), 60_000_000_000),
        ],
    };
    let decisions = preflight_delegation_proof_batch_prepare_request(&request, 10).unwrap();
    let response = prepare_delegation_proof_batch_with_root_proof(
        request,
        [44; 32],
        decisions,
        120_000_000_000,
        10,
        p(1),
        |_batch_id, _cert_hash| Ok(90),
    )
    .expect("batch prepare should produce metadata");
    let requested_entries = response
        .entries
        .iter()
        .rev()
        .map(|entry| RootDelegationProofBatchProofRef {
            issuer_pid: entry.issuer_pid,
            cert_hash: entry.cert_hash,
        })
        .collect::<Vec<_>>();
    let expected_hashes = requested_entries
        .iter()
        .map(|entry| entry.cert_hash)
        .collect::<Vec<_>>();
    let expected_issuers = requested_entries
        .iter()
        .map(|entry| entry.issuer_pid)
        .collect::<Vec<_>>();
    let mut requested_hashes = Vec::new();

    let retrieved = get_delegation_proof_batch_with_root_proof(
        RootDelegationProofBatchGetRequest {
            batch_id: response.batch_id,
            entries: requested_entries,
        },
        p(1),
        80,
        |cert_hash| {
            requested_hashes.push(cert_hash);
            Ok(root_proof(10))
        },
    )
    .expect("prepared batch should retrieve proofs");

    let retrieved_hashes = retrieved
        .proofs
        .iter()
        .map(|proof| proof.cert_hash)
        .collect::<Vec<_>>();
    let retrieved_issuers = retrieved
        .proofs
        .iter()
        .map(|proof| proof.issuer_pid)
        .collect::<Vec<_>>();
    assert_eq!(retrieved_hashes, expected_hashes);
    assert_eq!(retrieved_issuers, expected_issuers);
    assert_eq!(requested_hashes, expected_hashes);
}

#[test]
fn batch_install_preflight_accepts_retrieved_proof() {
    let response = prepared_batch(p(46), 46, 90);
    let retrieved = get_delegation_proof_batch_with_root_proof(
        batch_get_request(&response),
        p(1),
        80,
        |_cert_hash| Ok(root_proof(11)),
    )
    .expect("prepared batch should retrieve proofs");

    assert_eq!(
        preflight_delegation_proof_batch_install_proof(response.batch_id, &retrieved.proofs[0], 80,),
        Ok(())
    );
}

#[test]
fn batch_install_preflight_rejects_proof_mismatch() {
    let response = prepared_batch(p(47), 47, 90);
    let retrieved = get_delegation_proof_batch_with_root_proof(
        batch_get_request(&response),
        p(1),
        80,
        |_cert_hash| Ok(root_proof(12)),
    )
    .expect("prepared batch should retrieve proofs");
    let mut proof = retrieved.proofs[0].clone();
    proof.cert_hash = [9; 32];

    assert_eq!(
        preflight_delegation_proof_batch_install_proof(response.batch_id, &proof, 80),
        Err(RootDelegationProofInstallOutcome::ProofMismatch)
    );
}

#[test]
fn batch_install_preflight_rejects_stale_pending_metadata() {
    let response = prepared_batch(p(48), 48, 70);
    let retrieved = get_delegation_proof_batch_with_root_proof(
        batch_get_request(&response),
        p(1),
        60,
        |_cert_hash| Ok(root_proof(13)),
    )
    .expect("prepared batch should retrieve proofs before expiry");

    assert_eq!(
        preflight_delegation_proof_batch_install_proof(response.batch_id, &retrieved.proofs[0], 70,),
        Err(RootDelegationProofInstallOutcome::ExpiredOrSuperseded)
    );
}

#[test]
fn batch_install_preflight_reports_already_installed_after_success_mark() {
    let response = prepared_batch(p(49), 49, 70);
    let retrieved = get_delegation_proof_batch_with_root_proof(
        batch_get_request(&response),
        p(1),
        60,
        |_cert_hash| Ok(root_proof(14)),
    )
    .expect("prepared batch should retrieve proofs before expiry");
    let proof = &retrieved.proofs[0];
    assert_eq!(
        preflight_delegation_proof_batch_install_proof(response.batch_id, proof, 60),
        Ok(())
    );

    mark_delegation_proof_batch_installed(response.batch_id, proof.issuer_pid, proof.cert_hash);

    assert_eq!(
        preflight_delegation_proof_batch_install_proof(response.batch_id, proof, 80),
        Err(RootDelegationProofInstallOutcome::AlreadyInstalled)
    );
}

#[test]
fn pending_batch_cleanup_prunes_uninstalled_expired_metadata_and_replays() {
    let response = prepared_batch(p(50), 50, 70);
    let replay_fingerprint = [5; 32];
    cache_prepared_delegation_proof_batch_replay(
        response.batch_id,
        replay_fingerprint,
        response.clone(),
        70,
    );

    let cleanup = prune_expired_pending_delegation_proof_batch_metadata(70);
    assert!(cleanup.pending_entries >= 1);
    assert!(cleanup.replay_entries >= 1);

    let err = get_delegation_proof_batch_with_root_proof(
        batch_get_request(&response),
        p(1),
        70,
        |_cert_hash| panic!("pruned pending metadata must not request a root proof"),
    )
    .expect_err("expired pending metadata should be pruned");
    assert_eq!(err.class(), InternalErrorClass::Ops);

    let replay =
        pending_delegation_proof_batch_replay_response(response.batch_id, replay_fingerprint, 70)
            .expect("replay lookup should not fail");
    assert_eq!(replay, None);
}

#[test]
fn pending_batch_cleanup_keeps_installed_metadata_until_cert_expiry() {
    let response = prepared_batch(p(51), 51, 70);
    let retrieved = get_delegation_proof_batch_with_root_proof(
        batch_get_request(&response),
        p(1),
        60,
        |_cert_hash| Ok(root_proof(15)),
    )
    .expect("prepared batch should retrieve proofs before expiry");
    let proof = retrieved.proofs[0].clone();

    mark_delegation_proof_batch_installed(response.batch_id, proof.issuer_pid, proof.cert_hash);
    prune_expired_pending_delegation_proof_batch_metadata(80);

    assert!(
        pending_delegation_proof_batch_entry(response.batch_id, proof.issuer_pid, proof.cert_hash,)
            .is_ok(),
        "installed metadata should remain available for idempotent reinstall"
    );
    assert_eq!(
        preflight_delegation_proof_batch_install_proof(response.batch_id, &proof, 80),
        Err(RootDelegationProofInstallOutcome::AlreadyInstalled)
    );

    prune_expired_pending_delegation_proof_batch_metadata(response.entries[0].expires_at_ns);
    assert!(
        pending_delegation_proof_batch_entry(response.batch_id, proof.issuer_pid, proof.cert_hash,)
            .is_err(),
        "installed metadata should be removed once the certificate expires"
    );
}
