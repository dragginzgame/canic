use canic::{
    Error,
    cdk::types::Principal,
    dto::{
        auth::{
            ActiveDelegationProofStatus, ActiveDelegationProofStatusResponse, AuthRequestMetadata,
            DelegatedTokenPrepareRequest, DelegatedTokenPrepareResponse, DelegationAudience,
            RootDelegationProofBatchGetRequest, RootDelegationProofBatchGetResponse,
            RootDelegationProofBatchInstallRequest, RootDelegationProofBatchInstallResponse,
            RootDelegationProofBatchPrepareEntry, RootDelegationProofBatchPrepareRequest,
            RootDelegationProofBatchPrepareResponse, RootDelegationProofBatchProofRef,
            RootDelegationProofInstallOutcome, RootDelegationRenewalBatchView,
            RootDelegationRenewalProofBatchGetRequest, RootDelegationRenewalWorkListResponse,
            RootIssuerPolicyResponse, RootIssuerPolicyUpsertRequest,
            RootIssuerRenewalTemplateResponse, RootIssuerRenewalTemplateUpsertRequest,
        },
        error::ErrorCode,
        placement::sharding::ShardingRegistryResponse,
    },
    ids::{CanisterRole, cap},
    protocol,
};
use canic_testing_internal::canister;
use canic_testing_internal::pic::{
    CanicPicExt, create_user_shard, issue_delegated_token_from_active_proof_with_request_nonce,
    role_grant,
};
use canic_tests::root::{
    RootSetupProfile,
    assertions::assert_registry_parents,
    harness::{RootSetup, setup_cached_root, setup_root},
};
use std::time::Duration;

const INERT_WASM: &[u8] = b"\0asm\x01\0\0\0";
const INSTALL_CODE_COOLDOWN: Duration = Duration::from_mins(5);
const SECOND_NS: u64 = 1_000_000_000;

#[test]
fn user_hub_sharding_profile_prewarms_first_user_shard() {
    let setup = setup_cached_root(RootSetupProfile::Sharding);

    assert!(
        !setup.subnet_index.contains_key(&canister::APP),
        "sharding profile should not boot app",
    );
    assert!(
        !setup.subnet_index.contains_key(&canister::SCALE_HUB),
        "sharding profile should not boot scale_hub",
    );

    let user_hub_pid = user_hub_pid(&setup);

    let registry: Result<Result<ShardingRegistryResponse, Error>, _> =
        setup
            .pic
            .query_call_as(user_hub_pid, setup.root_id, "canic_sharding_registry", ());
    let registry = registry
        .expect("registry query transport failed")
        .expect("registry query application failed");
    let startup_shard_pid = registry
        .0
        .into_iter()
        .find(|entry| entry.entry.pool == "user_shards")
        .map(|entry| entry.pid)
        .expect("startup user shard must exist before first account create");

    let created: Result<Result<Principal, Error>, _> = setup.pic.update_call(
        user_hub_pid,
        "create_account",
        (Principal::from_slice(&[7; 29]),),
    );
    let shard_pid = created
        .expect("create_account transport failed")
        .expect("create_account application failed");
    assert_eq!(shard_pid, startup_shard_pid);
    setup
        .pic
        .wait_for_ready(shard_pid, 50, "user shard bootstrap");

    assert_registry_parents(
        &setup.pic,
        setup.root_id,
        &[
            (CanisterRole::ROOT, None),
            (canister::USER_HUB, Some(setup.root_id)),
            (canister::TEST, Some(setup.root_id)),
            (canister::USER_SHARD, Some(user_hub_pid)),
        ],
    );
}

#[test]
fn root_batch_provisioning_installs_active_proof_on_user_shard() {
    let setup = setup_cached_root(RootSetupProfile::Sharding);

    let verifier_pid = test_verifier_pid(&setup);
    let subject = test_subject(56);
    let shard_pid = create_user_shard_for_subject(&setup, subject);

    let (prepared, install_request) = install_root_batch_delegation_proof(&setup, shard_pid);
    let status = assert_active_delegation_proof_status(&setup, shard_pid, &prepared);
    verify_issuer_local_delegated_token(&setup, verifier_pid, shard_pid, subject, &status);
    assert_repeated_batch_install_is_idempotent(&setup, install_request);
    assert_active_delegation_proof_refresh_and_expiry(&setup, shard_pid, subject, &status);
}

#[test]
fn root_scheduled_renewal_bridge_refreshes_user_shard_active_proof() {
    let setup = setup_root(RootSetupProfile::Sharding);

    let verifier_pid = test_verifier_pid(&setup);
    let subject = test_subject(61);
    let shard_pid = create_user_shard_for_subject(&setup, subject);

    upsert_delegation_issuer(&setup, shard_pid);
    upsert_delegation_renewal_template(&setup, shard_pid);

    advance_pic_to_ns(
        &setup,
        setup
            .pic
            .current_time_nanos()
            .saturating_add(31 * SECOND_NS),
    );
    let first_work = scheduled_renewal_work_for_issuer(&setup, shard_pid);
    let first_status = bridge_scheduled_renewal_batch(&setup, first_work, shard_pid);
    verify_issuer_local_delegated_token(&setup, verifier_pid, shard_pid, subject, &first_status);

    advance_pic_to_ns(
        &setup,
        first_status
            .refresh_after_ns
            .expect("scheduled proof should expose refresh threshold"),
    );
    let refresh_work = scheduled_renewal_work_for_issuer(&setup, shard_pid);
    let refresh_status = bridge_scheduled_renewal_batch(&setup, refresh_work, shard_pid);
    assert_ne!(refresh_status.cert_hash, first_status.cert_hash);
    verify_issuer_local_delegated_token(&setup, verifier_pid, shard_pid, subject, &refresh_status);
}

#[test]
fn root_batch_install_reports_partial_failure_and_retry() {
    let setup = setup_cached_root(RootSetupProfile::Sharding);

    let subject = test_subject(58);
    let shard_pid = create_user_shard_for_subject(&setup, subject);
    let missing_issuer_pid = Principal::from_slice(&[159; 29]);

    upsert_delegation_issuer(&setup, shard_pid);
    upsert_delegation_issuer(&setup, missing_issuer_pid);

    let request = RootDelegationProofBatchPrepareRequest {
        metadata: Some(batch_metadata(58, shard_pid)),
        entries: vec![
            batch_prepare_entry(shard_pid),
            batch_prepare_entry(missing_issuer_pid),
        ],
    };
    let prepared = prepare_root_delegation_proof_batch(&setup, request);
    let retrieved = retrieve_root_delegation_proof_batch(&setup, &prepared);
    let install_request = RootDelegationProofBatchInstallRequest {
        batch_id: retrieved.batch_id,
        proofs: retrieved.proofs,
    };

    let installed = install_root_delegation_proof_batch(&setup, install_request.clone());
    assert_install_outcome(
        &installed,
        shard_pid,
        RootDelegationProofInstallOutcome::Installed,
    );
    assert_install_outcome(
        &installed,
        missing_issuer_pid,
        RootDelegationProofInstallOutcome::CallFailed,
    );

    let repeated = install_root_delegation_proof_batch(&setup, install_request);
    assert_install_outcome(
        &repeated,
        shard_pid,
        RootDelegationProofInstallOutcome::AlreadyInstalled,
    );
    assert_install_outcome(
        &repeated,
        missing_issuer_pid,
        RootDelegationProofInstallOutcome::CallFailed,
    );
}

#[test]
fn root_unavailable_after_batch_install_does_not_break_issuer_local_issuance() {
    let setup = setup_cached_root(RootSetupProfile::Sharding);

    let verifier_pid = test_verifier_pid(&setup);
    let subject = test_subject(60);
    let shard_pid = create_user_shard_for_subject(&setup, subject);

    upsert_delegation_issuer(&setup, shard_pid);

    let request = RootDelegationProofBatchPrepareRequest {
        metadata: Some(batch_metadata(60, shard_pid)),
        entries: vec![batch_prepare_entry_with_ttl(shard_pid, 600_000_000_000)],
    };
    let prepared = prepare_root_delegation_proof_batch(&setup, request);
    let retrieved = retrieve_root_delegation_proof_batch(&setup, &prepared);
    let installed = install_root_delegation_proof_batch(
        &setup,
        RootDelegationProofBatchInstallRequest {
            batch_id: retrieved.batch_id,
            proofs: retrieved.proofs,
        },
    );
    assert_install_outcome(
        &installed,
        shard_pid,
        RootDelegationProofInstallOutcome::Installed,
    );
    let status = assert_active_delegation_proof_status(&setup, shard_pid, &prepared);

    setup
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    setup
        .pic
        .reinstall_canister(setup.root_id, INERT_WASM.to_vec(), Vec::new(), None)
        .expect("root canister must be replaceable with inert wasm");
    setup.pic.tick();
    let root_ready = setup
        .pic
        .query_call::<bool, _>(setup.root_id, protocol::CANIC_READY, ());
    assert!(
        root_ready.is_err(),
        "root should not expose Canic endpoints after inert reinstall"
    );

    verify_issuer_local_delegated_token(&setup, verifier_pid, shard_pid, subject, &status);
}

fn user_hub_pid(setup: &RootSetup) -> Principal {
    sharding_profile_pid(setup, &canister::USER_HUB, "user_hub")
}

fn test_verifier_pid(setup: &RootSetup) -> Principal {
    sharding_profile_pid(setup, &canister::TEST, "test verifier")
}

fn sharding_profile_pid(setup: &RootSetup, role: &CanisterRole, label: &str) -> Principal {
    setup
        .subnet_index
        .get(role)
        .copied()
        .unwrap_or_else(|| panic!("{label} must exist in sharding profile"))
}

const fn test_subject(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn create_user_shard_for_subject(setup: &RootSetup, subject: Principal) -> Principal {
    create_user_shard(&setup.pic, user_hub_pid(setup), subject)
}

fn install_root_batch_delegation_proof(
    setup: &RootSetup,
    shard_pid: Principal,
) -> (
    RootDelegationProofBatchPrepareResponse,
    RootDelegationProofBatchInstallRequest,
) {
    upsert_delegation_issuer(setup, shard_pid);

    let request = RootDelegationProofBatchPrepareRequest {
        metadata: Some(batch_metadata(56, shard_pid)),
        entries: vec![batch_prepare_entry(shard_pid)],
    };
    let prepared = prepare_root_delegation_proof_batch(setup, request);
    assert_eq!(prepared.entries.len(), 1);

    let retrieved = retrieve_root_delegation_proof_batch(setup, &prepared);
    assert_eq!(retrieved.batch_id, prepared.batch_id);
    assert_eq!(retrieved.proofs.len(), 1);

    let install_request = RootDelegationProofBatchInstallRequest {
        batch_id: retrieved.batch_id,
        proofs: retrieved.proofs,
    };
    let installed = install_root_delegation_proof_batch(setup, install_request.clone());
    assert_eq!(installed.batch_id, prepared.batch_id);
    assert_eq!(installed.outcomes.len(), 1);
    assert_eq!(installed.outcomes[0].issuer_pid, shard_pid);
    assert_eq!(
        installed.outcomes[0].cert_hash,
        prepared.entries[0].cert_hash
    );
    assert_eq!(
        installed.outcomes[0].outcome,
        RootDelegationProofInstallOutcome::Installed
    );

    (prepared, install_request)
}

fn upsert_delegation_issuer(setup: &RootSetup, issuer_pid: Principal) {
    let registered: Result<RootIssuerPolicyResponse, Error> = setup.pic.update_call_or_panic(
        setup.root_id,
        protocol::CANIC_UPSERT_ROOT_ISSUER_POLICY,
        (RootIssuerPolicyUpsertRequest {
            issuer_pid,
            enabled: true,
            allowed_audiences: vec![DelegationAudience::Project("test".to_string())],
            allowed_grants: vec![role_grant(canister::TEST, vec![cap::VERIFY.to_string()])],
            max_cert_ttl_ns: 600_000_000_000,
            refresh_after_ratio_bps: 8_000,
        },),
    );
    let registered = registered.expect("root issuer registration application failed");
    assert_eq!(registered.issuer.issuer_pid, issuer_pid);
    assert!(registered.issuer.enabled);
}

fn upsert_delegation_renewal_template(setup: &RootSetup, issuer_pid: Principal) {
    let response: Result<RootIssuerRenewalTemplateResponse, Error> =
        setup.pic.update_call_or_panic(
            setup.root_id,
            protocol::CANIC_UPSERT_ROOT_ISSUER_RENEWAL_TEMPLATE,
            (RootIssuerRenewalTemplateUpsertRequest {
                issuer_pid,
                enabled: true,
                aud: DelegationAudience::Project("test".to_string()),
                grants: vec![role_grant(canister::TEST, vec![cap::VERIFY.to_string()])],
                cert_ttl_ns: 300_000_000_000,
            },),
        );
    let response = response.expect("renewal template upsert application failed");
    assert_eq!(response.template.issuer_pid, issuer_pid);
    assert!(response.template.enabled);
}

fn scheduled_renewal_work_for_issuer(
    setup: &RootSetup,
    issuer_pid: Principal,
) -> RootDelegationRenewalBatchView {
    for _ in 0..5 {
        let work: Result<RootDelegationRenewalWorkListResponse, Error> = setup
            .pic
            .query_call_or_panic(setup.root_id, protocol::CANIC_DELEGATION_RENEWAL_WORK, ());
        let work = work.expect("renewal work query application failed");
        if let Some(batch) = work.batches.into_iter().find(|batch| {
            batch
                .attempts
                .iter()
                .any(|attempt| attempt.issuer_pid == issuer_pid)
        }) {
            return batch;
        }
        setup.pic.tick();
    }
    panic!("scheduled renewal work was not prepared for issuer {issuer_pid}");
}

fn bridge_scheduled_renewal_batch(
    setup: &RootSetup,
    work: RootDelegationRenewalBatchView,
    issuer_pid: Principal,
) -> ActiveDelegationProofStatusResponse {
    let attempt = work
        .attempts
        .iter()
        .find(|attempt| attempt.issuer_pid == issuer_pid)
        .unwrap_or_else(|| panic!("batch missing attempt for issuer {issuer_pid}"));
    let retrieved: Result<RootDelegationProofBatchGetResponse, Error> =
        setup.pic.query_call_or_panic(
            setup.root_id,
            protocol::CANIC_GET_DELEGATION_RENEWAL_PROOF_BATCH,
            (RootDelegationRenewalProofBatchGetRequest {
                batch_id: work.batch_id,
            },),
        );
    let retrieved = retrieved.expect("scheduled renewal proof get application failed");
    assert_eq!(retrieved.batch_id, work.batch_id);
    let attempt_count =
        usize::try_from(work.attempt_count).expect("renewal attempt count fits usize");
    assert_eq!(retrieved.proofs.len(), attempt_count);

    let installed = install_root_delegation_proof_batch(
        setup,
        RootDelegationProofBatchInstallRequest {
            batch_id: retrieved.batch_id,
            proofs: retrieved.proofs,
        },
    );
    assert_install_outcome(
        &installed,
        issuer_pid,
        RootDelegationProofInstallOutcome::Installed,
    );

    assert_active_delegation_proof_status_for_cert(setup, issuer_pid, attempt.prepared_cert_hash)
}

fn batch_prepare_entry(issuer_pid: Principal) -> RootDelegationProofBatchPrepareEntry {
    batch_prepare_entry_with_ttl(issuer_pid, 60_000_000_000)
}

fn batch_prepare_entry_with_ttl(
    issuer_pid: Principal,
    cert_ttl_ns: u64,
) -> RootDelegationProofBatchPrepareEntry {
    RootDelegationProofBatchPrepareEntry {
        issuer_pid,
        aud: DelegationAudience::Project("test".to_string()),
        grants: vec![role_grant(canister::TEST, vec![cap::VERIFY.to_string()])],
        cert_ttl_ns,
    }
}

fn prepare_root_delegation_proof_batch(
    setup: &RootSetup,
    request: RootDelegationProofBatchPrepareRequest,
) -> RootDelegationProofBatchPrepareResponse {
    let prepared: Result<RootDelegationProofBatchPrepareResponse, Error> =
        setup.pic.update_call_or_panic(
            setup.root_id,
            protocol::CANIC_PREPARE_DELEGATION_PROOF_BATCH,
            (request,),
        );
    prepared.expect("batch prepare application failed")
}

fn retrieve_root_delegation_proof_batch(
    setup: &RootSetup,
    prepared: &RootDelegationProofBatchPrepareResponse,
) -> RootDelegationProofBatchGetResponse {
    let retrieved: Result<RootDelegationProofBatchGetResponse, Error> =
        setup.pic.query_call_or_panic(
            setup.root_id,
            protocol::CANIC_GET_DELEGATION_PROOF_BATCH,
            (RootDelegationProofBatchGetRequest {
                batch_id: prepared.batch_id,
                entries: prepared
                    .entries
                    .iter()
                    .map(|entry| RootDelegationProofBatchProofRef {
                        issuer_pid: entry.issuer_pid,
                        cert_hash: entry.cert_hash,
                    })
                    .collect(),
            },),
        );
    retrieved.expect("batch get application failed")
}

fn install_root_delegation_proof_batch(
    setup: &RootSetup,
    install_request: RootDelegationProofBatchInstallRequest,
) -> RootDelegationProofBatchInstallResponse {
    let installed: Result<RootDelegationProofBatchInstallResponse, Error> =
        setup.pic.update_call_or_panic(
            setup.root_id,
            protocol::CANIC_INSTALL_DELEGATION_PROOF_BATCH,
            (install_request,),
        );
    installed.expect("batch install application failed")
}

fn assert_install_outcome(
    response: &RootDelegationProofBatchInstallResponse,
    issuer_pid: Principal,
    expected: RootDelegationProofInstallOutcome,
) {
    let outcome = response
        .outcomes
        .iter()
        .find(|entry| entry.issuer_pid == issuer_pid)
        .unwrap_or_else(|| panic!("missing install outcome for issuer {issuer_pid}"));
    assert_eq!(outcome.outcome, expected);
}

fn assert_active_delegation_proof_status(
    setup: &RootSetup,
    shard_pid: Principal,
    prepared: &RootDelegationProofBatchPrepareResponse,
) -> ActiveDelegationProofStatusResponse {
    assert_active_delegation_proof_status_for_cert(setup, shard_pid, prepared.entries[0].cert_hash)
}

fn assert_active_delegation_proof_status_for_cert(
    setup: &RootSetup,
    shard_pid: Principal,
    cert_hash: [u8; 32],
) -> ActiveDelegationProofStatusResponse {
    let status = query_active_delegation_proof_status(setup, shard_pid);
    assert_eq!(status.status, ActiveDelegationProofStatus::Valid);
    assert_eq!(status.root_pid, Some(setup.root_id));
    assert_eq!(status.issuer_pid, Some(shard_pid));
    assert_eq!(status.cert_hash, Some(cert_hash));
    status
}

fn verify_issuer_local_delegated_token(
    setup: &RootSetup,
    verifier_pid: Principal,
    shard_pid: Principal,
    subject: Principal,
    status: &ActiveDelegationProofStatusResponse,
) {
    let token_ttl_ns = status
        .expires_at_ns
        .expect("valid active proof must expose expiry")
        .saturating_sub(setup.pic.current_time_nanos())
        .saturating_sub(1_000_000_000)
        .min(10_000_000_000);
    assert!(token_ttl_ns > 0, "active proof must have token lifetime");
    let token = issue_delegated_token_from_active_proof_with_request_nonce(
        &setup.pic,
        shard_pid,
        subject,
        DelegationAudience::Project("test".to_string()),
        vec![role_grant(canister::TEST, vec![cap::VERIFY.to_string()])],
        token_ttl_ns,
        setup.pic.current_time_nanos(),
    );
    let verified: Result<(), Error> = setup.pic.update_call_as_or_panic(
        verifier_pid,
        subject,
        "test_verify_delegated_token",
        (token,),
    );
    verified.expect("delegated token verifier application failed");
}

fn assert_repeated_batch_install_is_idempotent(
    setup: &RootSetup,
    install_request: RootDelegationProofBatchInstallRequest,
) {
    let repeated: Result<RootDelegationProofBatchInstallResponse, Error> =
        setup.pic.update_call_or_panic(
            setup.root_id,
            protocol::CANIC_INSTALL_DELEGATION_PROOF_BATCH,
            (install_request,),
        );
    let repeated = repeated.expect("repeated batch install application failed");
    assert_eq!(repeated.outcomes.len(), 1);
    assert_eq!(
        repeated.outcomes[0].outcome,
        RootDelegationProofInstallOutcome::AlreadyInstalled
    );
}

fn assert_active_delegation_proof_refresh_and_expiry(
    setup: &RootSetup,
    shard_pid: Principal,
    subject: Principal,
    installed_status: &ActiveDelegationProofStatusResponse,
) {
    advance_pic_to_ns(
        setup,
        installed_status
            .refresh_after_ns
            .expect("valid active proof status must expose refresh threshold"),
    );
    let refresh_status = query_active_delegation_proof_status(setup, shard_pid);
    assert_eq!(
        refresh_status.status,
        ActiveDelegationProofStatus::RefreshNeeded
    );
    assert_eq!(refresh_status.cert_hash, installed_status.cert_hash);

    advance_pic_to_ns(
        setup,
        installed_status
            .expires_at_ns
            .expect("valid active proof status must expose expiry"),
    );
    let expired_status = query_active_delegation_proof_status(setup, shard_pid);
    assert_eq!(expired_status.status, ActiveDelegationProofStatus::Expired);
    assert_eq!(expired_status.cert_hash, installed_status.cert_hash);

    let request = DelegatedTokenPrepareRequest {
        metadata: Some(batch_metadata(57, shard_pid)),
        subject,
        aud: DelegationAudience::Project("test".to_string()),
        grants: vec![role_grant(canister::TEST, vec![cap::VERIFY.to_string()])],
        ttl_ns: 1_000_000_000,
        ext: None,
    };
    let prepared: Result<DelegatedTokenPrepareResponse, Error> = setup.pic.update_call_as_or_panic(
        shard_pid,
        subject,
        protocol::CANIC_PREPARE_DELEGATED_TOKEN,
        (request,),
    );
    let err = prepared.expect_err("expired active proof must stop new delegated-token prepare");
    assert_eq!(err.code, ErrorCode::AuthProofExpired);
}

fn query_active_delegation_proof_status(
    setup: &RootSetup,
    shard_pid: Principal,
) -> ActiveDelegationProofStatusResponse {
    let status: Result<ActiveDelegationProofStatusResponse, Error> = setup.pic.query_call_or_panic(
        shard_pid,
        protocol::CANIC_ACTIVE_DELEGATION_PROOF_STATUS,
        (),
    );
    status.expect("active delegation proof status application failed")
}

fn advance_pic_to_ns(setup: &RootSetup, target_ns: u64) {
    let now_ns = setup.pic.current_time_nanos();
    if target_ns > now_ns {
        setup
            .pic
            .advance_time(Duration::from_nanos(target_ns - now_ns));
    }
    setup.pic.tick();
}

fn batch_metadata(id: u8, issuer_pid: Principal) -> AuthRequestMetadata {
    let mut request_id = [id; 32];
    for (index, byte) in issuer_pid.as_slice().iter().enumerate() {
        request_id[index % request_id.len()] ^= *byte;
    }
    AuthRequestMetadata {
        request_id,
        ttl_ns: 60_000_000_000,
    }
}
