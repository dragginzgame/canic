//! Delegated-auth chain-key hard-cut acceptance gates.
//!
//! These tests keep the first production implementation driven by executable
//! acceptance criteria, not only by the design doc.

use candid::{decode_one, encode_args};
use canic::{
    Error,
    cdk::types::Principal,
    dto::{
        auth::{
            ActiveDelegationProofStatus, ActiveDelegationProofStatusResponse, AuthRequestMetadata,
            DelegatedToken, DelegatedTokenGetRequest, DelegatedTokenPrepareRequest,
            DelegatedTokenPrepareResponse, DelegationAudience, RootIssuerPolicyResponse,
            RootIssuerPolicyUpsertRequest, RootIssuerRenewalStatusRequest,
            RootIssuerRenewalStatusResponse, RootIssuerRenewalTemplateResponse,
            RootIssuerRenewalTemplateUpsertRequest,
        },
        error::ErrorCode,
        metrics::{MetricEntry, MetricValue, MetricsKind},
        page::{Page, PageRequest},
    },
    ids::{CanisterRole, cap},
    protocol,
};
use canic_testing_internal::{
    canister,
    pic::{create_user_shard, role_grant},
};
use canic_tests::root::{
    RootSetupProfile,
    harness::{RootSetup, setup_cached_root, setup_root},
};

const TEST_CHAIN_KEY_ECDSA_PUBLIC_KEY: &str = "test_chain_key_ecdsa_public_key";
const TEST_CHAIN_KEY_KEY_NAME: &str = "key_1";
const TEST_PROVISION_CHAIN_KEY_DELEGATION_PROOF: &str =
    "test_provision_chain_key_delegation_proof_for_issuer";
const TEST_FLEET_CHAIN_KEY_PUBLIC_KEY_HEX: &str =
    "02f1a0a900c4b9d53ff5ec024c0e37e7a54174c6ae1d0a77312aebea349adc0b7a";
const SECOND_NS: u64 = 1_000_000_000;

#[test]
fn delegated_auth_chain_key_management_public_key_matches_test_fleet_trust_anchor() {
    let setup = setup_cached_root(RootSetupProfile::Capability);
    let public_key: Result<Vec<u8>, Error> = setup.pic.update_call_or_panic(
        setup.root_id,
        TEST_CHAIN_KEY_ECDSA_PUBLIC_KEY,
        (
            setup.root_id,
            TEST_CHAIN_KEY_KEY_NAME.to_string(),
            chain_key_derivation_path(),
        ),
    );
    let public_key = public_key.expect("key_1 public-key probe application failed");
    drop(setup);

    assert_eq!(
        hex_lower(&public_key),
        TEST_FLEET_CHAIN_KEY_PUBLIC_KEY_HEX,
        "local fleet chain-key trust anchor must match the management-canister derived root key",
    );
}

#[test]
fn delegated_auth_chain_key_batch_renews_without_external_liveness() {
    let setup = setup_root(RootSetupProfile::Sharding);
    let verifier_pid = sharding_profile_pid(&setup, &canister::TEST, "test verifier");
    let user_hub_pid = sharding_profile_pid(&setup, &canister::USER_HUB, "user_hub");
    let subject = Principal::from_slice(&[76; 29]);
    let issuer_pid = create_user_shard(&setup.pic, user_hub_pid, subject);

    let initial_status = active_delegation_proof_status(&setup, issuer_pid);
    assert_eq!(initial_status.status, ActiveDelegationProofStatus::Missing);

    upsert_root_issuer_policy(&setup, issuer_pid);
    upsert_root_issuer_renewal_template(&setup, issuer_pid);

    let renewed_status = wait_for_active_delegation_proof(&setup, issuer_pid);
    assert_eq!(renewed_status.root_pid, Some(setup.root_id));
    assert_eq!(renewed_status.issuer_pid, Some(issuer_pid));

    verify_issuer_local_delegated_token(&setup, verifier_pid, issuer_pid, subject, &renewed_status);
    drop(setup);
}

#[test]
fn delegated_auth_root_facade_provisions_new_issuer_before_login() {
    let setup = setup_root(RootSetupProfile::Sharding);
    let user_hub_pid = sharding_profile_pid(&setup, &canister::USER_HUB, "user_hub");
    let subject = Principal::from_slice(&[79; 29]);
    let issuer_pid = create_user_shard(&setup.pic, user_hub_pid, subject);

    assert_eq!(
        active_delegation_proof_status(&setup, issuer_pid).status,
        ActiveDelegationProofStatus::Missing,
    );
    upsert_root_issuer_policy(&setup, issuer_pid);
    upsert_root_issuer_renewal_template(&setup, issuer_pid);

    let provisioned: Result<(), Error> = setup.pic.update_call_or_panic(
        setup.root_id,
        TEST_PROVISION_CHAIN_KEY_DELEGATION_PROOF,
        (issuer_pid,),
    );
    provisioned.expect("root issuer-readiness provisioning failed");

    let status = active_delegation_proof_status(&setup, issuer_pid);
    assert_eq!(status.status, ActiveDelegationProofStatus::Valid);
    assert_eq!(status.root_pid, Some(setup.root_id));
    assert_eq!(status.issuer_pid, Some(issuer_pid));
    drop(setup);
}

#[test]
fn delegated_auth_lazy_repair_uses_cached_batch_and_does_not_sign_per_login() {
    let setup = setup_root(RootSetupProfile::Sharding);
    let verifier_pid = sharding_profile_pid(&setup, &canister::TEST, "test verifier");
    let user_hub_pid = sharding_profile_pid(&setup, &canister::USER_HUB, "user_hub");
    let subject = Principal::from_slice(&[77; 29]);
    let issuer_pid = create_user_shard(&setup.pic, user_hub_pid, subject);

    assert_eq!(
        active_delegation_proof_status(&setup, issuer_pid).status,
        ActiveDelegationProofStatus::Missing,
    );
    upsert_root_issuer_policy(&setup, issuer_pid);
    upsert_root_issuer_renewal_template(&setup, issuer_pid);
    assert_eq!(
        active_delegation_proof_status(&setup, issuer_pid).status,
        ActiveDelegationProofStatus::Missing,
        "test must start the login path without timer-installed active proof"
    );

    let management_updates_before = root_management_update_completed_count(&setup);
    issue_and_verify_delegated_token_with_bounded_retry(
        &setup,
        verifier_pid,
        issuer_pid,
        subject,
        10 * SECOND_NS,
        1,
    );

    let repaired_status = active_delegation_proof_status(&setup, issuer_pid);
    assert_eq!(repaired_status.status, ActiveDelegationProofStatus::Valid);
    assert_eq!(repaired_status.root_pid, Some(setup.root_id));
    assert_eq!(repaired_status.issuer_pid, Some(issuer_pid));

    let management_updates_after_repair = root_management_update_completed_count(&setup);
    assert_eq!(
        management_updates_after_repair,
        management_updates_before + 2,
        "first missing-proof login should make exactly one root chain-key signing operation"
    );

    issue_and_verify_delegated_token(&setup, verifier_pid, issuer_pid, subject, 10 * SECOND_NS, 2);

    assert_eq!(
        root_management_update_completed_count(&setup),
        management_updates_after_repair,
        "repeated login under a fresh active proof must not call root threshold signing"
    );
    let repeated_status = active_delegation_proof_status(&setup, issuer_pid);
    drop(setup);
    assert_eq!(repeated_status.status, ActiveDelegationProofStatus::Valid);
    assert_eq!(
        repeated_status.cert_hash, repaired_status.cert_hash,
        "repeated login should keep using the issuer-local active proof"
    );
}

#[test]
fn delegated_auth_concurrent_missing_proof_repairs_collapse_to_one_signature() {
    let setup = setup_root(RootSetupProfile::Sharding);
    let verifier_pid = sharding_profile_pid(&setup, &canister::TEST, "test verifier");
    let user_hub_pid = sharding_profile_pid(&setup, &canister::USER_HUB, "user_hub");
    let subject = Principal::from_slice(&[78; 29]);
    let issuer_pid = create_user_shard(&setup.pic, user_hub_pid, subject);

    assert_eq!(
        active_delegation_proof_status(&setup, issuer_pid).status,
        ActiveDelegationProofStatus::Missing,
    );
    upsert_root_issuer_policy(&setup, issuer_pid);
    upsert_root_issuer_renewal_template(&setup, issuer_pid);
    assert_eq!(
        active_delegation_proof_status(&setup, issuer_pid).status,
        ActiveDelegationProofStatus::Missing,
        "test must submit concurrent login updates while the issuer has no active proof"
    );

    let management_updates_before = root_management_update_completed_count(&setup);
    let messages = (0..2)
        .map(|index| {
            let request =
                delegated_token_prepare_request(issuer_pid, subject, 10 * SECOND_NS, index);
            let payload = encode_args((request,)).expect("encode prepare request");
            setup
                .pic
                .submit_call(
                    issuer_pid,
                    subject,
                    protocol::CANIC_PREPARE_DELEGATED_TOKEN,
                    payload,
                )
                .expect("submit concurrent delegated-token prepare")
        })
        .collect::<Vec<_>>();

    for _ in 0..80 {
        setup.pic.tick();
    }

    let mut successes = Vec::new();
    let mut retryable_failures = 0usize;
    for message in messages {
        let bytes = setup
            .pic
            .await_call(message)
            .expect("await concurrent delegated-token prepare");
        let prepared: Result<DelegatedTokenPrepareResponse, Error> =
            decode_one(&bytes).expect("decode delegated-token prepare result");
        match prepared {
            Ok(prepared) => successes.push(prepared),
            Err(err) if chain_key_proof_retryable(&err) => retryable_failures += 1,
            Err(err) => panic!("concurrent delegated-token prepare failed permanently: {err:?}"),
        }
    }

    assert_eq!(
        successes.len() + retryable_failures,
        2,
        "both concurrent missing-proof prepares must either succeed or return the bounded retry signal"
    );

    for prepared in successes {
        let token = get_prepared_delegated_token(&setup, issuer_pid, subject, prepared);
        verify_delegated_token(&setup, verifier_pid, subject, token);
    }
    for index in 0..retryable_failures {
        issue_and_verify_delegated_token_with_bounded_retry(
            &setup,
            verifier_pid,
            issuer_pid,
            subject,
            10 * SECOND_NS,
            100 + index as u64,
        );
    }

    let repaired_status = active_delegation_proof_status(&setup, issuer_pid);
    assert_eq!(repaired_status.status, ActiveDelegationProofStatus::Valid);
    assert_eq!(
        root_management_update_completed_count(&setup),
        management_updates_before + 2,
        "concurrent missing-proof login updates and retries should perform one root chain-key signing operation"
    );

    issue_and_verify_delegated_token(
        &setup,
        verifier_pid,
        issuer_pid,
        subject,
        10 * SECOND_NS,
        200,
    );
    let management_updates_after_reuse = root_management_update_completed_count(&setup);
    drop(setup);
    assert_eq!(
        management_updates_after_reuse,
        management_updates_before + 2,
        "fresh issuer-local proof reuse must require zero additional threshold signatures"
    );
}

#[test]
fn delegated_auth_timer_batches_multiple_issuers_with_one_signature() {
    let setup = setup_root(RootSetupProfile::Sharding);
    let verifier_pid = sharding_profile_pid(&setup, &canister::TEST, "test verifier");
    let user_hub_pid = sharding_profile_pid(&setup, &canister::USER_HUB, "user_hub");
    let issuers = create_distinct_user_shard_issuers(&setup, user_hub_pid, 2);

    for (_, issuer_pid) in &issuers {
        assert_eq!(
            active_delegation_proof_status(&setup, *issuer_pid).status,
            ActiveDelegationProofStatus::Missing,
        );
        upsert_root_issuer_policy(&setup, *issuer_pid);
        upsert_root_issuer_renewal_template(&setup, *issuer_pid);
    }

    let management_updates_before = root_management_update_completed_count(&setup);
    let statuses = issuers
        .iter()
        .map(|(subject, issuer_pid)| {
            let status = wait_for_active_delegation_proof(&setup, *issuer_pid);
            assert_eq!(status.root_pid, Some(setup.root_id));
            assert_eq!(status.issuer_pid, Some(*issuer_pid));
            verify_issuer_local_delegated_token(
                &setup,
                verifier_pid,
                *issuer_pid,
                *subject,
                &status,
            );
            status
        })
        .collect::<Vec<_>>();

    let management_updates_after_renewal = root_management_update_completed_count(&setup);
    drop(setup);
    assert_eq!(
        management_updates_after_renewal,
        management_updates_before + 2,
        "timer renewal for two due issuers should perform one root chain-key signing operation"
    );
    assert_ne!(
        statuses[0].issuer_pid, statuses[1].issuer_pid,
        "test must cover two distinct issuer canisters"
    );
}

fn upsert_root_issuer_policy(setup: &RootSetup, issuer_pid: Principal) {
    let response: Result<RootIssuerPolicyResponse, Error> = setup.pic.update_call_or_panic(
        setup.root_id,
        protocol::CANIC_UPSERT_ROOT_ISSUER_POLICY,
        (RootIssuerPolicyUpsertRequest {
            issuer_pid,
            enabled: true,
            allowed_audiences: vec![DelegationAudience::Project("test".to_string())],
            allowed_grants: vec![role_grant(canister::TEST, vec![cap::VERIFY.to_string()])],
            max_cert_ttl_ns: 60 * SECOND_NS,
            refresh_after_ratio_bps: 8_000,
        },),
    );
    let response = response.expect("root issuer policy upsert application failed");
    assert_eq!(response.issuer.issuer_pid, issuer_pid);
    assert!(response.issuer.enabled);
}

fn upsert_root_issuer_renewal_template(setup: &RootSetup, issuer_pid: Principal) {
    let response: Result<RootIssuerRenewalTemplateResponse, Error> =
        setup.pic.update_call_or_panic(
            setup.root_id,
            protocol::CANIC_UPSERT_ROOT_ISSUER_RENEWAL_TEMPLATE,
            (RootIssuerRenewalTemplateUpsertRequest {
                issuer_pid,
                enabled: true,
                aud: DelegationAudience::Project("test".to_string()),
                grants: vec![role_grant(canister::TEST, vec![cap::VERIFY.to_string()])],
                cert_ttl_ns: 60 * SECOND_NS,
            },),
        );
    let response = response.expect("root issuer renewal template upsert application failed");
    assert_eq!(response.template.issuer_pid, issuer_pid);
    assert!(response.template.enabled);
}

fn wait_for_active_delegation_proof(
    setup: &RootSetup,
    issuer_pid: Principal,
) -> ActiveDelegationProofStatusResponse {
    let mut last_status = active_delegation_proof_status(setup, issuer_pid);
    for _ in 0..80 {
        if last_status.status == ActiveDelegationProofStatus::Valid {
            return last_status;
        }
        setup.pic.tick();
        last_status = active_delegation_proof_status(setup, issuer_pid);
    }

    let renewal_status: Result<RootIssuerRenewalStatusResponse, Error> =
        setup.pic.query_call_or_panic(
            setup.root_id,
            protocol::CANIC_ROOT_ISSUER_RENEWAL_STATUS,
            (RootIssuerRenewalStatusRequest { issuer_pid },),
        );
    panic!(
        "chain-key renewal did not install active proof; issuer_status={last_status:?}; root_status={:?}",
        renewal_status.expect("root renewal status query application failed")
    );
}

fn active_delegation_proof_status(
    setup: &RootSetup,
    issuer_pid: Principal,
) -> ActiveDelegationProofStatusResponse {
    let status: Result<ActiveDelegationProofStatusResponse, Error> = setup.pic.query_call_or_panic(
        issuer_pid,
        protocol::CANIC_ACTIVE_DELEGATION_PROOF_STATUS,
        (),
    );
    status.expect("active delegation proof status application failed")
}

fn create_distinct_user_shard_issuers(
    setup: &RootSetup,
    user_hub_pid: Principal,
    target_count: usize,
) -> Vec<(Principal, Principal)> {
    let mut issuers = Vec::<(Principal, Principal)>::new();
    for index in 0..150 {
        let subject = numbered_subject(80, index);
        let issuer_pid = create_user_shard(&setup.pic, user_hub_pid, subject);
        if issuers
            .iter()
            .all(|(_, existing_pid)| *existing_pid != issuer_pid)
        {
            issuers.push((subject, issuer_pid));
            if issuers.len() == target_count {
                return issuers;
            }
        }
    }

    panic!(
        "could not create {target_count} distinct user-shard issuers; created {}",
        issuers.len()
    );
}

const fn numbered_subject(prefix: u8, index: u16) -> Principal {
    let mut bytes = [prefix; 29];
    let index_bytes = index.to_be_bytes();
    bytes[27] = index_bytes[0];
    bytes[28] = index_bytes[1];
    Principal::from_slice(&bytes)
}

fn verify_issuer_local_delegated_token(
    setup: &RootSetup,
    verifier_pid: Principal,
    issuer_pid: Principal,
    subject: Principal,
    status: &ActiveDelegationProofStatusResponse,
) {
    let token_ttl_ns = status
        .expires_at_ns
        .expect("valid active proof must expose expiry")
        .saturating_sub(setup.pic.current_time_nanos())
        .saturating_sub(SECOND_NS)
        .min(10 * SECOND_NS);
    assert!(token_ttl_ns > 0, "active proof must have token lifetime");
    issue_and_verify_delegated_token(
        setup,
        verifier_pid,
        issuer_pid,
        subject,
        token_ttl_ns,
        setup.pic.current_time_nanos(),
    );
}

fn issue_and_verify_delegated_token(
    setup: &RootSetup,
    verifier_pid: Principal,
    issuer_pid: Principal,
    subject: Principal,
    token_ttl_ns: u64,
    request_nonce: u64,
) {
    let token = issue_delegated_token_once(setup, issuer_pid, subject, token_ttl_ns, request_nonce)
        .expect("delegated-token issue application failed");
    verify_delegated_token(setup, verifier_pid, subject, token);
}

fn issue_and_verify_delegated_token_with_bounded_retry(
    setup: &RootSetup,
    verifier_pid: Principal,
    issuer_pid: Principal,
    subject: Principal,
    token_ttl_ns: u64,
    request_nonce: u64,
) {
    let token = issue_delegated_token_with_bounded_retry(
        setup,
        issuer_pid,
        subject,
        token_ttl_ns,
        request_nonce,
    );
    verify_delegated_token(setup, verifier_pid, subject, token);
}

fn issue_delegated_token_with_bounded_retry(
    setup: &RootSetup,
    issuer_pid: Principal,
    subject: Principal,
    token_ttl_ns: u64,
    request_nonce: u64,
) -> DelegatedToken {
    let mut last_retry: Option<Error> = None;
    for offset in 0..8 {
        match issue_delegated_token_once(
            setup,
            issuer_pid,
            subject,
            token_ttl_ns,
            request_nonce + offset,
        ) {
            Ok(token) => return token,
            Err(err) if chain_key_proof_retryable(&err) => {
                last_retry = Some(err);
                setup.pic.tick();
            }
            Err(err) => panic!("delegated-token issue application failed: {err:?}"),
        }
    }

    panic!(
        "delegated-token issue did not complete after bounded chain-key repair retries: {last_retry:?}"
    );
}

fn issue_delegated_token_once(
    setup: &RootSetup,
    issuer_pid: Principal,
    subject: Principal,
    token_ttl_ns: u64,
    request_nonce: u64,
) -> Result<DelegatedToken, Error> {
    let aud = DelegationAudience::Project("test".to_string());
    let grants = vec![role_grant(canister::TEST, vec![cap::VERIFY.to_string()])];
    let prepared: Result<DelegatedTokenPrepareResponse, Error> = setup.pic.update_call_as_or_panic(
        issuer_pid,
        subject,
        protocol::CANIC_PREPARE_DELEGATED_TOKEN,
        (DelegatedTokenPrepareRequest {
            metadata: Some(token_request_metadata(
                issuer_pid,
                subject,
                token_ttl_ns,
                request_nonce,
            )),
            subject,
            aud,
            grants,
            ttl_ns: token_ttl_ns,
            ext: None,
        },),
    );
    let prepared = prepared?;
    let issued: Result<DelegatedToken, Error> = setup.pic.query_call_as_or_panic(
        issuer_pid,
        subject,
        protocol::CANIC_GET_DELEGATED_TOKEN,
        (DelegatedTokenGetRequest {
            claims_hash: prepared.claims_hash,
        },),
    );
    issued
}

fn delegated_token_prepare_request(
    issuer_pid: Principal,
    subject: Principal,
    token_ttl_ns: u64,
    request_nonce: u64,
) -> DelegatedTokenPrepareRequest {
    DelegatedTokenPrepareRequest {
        metadata: Some(token_request_metadata(
            issuer_pid,
            subject,
            token_ttl_ns,
            request_nonce,
        )),
        subject,
        aud: DelegationAudience::Project("test".to_string()),
        grants: vec![role_grant(canister::TEST, vec![cap::VERIFY.to_string()])],
        ttl_ns: token_ttl_ns,
        ext: None,
    }
}

fn get_prepared_delegated_token(
    setup: &RootSetup,
    issuer_pid: Principal,
    subject: Principal,
    prepared: DelegatedTokenPrepareResponse,
) -> DelegatedToken {
    let issued: Result<DelegatedToken, Error> = setup.pic.query_call_as_or_panic(
        issuer_pid,
        subject,
        protocol::CANIC_GET_DELEGATED_TOKEN,
        (DelegatedTokenGetRequest {
            claims_hash: prepared.claims_hash,
        },),
    );
    issued.expect("delegated-token get application failed")
}

fn chain_key_proof_retryable(err: &Error) -> bool {
    err.code == ErrorCode::AuthProofPending
}

fn token_request_metadata(
    issuer_pid: Principal,
    subject: Principal,
    token_ttl_ns: u64,
    request_nonce: u64,
) -> AuthRequestMetadata {
    let mut request_id = [0u8; 32];
    mix_principal(&mut request_id, 0, issuer_pid);
    mix_principal(&mut request_id, 7, subject);
    mix_u64(&mut request_id, 13, token_ttl_ns);
    mix_u64(&mut request_id, 21, request_nonce);

    AuthRequestMetadata {
        request_id,
        ttl_ns: 60 * SECOND_NS,
    }
}

fn mix_principal(request_id: &mut [u8; 32], offset: usize, principal: Principal) {
    for (index, byte) in principal.as_slice().iter().enumerate() {
        request_id[(offset + index) % 32] ^= *byte;
    }
}

fn mix_u64(request_id: &mut [u8; 32], offset: usize, value: u64) {
    for (index, byte) in value.to_le_bytes().iter().enumerate() {
        request_id[(offset + index) % 32] ^= *byte;
    }
}

fn verify_delegated_token(
    setup: &RootSetup,
    verifier_pid: Principal,
    subject: Principal,
    token: DelegatedToken,
) {
    let verified: Result<(), Error> = setup.pic.update_call_as_or_panic(
        verifier_pid,
        subject,
        "test_verify_delegated_token",
        (token,),
    );
    verified.expect("delegated token verifier application failed");
}

fn root_management_update_completed_count(setup: &RootSetup) -> u64 {
    metric_count(
        &query_metrics(&setup.pic, setup.root_id, MetricsKind::Platform),
        &["platform_call", "management", "update", "completed", "ok"],
    )
}

fn query_metrics(
    pic: &ic_testkit::pic::Pic,
    canister_id: Principal,
    kind: MetricsKind,
) -> Vec<MetricEntry> {
    let response: Result<Page<MetricEntry>, Error> = pic
        .query_call(
            canister_id,
            protocol::CANIC_METRICS,
            (
                kind,
                PageRequest {
                    limit: 256,
                    offset: 0,
                },
            ),
        )
        .expect("metrics transport query failed");

    response.expect("metrics application query failed").entries
}

fn metric_count(entries: &[MetricEntry], labels: &[&str]) -> u64 {
    entries
        .iter()
        .filter(|entry| metric_labels_match(entry, labels))
        .map(metric_entry_count)
        .sum()
}

fn metric_labels_match(entry: &MetricEntry, labels: &[&str]) -> bool {
    entry.labels.len() == labels.len()
        && entry
            .labels
            .iter()
            .zip(labels.iter())
            .all(|(actual, expected)| actual == expected)
}

const fn metric_entry_count(entry: &MetricEntry) -> u64 {
    match entry.value {
        MetricValue::Count(count) | MetricValue::CountAndU64 { count, .. } => count,
        MetricValue::U128(_) => 0,
    }
}

fn sharding_profile_pid(setup: &RootSetup, role: &CanisterRole, label: &str) -> Principal {
    setup
        .subnet_index
        .get(role)
        .copied()
        .unwrap_or_else(|| panic!("{label} must exist in sharding profile"))
}

fn chain_key_derivation_path() -> Vec<Vec<u8>> {
    vec![b"canic".to_vec(), b"delegation".to_vec()]
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}
