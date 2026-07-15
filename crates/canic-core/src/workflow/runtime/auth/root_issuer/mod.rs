//! Module: workflow::runtime::auth::root_issuer
//!
//! Responsibility: orchestrate root issuer policy and renewal-template admission.
//! Does not own: DTO conversion, persisted records, or pure admission rules.
//! Boundary: API delegates here; workflow invokes policy before ops mutation.

use super::RuntimeAuthWorkflow;
use crate::{
    InternalError,
    domain::policy::pure::auth::{
        AuthPolicyError, validate_root_issuer_policy_upsert,
        validate_root_issuer_renewal_template_upsert,
    },
    dto::auth::{
        RootIssuerPolicyResponse, RootIssuerPolicyUpsertRequest, RootIssuerRenewalStatusRequest,
        RootIssuerRenewalStatusResponse, RootIssuerRenewalTemplateResponse,
        RootIssuerRenewalTemplateUpsertRequest,
    },
    ops::{auth::AuthOps, ic::IcOps},
};

impl RuntimeAuthWorkflow {
    /// Admit and persist one root issuer policy.
    pub fn upsert_root_issuer_policy(
        request: RootIssuerPolicyUpsertRequest,
    ) -> Result<RootIssuerPolicyResponse, InternalError> {
        let policy = AuthOps::root_issuer_policy_from_request(request);
        validate_root_issuer_policy_upsert(&policy).map_err(map_policy_upsert_error)?;
        Ok(AuthOps::commit_root_issuer_policy(policy))
    }

    /// Admit and persist one root-managed issuer renewal template.
    pub fn upsert_root_issuer_renewal_template(
        request: RootIssuerRenewalTemplateUpsertRequest,
    ) -> Result<RootIssuerRenewalTemplateResponse, InternalError> {
        upsert_root_issuer_renewal_template_with_start(
            request,
            IcOps::now_nanos(),
            Self::start_root_delegation_renewal_timer_soon_if_configured,
        )
    }

    /// Project root-managed renewal status for one issuer.
    pub fn root_issuer_renewal_status(
        request: RootIssuerRenewalStatusRequest,
    ) -> RootIssuerRenewalStatusResponse {
        AuthOps::root_issuer_renewal_status(request)
    }
}

fn upsert_root_issuer_renewal_template_with_start<F>(
    request: RootIssuerRenewalTemplateUpsertRequest,
    now_ns: u64,
    start_timer: F,
) -> Result<RootIssuerRenewalTemplateResponse, InternalError>
where
    F: FnOnce() -> Result<(), InternalError>,
{
    let template = AuthOps::root_issuer_renewal_template_from_request(request);
    let policy = AuthOps::root_issuer_policy(template.issuer_pid);
    validate_root_issuer_renewal_template_upsert(policy.as_ref(), &template)
        .map_err(map_renewal_template_upsert_error)?;

    let enabled = template.enabled;
    let response = AuthOps::commit_root_issuer_renewal_template(template, now_ns);
    if enabled {
        start_timer()?;
    }
    Ok(response)
}

fn map_policy_upsert_error(err: AuthPolicyError) -> InternalError {
    InternalError::invalid_input(err.to_string())
}

fn map_renewal_template_upsert_error(err: AuthPolicyError) -> InternalError {
    match err {
        AuthPolicyError::RootIssuerCertTtlZero => InternalError::invalid_input(
            "root issuer renewal certificate TTL must be greater than zero",
        ),
        AuthPolicyError::RootIssuerRenewalGrantRequired => {
            InternalError::invalid_input(err.to_string())
        }
        _ => InternalError::forbidden(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        dto::{
            auth::{DelegatedRoleGrant, DelegationAudience},
            error::ErrorCode,
        },
        ids::CanisterRole,
        ops::storage::auth::AuthStateOps,
    };
    use std::cell::Cell;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn grant(scope: &str) -> DelegatedRoleGrant {
        DelegatedRoleGrant {
            target: CanisterRole::owned("project_instance".to_string()),
            scopes: vec![scope.to_string()],
        }
    }

    fn policy_request(issuer_pid: Principal) -> RootIssuerPolicyUpsertRequest {
        RootIssuerPolicyUpsertRequest {
            issuer_pid,
            enabled: true,
            allowed_audiences: vec![DelegationAudience::Project("test".to_string())],
            allowed_grants: vec![grant("canic.issue")],
            max_cert_ttl_ns: 120_000_000_000,
            refresh_after_ratio_bps: 8_000,
        }
    }

    fn renewal_request(issuer_pid: Principal) -> RootIssuerRenewalTemplateUpsertRequest {
        RootIssuerRenewalTemplateUpsertRequest {
            issuer_pid,
            enabled: true,
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![grant("canic.issue")],
            cert_ttl_ns: 60_000_000_000,
        }
    }

    #[test]
    fn root_issuer_policy_upsert_accepts_and_advances_registry_epoch() {
        let issuer_pid = p(121);
        let epoch_before = AuthStateOps::delegated_auth_registry_epoch();

        let response = RuntimeAuthWorkflow::upsert_root_issuer_policy(policy_request(issuer_pid))
            .expect("valid root issuer policy should be accepted");

        assert_eq!(response.issuer.issuer_pid, issuer_pid);
        assert_eq!(
            AuthStateOps::root_issuer_policy(issuer_pid)
                .expect("accepted policy must be persisted")
                .issuer_pid,
            issuer_pid
        );
        assert_eq!(
            AuthStateOps::delegated_auth_registry_epoch(),
            epoch_before + 1
        );
    }

    #[test]
    fn root_issuer_policy_rejections_preserve_policy_and_registry_epoch() {
        let issuer_pid = p(122);
        RuntimeAuthWorkflow::upsert_root_issuer_policy(policy_request(issuer_pid))
            .expect("baseline root issuer policy should be accepted");
        let policy_before = AuthStateOps::root_issuer_policy(issuer_pid);
        let epoch_before = AuthStateOps::delegated_auth_registry_epoch();

        let mut zero_ttl = policy_request(issuer_pid);
        zero_ttl.max_cert_ttl_ns = 0;
        let mut zero_ratio = policy_request(issuer_pid);
        zero_ratio.refresh_after_ratio_bps = 0;
        let mut full_ratio = policy_request(issuer_pid);
        full_ratio.refresh_after_ratio_bps = 10_000;
        let mut no_audience = policy_request(issuer_pid);
        no_audience.allowed_audiences.clear();
        let mut no_grant = policy_request(issuer_pid);
        no_grant.allowed_grants.clear();

        for request in [zero_ttl, zero_ratio, full_ratio, no_audience, no_grant] {
            let err = RuntimeAuthWorkflow::upsert_root_issuer_policy(request)
                .expect_err("invalid root issuer policy must be rejected");
            assert_eq!(
                err.public_error().map(|error| error.code),
                Some(ErrorCode::InvalidInput)
            );
            assert_eq!(AuthStateOps::root_issuer_policy(issuer_pid), policy_before);
            assert_eq!(AuthStateOps::delegated_auth_registry_epoch(), epoch_before);
        }
    }

    #[test]
    fn renewal_template_admission_precedes_mutation_and_timer_start() {
        let issuer_pid = p(123);
        RuntimeAuthWorkflow::upsert_root_issuer_policy(policy_request(issuer_pid))
            .expect("root issuer policy should be accepted");
        let timer_starts = Cell::new(0);

        let response =
            upsert_root_issuer_renewal_template_with_start(renewal_request(issuer_pid), 90, || {
                assert!(AuthStateOps::root_issuer_renewal_template(issuer_pid).is_some());
                timer_starts.set(timer_starts.get() + 1);
                Ok(())
            })
            .expect("matching renewal template should be accepted");

        assert_eq!(response.template.issuer_pid, issuer_pid);
        assert_eq!(timer_starts.get(), 1);
        assert!(AuthStateOps::root_issuer_renewal_template(issuer_pid).is_some());
    }

    #[test]
    fn renewal_template_rejections_preserve_state_and_skip_timer() {
        let issuer_pid = p(124);
        RuntimeAuthWorkflow::upsert_root_issuer_policy(policy_request(issuer_pid))
            .expect("root issuer policy should be accepted");
        let epoch_before = AuthStateOps::delegated_auth_registry_epoch();
        let timer_started = Cell::new(false);

        let mut zero_ttl = renewal_request(issuer_pid);
        zero_ttl.cert_ttl_ns = 0;
        let mut no_grant = renewal_request(issuer_pid);
        no_grant.grants.clear();
        let mut widened = renewal_request(issuer_pid);
        widened.grants = vec![grant("canic.admin")];
        let unregistered = renewal_request(p(125));

        for (request, expected_code) in [
            (zero_ttl, ErrorCode::InvalidInput),
            (no_grant, ErrorCode::InvalidInput),
            (widened, ErrorCode::Forbidden),
            (unregistered, ErrorCode::Forbidden),
        ] {
            let rejected_issuer = request.issuer_pid;
            let err = upsert_root_issuer_renewal_template_with_start(request, 90, || {
                timer_started.set(true);
                Ok(())
            })
            .expect_err("invalid renewal template must be rejected");

            assert_eq!(
                err.public_error().map(|error| error.code),
                Some(expected_code)
            );
            assert!(AuthStateOps::root_issuer_renewal_template(rejected_issuer).is_none());
            assert_eq!(AuthStateOps::delegated_auth_registry_epoch(), epoch_before);
            assert!(!timer_started.get());
        }
    }
}
