use super::*;
use crate::{
    cdk::types::Principal,
    dto::auth::{
        ChainKeyAlgorithm, ChainKeyBatchHeaderV1, ChainKeyBatchWitnessV1, ChainKeyDelegationCertV1,
        ChainKeyKeyId, DelegatedRoleGrant, DelegationAudience, DelegationCert,
        IssuerProofAlgorithm, IssuerProofBinding, RootIssuerRenewalBatchStatus,
        RootIssuerRenewalStatusRequest,
    },
    ids::CanisterRole,
    model::auth::{
        RootDelegatedRoleGrantPolicy, RootDelegationAudiencePolicy, RootIssuerPolicy,
        RootIssuerRenewalState,
    },
    ops::storage::auth::{
        AuthStateOps, ChainKeyRootDelegationBatch, ChainKeyRootDelegationBatchIssuer,
        ChainKeyRootDelegationBatchStatus,
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

fn renewal_batch(
    batch_id: [u8; 32],
    issuer_pid: Principal,
    cert_hash: [u8; 32],
) -> ChainKeyRootDelegationBatch {
    let root_pid = p(1);
    let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 {
        seed_hash: [42; 32],
    };
    ChainKeyRootDelegationBatch {
        batch_id,
        status: ChainKeyRootDelegationBatchStatus::Installing,
        header_hash: [43; 32],
        header: ChainKeyBatchHeaderV1 {
            schema_version: 1,
            root_canister_id: root_pid,
            batch_id,
            proof_epoch: u64::from(batch_id[0]),
            registry_epoch: 1,
            registry_hash: [44; 32],
            tree_root: [45; 32],
            not_before_ns: 10,
            expires_at_ns: 200,
            algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
            key_id: ChainKeyKeyId {
                name: "test_key_1".to_string(),
            },
            derivation_path_hash: [46; 32],
            key_version: 1,
        },
        signature: None,
        issuers: vec![ChainKeyRootDelegationBatchIssuer {
            issuer_pid,
            cert_hash,
            delegation_cert: DelegationCert {
                root_pid,
                issuer_pid,
                issuer_proof_alg: IssuerProofAlgorithm::IcCanisterSignatureV1,
                issuer_proof_binding_hash: [47; 32],
                issuer_proof_binding,
                issued_at_ns: 10,
                not_before_ns: 10,
                expires_at_ns: 200,
                max_token_ttl_ns: 60,
                aud: DelegationAudience::Project("test".to_string()),
                grants: vec![grant("canic.issue")],
            },
            chain_key_delegation_cert: ChainKeyDelegationCertV1 {
                root_canister_id: root_pid,
                issuer_canister_id: issuer_pid,
                proof_epoch: u64::from(batch_id[0]),
                issuer_proof_algorithm: IssuerProofAlgorithm::IcCanisterSignatureV1,
                issuer_proof_binding_hash: [47; 32],
                issuer_proof_binding,
                max_token_ttl_ns: 60,
                audience: DelegationAudience::Project("test".to_string()),
                grants: vec![grant("canic.issue")],
                not_before_ns: 10,
                expires_at_ns: 200,
                registry_epoch: 1,
                registry_hash: [44; 32],
            },
            issuer_witness: ChainKeyBatchWitnessV1 { steps: Vec::new() },
            refresh_after_ns: 160,
            installed_at_ns: None,
            last_failure: None,
        }],
        prepared_at_ns: 10,
        signed_at_ns: Some(20),
        install_started_at_ns: Some(30),
        installed_at_ns: None,
        retry_after_ns: None,
        failure: None,
    }
}

#[test]
fn commit_root_issuer_renewal_template_persists_projected_template() {
    let issuer_pid = p(81);
    let template = root_issuer_renewal_template_from_request(upsert_request(issuer_pid));

    let response = commit_root_issuer_renewal_template(template, 10);

    assert_eq!(response.template.issuer_pid, issuer_pid);
    assert_eq!(response.template.grants, vec![grant("canic.issue")]);
    assert_eq!(
        root_issuer_renewal_status(RootIssuerRenewalStatusRequest { issuer_pid }).template,
        Some(response.template)
    );
}

#[test]
fn disabled_root_issuer_renewal_template_can_be_staged_without_policy() {
    let issuer_pid = p(83);
    let mut request = upsert_request(issuer_pid);
    request.enabled = false;
    request.grants.clear();

    let template = root_issuer_renewal_template_from_request(request);
    let response = commit_root_issuer_renewal_template(template, 10);

    assert!(!response.template.enabled);
    assert_eq!(
        root_issuer_renewal_status(RootIssuerRenewalStatusRequest { issuer_pid }).template,
        Some(response.template)
    );
}

#[test]
fn disabling_root_issuer_renewal_template_records_disabled_state() {
    let issuer_pid = p(84);
    AuthStateOps::upsert_root_issuer_policy(policy(issuer_pid));
    let active_template = root_issuer_renewal_template_from_request(upsert_request(issuer_pid));
    AuthStateOps::upsert_root_issuer_renewal_template(active_template.clone());
    let active_fingerprint = renewal_template_fingerprint(&active_template);
    AuthStateOps::upsert_root_issuer_renewal_state(RootIssuerRenewalState {
        issuer_pid,
        template_fingerprint: active_fingerprint,
        last_installed_cert_hash: None,
        last_installed_expires_at_ns: None,
        last_installed_refresh_after_ns: None,
        next_attempt_after_ns: 0,
        updated_at_ns: 10,
    });
    let mut request = upsert_request(issuer_pid);
    request.enabled = false;

    let template = root_issuer_renewal_template_from_request(request);
    let response = commit_root_issuer_renewal_template(template, 90);

    assert!(!response.template.enabled);
    let state = AuthStateOps::root_issuer_renewal_state(issuer_pid)
        .expect("issuer renewal state should remain observable");
    assert_eq!(state.next_attempt_after_ns, 90);
    assert_eq!(state.updated_at_ns, 90);
    assert_ne!(state.template_fingerprint, active_fingerprint);
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
        next_attempt_after_ns: 90,
        updated_at_ns: 80,
    };
    AuthStateOps::upsert_root_issuer_renewal_state(state);

    let status = root_issuer_renewal_status(RootIssuerRenewalStatusRequest { issuer_pid });

    assert_eq!(status.template, None);
    assert_eq!(
        status
            .state
            .as_ref()
            .map(|state| state.next_attempt_after_ns),
        Some(90)
    );
    assert_eq!(status.latest_batch, None);
}

#[test]
fn root_issuer_renewal_status_projects_latest_chain_key_batch() {
    let issuer_pid = p(88);
    let installed_issuer_pid = p(89);
    AuthStateOps::upsert_chain_key_root_delegation_batch(renewal_batch(
        [4; 32], issuer_pid, [5; 32],
    ));
    let mut latest = renewal_batch([6; 32], issuer_pid, [7; 32]);
    latest.issuers[0].last_failure = Some("CallFailed".to_string());
    let mut installed_issuer = renewal_batch([6; 32], installed_issuer_pid, [8; 32])
        .issuers
        .remove(0);
    installed_issuer.installed_at_ns = Some(40);
    latest.issuers.push(installed_issuer);
    latest.retry_after_ns = Some(300);
    latest.failure = Some("CallFailed".to_string());
    AuthStateOps::upsert_chain_key_root_delegation_batch(latest);

    let status = root_issuer_renewal_status(RootIssuerRenewalStatusRequest { issuer_pid });
    let latest = status
        .latest_batch
        .expect("latest chain-key batch should be projected");

    assert_eq!(latest.batch_id, [6; 32]);
    assert_eq!(latest.status, RootIssuerRenewalBatchStatus::Installing);
    assert_eq!(latest.cert_hash, [7; 32]);
    assert_eq!(latest.proof_epoch, 6);
    assert_eq!(latest.retry_after_ns, Some(300));
    assert_eq!(latest.failure.as_deref(), Some("CallFailed"));

    let installed_status = root_issuer_renewal_status(RootIssuerRenewalStatusRequest {
        issuer_pid: installed_issuer_pid,
    });
    let installed = installed_status
        .latest_batch
        .expect("installed issuer should project the shared batch");
    assert_eq!(installed.status, RootIssuerRenewalBatchStatus::Installed);
    assert_eq!(installed.installed_at_ns, Some(40));
    assert_eq!(installed.failure, None);
}
