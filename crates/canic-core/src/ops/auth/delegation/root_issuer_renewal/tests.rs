use super::*;
use crate::{
    cdk::types::Principal,
    domain::policy::auth::{
        RootDelegatedRoleGrantPolicy, RootDelegationAudiencePolicy, RootDelegationRenewalBatch,
        RootIssuerPolicy, RootIssuerRenewalAttempt, RootIssuerRenewalProofRef,
        RootIssuerRenewalState,
    },
    dto::auth::{
        DelegatedRoleGrant, DelegationAudience, DelegationCert, DelegationProof,
        IcCanisterSignatureProofV1, IssuerProofAlgorithm, IssuerProofBinding,
        RootDelegationProofBatchEntry, RootDelegationProofBatchGetResponse,
        RootDelegationProofBatchPrepareRequest, RootDelegationProofBatchPrepareResponse,
        RootDelegationProofBatchProof, RootDelegationProofBatchProofRef,
        RootDelegationRenewalProofBatchGetRequest, RootIssuerRenewalAttemptStatus,
        RootIssuerRenewalOutcome, RootProof,
    },
    dto::error::ErrorCode,
    ids::CanisterRole,
    ops::{
        runtime::metrics::delegated_auth::{DelegatedAuthMetricKey, DelegatedAuthMetricOperation},
        storage::auth::AuthStateOps,
    },
};

fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

fn grant(scope: &str) -> DelegatedRoleGrant {
    DelegatedRoleGrant {
        target: CanisterRole::owned("project_instance".to_string()),
        scopes: vec![scope.to_string()],
    }
}

fn policy(issuer_pid: Principal) -> RootIssuerPolicy {
    RootIssuerPolicy {
        issuer_pid,
        enabled: true,
        allowed_audiences: vec![RootDelegationAudiencePolicy::Project("test".to_string())],
        allowed_grants: vec![RootDelegatedRoleGrantPolicy {
            target: CanisterRole::owned("project_instance".to_string()),
            scopes: vec!["canic.issue".to_string()],
        }],
        max_cert_ttl_ns: 120_000_000_000,
        refresh_after_ratio_bps: 8_000,
    }
}

fn upsert_request(issuer_pid: Principal) -> RootIssuerRenewalTemplateUpsertRequest {
    RootIssuerRenewalTemplateUpsertRequest {
        issuer_pid,
        enabled: true,
        aud: DelegationAudience::Project("test".to_string()),
        grants: vec![grant("canic.issue")],
        cert_ttl_ns: 60_000_000_000,
    }
}

fn renewal_attempt(
    attempt_id: [u8; 32],
    batch_id: [u8; 32],
    issuer_pid: Principal,
    cert_hash: [u8; 32],
) -> RootIssuerRenewalAttempt {
    RootIssuerRenewalAttempt {
        attempt_id,
        issuer_pid,
        template_fingerprint: [44; 32],
        batch_id,
        proof_ref: RootIssuerRenewalProofRef {
            issuer_pid,
            cert_hash,
        },
        status: PolicyRenewalAttemptStatus::Prepared,
        prepared_at_ns: 10,
        retrieval_expires_at_ns: 70,
        install_deadline_ns: 90,
        prepared_cert_hash: cert_hash,
        prepared_expires_at_ns: 200,
        prepared_refresh_after_ns: 160,
        failure: None,
    }
}

fn renewal_batch(batch_id: [u8; 32], attempt_ids: Vec<[u8; 32]>) -> RootDelegationRenewalBatch {
    RootDelegationRenewalBatch {
        batch_id,
        attempt_ids,
        prepared_at_ns: 10,
        retrieval_expires_at_ns: 70,
    }
}

fn proof_for(
    issuer_pid: Principal,
    cert_hash: [u8; 32],
    expires_at_ns: u64,
) -> RootDelegationProofBatchProof {
    RootDelegationProofBatchProof {
        issuer_pid,
        cert_hash,
        proof: DelegationProof {
            cert: DelegationCert {
                root_pid: p(1),
                issuer_pid,
                issuer_proof_alg: IssuerProofAlgorithm::IcCanisterSignatureV1,
                issuer_proof_binding_hash: [4; 32],
                issuer_proof_binding: IssuerProofBinding::IcCanisterSignatureV1 {
                    seed_hash: [5; 32],
                },
                issued_at_ns: 10,
                not_before_ns: 10,
                expires_at_ns,
                max_token_ttl_ns: 30,
                aud: DelegationAudience::Project("test".to_string()),
                grants: vec![grant("canic.issue")],
            },
            root_proof: RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
                signature_cbor: vec![8; 64],
                public_key_der: vec![9; 32],
            }),
        },
    }
}

fn schedule_install_attempt(
    issuer_pid: Principal,
    batch_id: [u8; 32],
    attempt_id: [u8; 32],
    cert_hash: [u8; 32],
) -> RootIssuerRenewalAttempt {
    let template = root_issuer_renewal_template_from_request(upsert_request(issuer_pid));
    AuthStateOps::upsert_root_issuer_renewal_template(template.clone());

    let mut attempt = renewal_attempt(attempt_id, batch_id, issuer_pid, cert_hash);
    attempt.template_fingerprint = renewal_template_fingerprint(&template);
    AuthStateOps::upsert_root_issuer_renewal_attempt(attempt.clone());
    AuthStateOps::upsert_root_delegation_renewal_batch(renewal_batch(batch_id, vec![attempt_id]));
    AuthStateOps::upsert_root_issuer_renewal_state(RootIssuerRenewalState {
        issuer_pid,
        template_fingerprint: attempt.template_fingerprint,
        last_installed_cert_hash: None,
        last_installed_expires_at_ns: None,
        last_installed_refresh_after_ns: None,
        active_attempt_id: Some(attempt_id),
        last_outcome: PolicyRenewalOutcome::NeverRun,
        consecutive_failures: 0,
        next_attempt_after_ns: 0,
        updated_at_ns: 10,
    });

    attempt
}

fn renewal_attempt_metric_count(
    outcome: DelegatedAuthMetricOutcome,
    reason: DelegatedAuthMetricReason,
) -> u64 {
    let key = DelegatedAuthMetricKey {
        operation: DelegatedAuthMetricOperation::RenewalAttempt,
        outcome,
        reason,
    };
    DelegatedAuthMetrics::event_snapshot()
        .into_iter()
        .find_map(|(event_key, count)| (event_key == key).then_some(count))
        .unwrap_or(0)
}

fn fake_prepare_response(
    request: RootDelegationProofBatchPrepareRequest,
) -> RootDelegationProofBatchPrepareResponse {
    let batch_id = request
        .metadata
        .expect("renewal prepare must include replay metadata")
        .request_id;
    RootDelegationProofBatchPrepareResponse {
        batch_id,
        retrieval_expires_at_ns: 70,
        entries: request
            .entries
            .into_iter()
            .enumerate()
            .map(|(idx, entry)| RootDelegationProofBatchEntry {
                issuer_pid: entry.issuer_pid,
                cert_hash: [u8::try_from(idx + 1).expect("small test index"); 32],
                expires_at_ns: 200,
                refresh_after_ns: 160,
            })
            .collect(),
    }
}

#[test]
fn upsert_root_issuer_renewal_template_accepts_registered_policy() {
    let issuer_pid = p(81);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer_pid));

    let response = upsert_root_issuer_renewal_template(upsert_request(issuer_pid), 10)
        .expect("template should be accepted");

    assert_eq!(response.template.issuer_pid, issuer_pid);
    assert_eq!(response.template.grants, vec![grant("canic.issue")]);
    assert_eq!(
        root_issuer_renewal_status(RootIssuerRenewalStatusRequest { issuer_pid }).template,
        Some(response.template)
    );
}

#[test]
fn upsert_root_issuer_renewal_template_rejects_policy_widening() {
    let issuer_pid = p(82);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer_pid));
    let mut request = upsert_request(issuer_pid);
    request.grants = vec![grant("canic.admin")];

    assert!(upsert_root_issuer_renewal_template(request, 10).is_err());
}

#[test]
fn disabled_root_issuer_renewal_template_can_be_staged_without_policy() {
    let issuer_pid = p(83);
    let mut request = upsert_request(issuer_pid);
    request.enabled = false;
    request.grants.clear();

    let response = upsert_root_issuer_renewal_template(request, 10)
        .expect("disabled template should not require an issuer policy");

    assert!(!response.template.enabled);
    assert_eq!(
        root_issuer_renewal_status(RootIssuerRenewalStatusRequest { issuer_pid }).template,
        Some(response.template)
    );
}

#[test]
fn disabling_root_issuer_renewal_template_clears_active_attempt() {
    let issuer_pid = p(84);
    let batch_id = [84; 32];
    let attempt_id = [85; 32];
    AuthStateOps::upsert_root_issuer_policy(policy(issuer_pid));
    let active_attempt = schedule_install_attempt(issuer_pid, batch_id, attempt_id, [86; 32]);
    let mut request = upsert_request(issuer_pid);
    request.enabled = false;

    let response = upsert_root_issuer_renewal_template(request, 90)
        .expect("disabled template should be accepted");

    assert!(!response.template.enabled);
    let attempt = AuthStateOps::root_issuer_renewal_attempt(attempt_id)
        .expect("disabled attempt should remain observable");
    assert_eq!(attempt.status, PolicyRenewalAttemptStatus::Disabled);
    assert_eq!(
        attempt.failure,
        Some(PolicyRenewalOutcome::TemplateDisabled)
    );

    let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
        .expect("issuer renewal state should remain observable");
    assert_eq!(state.active_attempt_id, None);
    assert_eq!(state.last_outcome, PolicyRenewalOutcome::TemplateDisabled);
    assert_eq!(state.consecutive_failures, 0);
    assert_eq!(state.next_attempt_after_ns, 90);
    assert_eq!(state.updated_at_ns, 90);
    assert_ne!(
        state.template_fingerprint,
        active_attempt.template_fingerprint
    );
}

#[test]
fn manual_delegation_install_success_updates_matching_renewal_state() {
    let issuer_pid = p(230);
    let cert_hash = [231; 32];
    AuthStateOps::upsert_root_issuer_policy(policy(issuer_pid));
    upsert_root_issuer_renewal_template(upsert_request(issuer_pid), 10)
        .expect("template should be accepted");
    let proof = proof_for(issuer_pid, cert_hash, 60_000_000_010);

    record_manual_delegation_renewal_install_outcome(
        &proof,
        RootDelegationProofInstallOutcome::Installed,
        20,
    );

    let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
        .expect("manual install should record renewal state");
    assert_eq!(state.last_installed_cert_hash, Some(cert_hash));
    assert_eq!(state.last_installed_expires_at_ns, Some(60_000_000_010));
    assert_eq!(state.last_installed_refresh_after_ns, Some(48_000_000_010));
    assert_eq!(state.active_attempt_id, None);
    assert_eq!(state.last_outcome, PolicyRenewalOutcome::Installed);
    assert_eq!(state.consecutive_failures, 0);
    assert_eq!(state.next_attempt_after_ns, 48_000_000_010);
    assert_eq!(state.updated_at_ns, 20);
}

#[test]
fn root_issuer_renewal_status_reports_root_owned_state() {
    let issuer_pid = p(87);
    let state = RootIssuerRenewalState {
        issuer_pid,
        template_fingerprint: [1; 32],
        last_installed_cert_hash: Some([2; 32]),
        last_installed_expires_at_ns: Some(200),
        last_installed_refresh_after_ns: Some(160),
        active_attempt_id: Some([3; 32]),
        last_outcome: PolicyRenewalOutcome::RetrievalExpired,
        consecutive_failures: 2,
        next_attempt_after_ns: 90,
        updated_at_ns: 80,
    };
    AuthStateOps::upsert_root_issuer_renewal_attempt(renewal_attempt(
        [3; 32], [4; 32], issuer_pid, [5; 32],
    ));
    AuthStateOps::upsert_root_issuer_renewal_state(state);

    let status = root_issuer_renewal_status(RootIssuerRenewalStatusRequest { issuer_pid });

    assert_eq!(status.template, None);
    assert_eq!(
        status
            .state
            .as_ref()
            .map(|state| state.last_outcome.clone()),
        Some(RootIssuerRenewalOutcome::RetrievalExpired)
    );
    assert_eq!(
        status
            .active_attempt
            .as_ref()
            .map(|attempt| attempt.batch_id),
        Some([4; 32])
    );
}

#[test]
fn renewal_proof_batch_get_uses_scheduled_refs_only() {
    let batch_id = [90; 32];
    let first_attempt_id = [91; 32];
    let second_attempt_id = [92; 32];
    let first_issuer = p(91);
    let second_issuer = p(92);
    AuthStateOps::upsert_root_issuer_renewal_attempt(renewal_attempt(
        first_attempt_id,
        batch_id,
        first_issuer,
        [93; 32],
    ));
    AuthStateOps::upsert_root_issuer_renewal_attempt(renewal_attempt(
        second_attempt_id,
        batch_id,
        second_issuer,
        [94; 32],
    ));
    AuthStateOps::upsert_root_delegation_renewal_batch(renewal_batch(
        batch_id,
        vec![first_attempt_id, second_attempt_id],
    ));

    let response = get_delegation_renewal_proof_batch_with_getter(
        RootDelegationRenewalProofBatchGetRequest { batch_id },
        20,
        |request| {
            assert_eq!(request.batch_id, batch_id);
            assert_eq!(
                request.entries,
                vec![
                    RootDelegationProofBatchProofRef {
                        issuer_pid: first_issuer,
                        cert_hash: [93; 32],
                    },
                    RootDelegationProofBatchProofRef {
                        issuer_pid: second_issuer,
                        cert_hash: [94; 32],
                    },
                ]
            );
            Ok(RootDelegationProofBatchGetResponse {
                batch_id,
                proofs: Vec::new(),
            })
        },
    )
    .expect("scheduled renewal batch should retrieve through resolved refs");

    assert_eq!(response.batch_id, batch_id);
}

#[test]
fn renewal_proof_batch_get_rejects_expired_or_nonprepared_attempts() {
    let batch_id = [95; 32];
    let attempt_id = [96; 32];
    let mut attempt = renewal_attempt(attempt_id, batch_id, p(95), [97; 32]);
    attempt.status = PolicyRenewalAttemptStatus::Installing;
    AuthStateOps::upsert_root_issuer_renewal_attempt(attempt);
    AuthStateOps::upsert_root_delegation_renewal_batch(renewal_batch(batch_id, vec![attempt_id]));

    let err = get_delegation_renewal_proof_batch_with_getter(
        RootDelegationRenewalProofBatchGetRequest { batch_id },
        20,
        |_| panic!("nonprepared attempt must not call generic proof retrieval"),
    )
    .expect_err("nonprepared scheduled attempt should reject");
    assert!(err.to_string().contains("not prepared"));

    let expired_batch_id = [98; 32];
    let expired_attempt_id = [99; 32];
    AuthStateOps::upsert_root_issuer_renewal_attempt(renewal_attempt(
        expired_attempt_id,
        expired_batch_id,
        p(96),
        [100; 32],
    ));
    AuthStateOps::upsert_root_delegation_renewal_batch(renewal_batch(
        expired_batch_id,
        vec![expired_attempt_id],
    ));

    let err = get_delegation_renewal_proof_batch_with_getter(
        RootDelegationRenewalProofBatchGetRequest {
            batch_id: expired_batch_id,
        },
        70,
        |_| panic!("expired batch must not call generic proof retrieval"),
    )
    .expect_err("expired scheduled batch should reject");
    assert!(err.to_string().contains("expired"));
}

#[test]
fn renewal_batch_install_gate_accepts_live_scheduled_work() {
    let issuer_pid = p(144);
    let batch_id = [144; 32];
    let attempt_id = [145; 32];
    schedule_install_attempt(issuer_pid, batch_id, attempt_id, [146; 32]);

    ensure_delegation_renewal_batch_scheduled(batch_id, 30)
        .expect("live scheduled renewal work should pass provisioner gate");

    assert_eq!(
        AuthStateOps::root_issuer_renewal_attempt(attempt_id)
            .expect("attempt should remain stored")
            .status,
        PolicyRenewalAttemptStatus::Prepared
    );
    assert_eq!(
        AuthStateOps::root_issuer_renewal_state(issuer_pid)
            .expect("state should remain stored")
            .active_attempt_id,
        Some(attempt_id)
    );
}

#[test]
fn renewal_batch_install_gate_expires_late_scheduled_work() {
    let issuer_pid = p(147);
    let batch_id = [147; 32];
    let attempt_id = [148; 32];
    schedule_install_attempt(issuer_pid, batch_id, attempt_id, [149; 32]);

    let mut attempt =
        AuthStateOps::root_issuer_renewal_attempt(attempt_id).expect("attempt should be scheduled");
    attempt.retrieval_expires_at_ns = 40;
    attempt.install_deadline_ns = 40;
    AuthStateOps::upsert_root_issuer_renewal_attempt(attempt);
    let mut batch =
        AuthStateOps::root_delegation_renewal_batch(batch_id).expect("batch should be stored");
    batch.retrieval_expires_at_ns = 40;
    AuthStateOps::upsert_root_delegation_renewal_batch(batch);

    let err = ensure_delegation_renewal_batch_scheduled(batch_id, 40)
        .expect_err("expired scheduled work should fail provisioner gate");

    assert!(err.to_string().contains("install deadline expired"));
    let attempt = AuthStateOps::root_issuer_renewal_attempt(attempt_id)
        .expect("attempt should remain visible");
    assert_eq!(attempt.status, PolicyRenewalAttemptStatus::Expired);
    assert_eq!(
        attempt.failure,
        Some(PolicyRenewalOutcome::InstallDeadlineExpired)
    );

    let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
        .expect("issuer renewal state should remain visible");
    assert_eq!(state.active_attempt_id, None);
    assert_eq!(
        state.last_outcome,
        PolicyRenewalOutcome::InstallDeadlineExpired
    );
    assert_eq!(state.consecutive_failures, 1);
    assert_eq!(state.next_attempt_after_ns, 40);
    assert_eq!(AuthStateOps::root_delegation_renewal_batch(batch_id), None);
}

#[test]
fn delegation_renewal_work_lists_retrievable_batches_only() {
    let valid_batch_id = [210; 32];
    let valid_attempt_id = [211; 32];
    let skipped_batch_id = [212; 32];
    let skipped_attempt_id = [213; 32];
    let expired_batch_id = [214; 32];
    let expired_attempt_id = [215; 32];

    AuthStateOps::upsert_root_issuer_renewal_attempt(renewal_attempt(
        valid_attempt_id,
        valid_batch_id,
        p(210),
        [216; 32],
    ));
    AuthStateOps::upsert_root_delegation_renewal_batch(renewal_batch(
        valid_batch_id,
        vec![valid_attempt_id],
    ));

    let mut skipped_attempt =
        renewal_attempt(skipped_attempt_id, skipped_batch_id, p(211), [217; 32]);
    skipped_attempt.status = PolicyRenewalAttemptStatus::Installing;
    AuthStateOps::upsert_root_issuer_renewal_attempt(skipped_attempt);
    AuthStateOps::upsert_root_delegation_renewal_batch(renewal_batch(
        skipped_batch_id,
        vec![skipped_attempt_id],
    ));

    AuthStateOps::upsert_root_issuer_renewal_attempt(renewal_attempt(
        expired_attempt_id,
        expired_batch_id,
        p(212),
        [218; 32],
    ));
    let mut expired_batch = renewal_batch(expired_batch_id, vec![expired_attempt_id]);
    expired_batch.retrieval_expires_at_ns = 15;
    AuthStateOps::upsert_root_delegation_renewal_batch(expired_batch);

    let work = delegation_renewal_work(20);

    let valid_batch = work
        .batches
        .iter()
        .find(|batch| batch.batch_id == valid_batch_id)
        .expect("valid scheduled batch should be advertised");
    assert_eq!(valid_batch.attempt_count, 1);
    assert_eq!(valid_batch.attempts.len(), 1);
    assert_eq!(valid_batch.attempts[0].attempt_id, valid_attempt_id);
    assert_eq!(
        valid_batch.attempts[0].status,
        RootIssuerRenewalAttemptStatus::Prepared
    );
    assert!(
        work.batches
            .iter()
            .all(|batch| batch.batch_id != skipped_batch_id)
    );
    assert!(
        work.batches
            .iter()
            .all(|batch| batch.batch_id != expired_batch_id)
    );
}

#[test]
fn prepare_due_delegation_renewals_schedules_initial_enabled_templates() {
    DelegatedAuthMetrics::reset();

    let first_issuer = p(101);
    let second_issuer = p(100);
    AuthStateOps::upsert_root_issuer_policy(policy(first_issuer));
    AuthStateOps::upsert_root_issuer_policy(policy(second_issuer));
    upsert_root_issuer_renewal_template(upsert_request(first_issuer), 10)
        .expect("first template should be accepted");
    upsert_root_issuer_renewal_template(upsert_request(second_issuer), 10)
        .expect("second template should be accepted");

    let result = prepare_due_delegation_renewals_with_prepare(120_000_000_000, 10, |request| {
        assert!(
            request
                .entries
                .iter()
                .any(|entry| entry.issuer_pid == first_issuer)
        );
        assert!(
            request
                .entries
                .iter()
                .any(|entry| entry.issuer_pid == second_issuer)
        );
        Ok(fake_prepare_response(request))
    })
    .expect("due renewal templates should prepare");

    let batch_id = result
        .prepared_batch_id
        .expect("initial enabled templates should create a batch");
    let batch = AuthStateOps::root_delegation_renewal_batch(batch_id)
        .expect("scheduler should persist renewal batch");

    assert!(result.prepared_attempts >= 2);
    assert_eq!(
        renewal_attempt_metric_count(
            DelegatedAuthMetricOutcome::Started,
            DelegatedAuthMetricReason::Ok,
        ),
        u64::try_from(result.prepared_attempts).expect("prepared attempt count should fit u64")
    );
    assert!(batch.attempt_ids.len() >= 2);
    for issuer_pid in [first_issuer, second_issuer] {
        let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
            .expect("scheduler should update issuer renewal state");
        let attempt = AuthStateOps::root_issuer_renewal_attempt(
            state
                .active_attempt_id
                .expect("scheduler should set active attempt id"),
        )
        .expect("scheduler should persist active attempt");
        assert_eq!(attempt.issuer_pid, issuer_pid);
        assert_eq!(attempt.status, PolicyRenewalAttemptStatus::Prepared);
    }
}

#[test]
fn prepare_due_delegation_renewals_records_quota_prepare_failure() {
    let issuer_pid = p(141);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer_pid));
    upsert_root_issuer_renewal_template(upsert_request(issuer_pid), 10)
        .expect("template should be accepted");
    let template = root_issuer_renewal_template_from_request(upsert_request(issuer_pid));
    AuthStateOps::upsert_root_issuer_renewal_state(RootIssuerRenewalState {
        issuer_pid,
        template_fingerprint: renewal_template_fingerprint(&template),
        last_installed_cert_hash: Some([142; 32]),
        last_installed_expires_at_ns: Some(1_000),
        last_installed_refresh_after_ns: Some(15),
        active_attempt_id: None,
        last_outcome: PolicyRenewalOutcome::Installed,
        consecutive_failures: 2,
        next_attempt_after_ns: 0,
        updated_at_ns: 12,
    });

    let err = prepare_due_delegation_renewals_with_prepare(120_000_000_000, 20, |_| {
        Err(InternalError::resource_exhausted(
            "pending renewal quota exhausted",
        ))
    })
    .expect_err("prepare quota failure should propagate");

    assert_eq!(
        err.public_error().map(|public| public.code),
        Some(ErrorCode::ResourceExhausted)
    );
    let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
        .expect("quota failure should update issuer renewal state");
    assert_eq!(state.active_attempt_id, None);
    assert_eq!(state.last_installed_cert_hash, Some([142; 32]));
    assert_eq!(state.last_installed_expires_at_ns, Some(1_000));
    assert_eq!(state.last_installed_refresh_after_ns, Some(15));
    assert_eq!(state.last_outcome, PolicyRenewalOutcome::QuotaExceeded);
    assert_eq!(state.consecutive_failures, 3);
    assert_eq!(state.next_attempt_after_ns, 60_000_000_020);
    assert_eq!(state.updated_at_ns, 20);
}

#[test]
fn prepare_due_delegation_renewals_records_policy_prepare_failure() {
    let issuer_pid = p(143);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer_pid));
    upsert_root_issuer_renewal_template(upsert_request(issuer_pid), 10)
        .expect("template should be accepted");

    let err = prepare_due_delegation_renewals_with_prepare(120_000_000_000, 30, |_| {
        Err(InternalError::forbidden(
            "root issuer policy rejected renewal",
        ))
    })
    .expect_err("policy failure should propagate");

    assert_eq!(
        err.public_error().map(|public| public.code),
        Some(ErrorCode::Forbidden)
    );
    let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
        .expect("policy failure should update issuer renewal state");
    assert_eq!(state.active_attempt_id, None);
    assert_eq!(state.last_outcome, PolicyRenewalOutcome::PolicyRejected);
    assert_eq!(state.consecutive_failures, 1);
    assert_eq!(state.next_attempt_after_ns, 60_000_000_030);
    assert_eq!(state.updated_at_ns, 30);
}

#[test]
fn prepare_due_delegation_renewals_skips_fresh_or_active_attempts() {
    let fresh_issuer = p(102);
    let active_issuer = p(103);
    let fresh_template = root_issuer_renewal_template_from_request(upsert_request(fresh_issuer));
    let active_template = root_issuer_renewal_template_from_request(upsert_request(active_issuer));

    let fresh_state = RootIssuerRenewalState {
        issuer_pid: fresh_issuer,
        template_fingerprint: renewal_template_fingerprint(&fresh_template),
        last_installed_cert_hash: Some([1; 32]),
        last_installed_expires_at_ns: Some(1_000),
        last_installed_refresh_after_ns: Some(900),
        active_attempt_id: None,
        last_outcome: PolicyRenewalOutcome::Installed,
        consecutive_failures: 0,
        next_attempt_after_ns: 0,
        updated_at_ns: 20,
    };

    let active_attempt_id = [104; 32];
    let mut active_attempt =
        renewal_attempt(active_attempt_id, [105; 32], active_issuer, [106; 32]);
    active_attempt.install_deadline_ns = 200;
    AuthStateOps::upsert_root_issuer_renewal_attempt(active_attempt);
    let active_state = RootIssuerRenewalState {
        issuer_pid: active_issuer,
        template_fingerprint: renewal_template_fingerprint(&active_template),
        last_installed_cert_hash: Some([2; 32]),
        last_installed_expires_at_ns: Some(200),
        last_installed_refresh_after_ns: Some(100),
        active_attempt_id: Some(active_attempt_id),
        last_outcome: PolicyRenewalOutcome::Installed,
        consecutive_failures: 0,
        next_attempt_after_ns: 0,
        updated_at_ns: 20,
    };

    assert!(!renewal_template_due(
        30,
        renewal_template_fingerprint(&fresh_template),
        Some(&fresh_state)
    ));
    assert!(!renewal_template_due(
        150,
        renewal_template_fingerprint(&active_template),
        Some(&active_state)
    ));
}

#[test]
fn prepare_due_delegation_renewals_expires_stale_active_attempts() {
    let issuer_pid = p(136);
    let stale_attempt_id = [137; 32];
    let stale_batch_id = [138; 32];
    let stale_cert_hash = [139; 32];
    AuthStateOps::upsert_root_issuer_policy(policy(issuer_pid));
    upsert_root_issuer_renewal_template(upsert_request(issuer_pid), 10)
        .expect("template should be accepted");
    let template = root_issuer_renewal_template_from_request(upsert_request(issuer_pid));

    let mut stale_attempt = renewal_attempt(
        stale_attempt_id,
        stale_batch_id,
        issuer_pid,
        stale_cert_hash,
    );
    stale_attempt.template_fingerprint = renewal_template_fingerprint(&template);
    stale_attempt.retrieval_expires_at_ns = 40;
    stale_attempt.install_deadline_ns = 40;
    AuthStateOps::upsert_root_issuer_renewal_attempt(stale_attempt);
    AuthStateOps::upsert_root_delegation_renewal_batch(renewal_batch(
        stale_batch_id,
        vec![stale_attempt_id],
    ));
    AuthStateOps::upsert_root_issuer_renewal_state(RootIssuerRenewalState {
        issuer_pid,
        template_fingerprint: renewal_template_fingerprint(&template),
        last_installed_cert_hash: Some([140; 32]),
        last_installed_expires_at_ns: Some(100),
        last_installed_refresh_after_ns: Some(30),
        active_attempt_id: Some(stale_attempt_id),
        last_outcome: PolicyRenewalOutcome::Installed,
        consecutive_failures: 0,
        next_attempt_after_ns: 0,
        updated_at_ns: 20,
    });

    let result = prepare_due_delegation_renewals_with_prepare(120_000_000_000, 80, |request| {
        assert!(
            request
                .entries
                .iter()
                .any(|entry| entry.issuer_pid == issuer_pid)
        );
        Ok(fake_prepare_response(request))
    })
    .expect("expired active attempt should allow fresh renewal prepare");

    assert!(result.prepared_batch_id.is_some());
    let stale_attempt = AuthStateOps::root_issuer_renewal_attempt(stale_attempt_id)
        .expect("stale attempt should remain stored");
    assert_eq!(stale_attempt.status, PolicyRenewalAttemptStatus::Expired);
    assert_eq!(
        stale_attempt.failure,
        Some(PolicyRenewalOutcome::RetrievalExpired)
    );
    assert_eq!(
        AuthStateOps::root_delegation_renewal_batch(stale_batch_id),
        None
    );

    let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
        .expect("issuer state should remain stored");
    assert_ne!(state.active_attempt_id, Some(stale_attempt_id));
    assert_eq!(state.last_outcome, PolicyRenewalOutcome::RetrievalExpired);
    assert_eq!(state.consecutive_failures, 1);
}

#[test]
fn scheduled_renewal_install_preflight_marks_attempt_installing() {
    let issuer_pid = p(120);
    let batch_id = [121; 32];
    let attempt_id = [122; 32];
    let cert_hash = [123; 32];
    schedule_install_attempt(issuer_pid, batch_id, attempt_id, cert_hash);

    let scheduled_attempt_id = preflight_delegation_renewal_proof_install(
        batch_id,
        &proof_for(issuer_pid, cert_hash, 200),
        30,
    )
    .expect("scheduled renewal install should preflight");

    assert_eq!(scheduled_attempt_id, Some(attempt_id));
    assert_eq!(
        AuthStateOps::root_issuer_renewal_attempt(attempt_id)
            .expect("attempt should remain stored")
            .status,
        PolicyRenewalAttemptStatus::Installing
    );
    assert_eq!(
        AuthStateOps::root_issuer_renewal_state(issuer_pid)
            .expect("state should remain stored")
            .active_attempt_id,
        Some(attempt_id)
    );
}

#[test]
fn scheduled_renewal_install_success_updates_issuer_state() {
    let issuer_pid = p(124);
    let batch_id = [125; 32];
    let attempt_id = [126; 32];
    let cert_hash = [127; 32];
    let mut attempt = schedule_install_attempt(issuer_pid, batch_id, attempt_id, cert_hash);
    attempt.status = PolicyRenewalAttemptStatus::Installing;
    AuthStateOps::upsert_root_issuer_renewal_attempt(attempt);

    record_delegation_renewal_install_outcome(
        attempt_id,
        RootDelegationProofInstallOutcome::Installed,
        40,
    );

    assert_eq!(
        AuthStateOps::root_issuer_renewal_attempt(attempt_id)
            .expect("attempt should remain stored")
            .status,
        PolicyRenewalAttemptStatus::Installed
    );
    let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
        .expect("issuer renewal state should update");
    assert_eq!(state.active_attempt_id, None);
    assert_eq!(state.last_installed_cert_hash, Some(cert_hash));
    assert_eq!(state.last_installed_expires_at_ns, Some(200));
    assert_eq!(state.last_installed_refresh_after_ns, Some(160));
    assert_eq!(state.last_outcome, PolicyRenewalOutcome::Installed);
    assert_eq!(state.consecutive_failures, 0);
}

#[test]
fn scheduled_renewal_install_call_failure_remains_retryable() {
    DelegatedAuthMetrics::reset();

    let issuer_pid = p(128);
    let batch_id = [129; 32];
    let attempt_id = [130; 32];
    let cert_hash = [131; 32];
    let mut attempt = schedule_install_attempt(issuer_pid, batch_id, attempt_id, cert_hash);
    attempt.status = PolicyRenewalAttemptStatus::Installing;
    AuthStateOps::upsert_root_issuer_renewal_attempt(attempt);

    record_delegation_renewal_install_outcome(
        attempt_id,
        RootDelegationProofInstallOutcome::CallFailed,
        40,
    );

    let attempt = AuthStateOps::root_issuer_renewal_attempt(attempt_id)
        .expect("attempt should remain stored");
    assert_eq!(attempt.status, PolicyRenewalAttemptStatus::FailedRetryable);
    assert_eq!(
        attempt.failure,
        Some(PolicyRenewalOutcome::IssuerCallFailed)
    );

    let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
        .expect("issuer renewal state should update");
    assert_eq!(state.active_attempt_id, Some(attempt_id));
    assert_eq!(state.last_outcome, PolicyRenewalOutcome::IssuerCallFailed);
    assert_eq!(state.consecutive_failures, 1);
    assert_eq!(state.next_attempt_after_ns, 90);
    assert_eq!(
        renewal_attempt_metric_count(
            DelegatedAuthMetricOutcome::Failed,
            DelegatedAuthMetricReason::RetryScheduled,
        ),
        1
    );
}

#[test]
fn scheduled_renewal_install_rejects_changed_template() {
    let issuer_pid = p(132);
    let batch_id = [133; 32];
    let attempt_id = [134; 32];
    let cert_hash = [135; 32];
    schedule_install_attempt(issuer_pid, batch_id, attempt_id, cert_hash);

    let mut changed_template =
        root_issuer_renewal_template_from_request(upsert_request(issuer_pid));
    changed_template.cert_ttl_ns += 1;
    AuthStateOps::upsert_root_issuer_renewal_template(changed_template);

    let outcome = preflight_delegation_renewal_proof_install(
        batch_id,
        &proof_for(issuer_pid, cert_hash, 200),
        30,
    )
    .expect_err("changed template should reject scheduled install");

    assert_eq!(
        outcome,
        RootDelegationProofInstallOutcome::ExpiredOrSuperseded
    );
    let attempt = AuthStateOps::root_issuer_renewal_attempt(attempt_id)
        .expect("attempt should remain stored");
    assert_eq!(attempt.status, PolicyRenewalAttemptStatus::FailedTerminal);
    assert_eq!(attempt.failure, Some(PolicyRenewalOutcome::TemplateChanged));

    let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
        .expect("issuer renewal state should update");
    assert_eq!(state.active_attempt_id, None);
    assert_eq!(state.last_outcome, PolicyRenewalOutcome::TemplateChanged);
}
