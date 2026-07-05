use super::*;
use crate::{
    cdk::types::Principal,
    domain::policy::pure::auth::{
        RootDelegatedRoleGrantPolicy, RootDelegationAudiencePolicy, RootIssuerPolicy,
        RootIssuerRenewalAttempt, RootIssuerRenewalProofRef, RootIssuerRenewalState,
    },
    dto::auth::{
        DelegatedRoleGrant, DelegationAudience, RootIssuerRenewalOutcome,
        RootIssuerRenewalStatusRequest,
    },
    ids::CanisterRole,
    ops::storage::auth::AuthStateOps,
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
    let attempt_id = [85; 32];
    AuthStateOps::upsert_root_issuer_policy(policy(issuer_pid));
    let active_template = root_issuer_renewal_template_from_request(upsert_request(issuer_pid));
    AuthStateOps::upsert_root_issuer_renewal_template(active_template.clone());
    let mut active_attempt = renewal_attempt(attempt_id, [86; 32], issuer_pid, [87; 32]);
    active_attempt.template_fingerprint = renewal_template_fingerprint(&active_template);
    AuthStateOps::upsert_root_issuer_renewal_attempt(active_attempt.clone());
    AuthStateOps::upsert_root_issuer_renewal_state(RootIssuerRenewalState {
        issuer_pid,
        template_fingerprint: active_attempt.template_fingerprint,
        last_installed_cert_hash: None,
        last_installed_expires_at_ns: None,
        last_installed_refresh_after_ns: None,
        active_attempt_id: Some(attempt_id),
        last_outcome: PolicyRenewalOutcome::NeverRun,
        consecutive_failures: 0,
        next_attempt_after_ns: 0,
        updated_at_ns: 10,
    });
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
