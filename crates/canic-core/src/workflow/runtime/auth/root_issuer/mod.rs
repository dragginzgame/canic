//! Module: workflow::runtime::auth::root_issuer
//!
//! Responsibility: admit issuer authority and automatic renewal together.
//! Does not own: DTO conversion, persisted records, or pure admission rules.
//! Boundary: workflow validates the complete configuration before ops mutation.

use crate::{
    InternalError,
    domain::policy::pure::{
        PolicyError,
        auth::{
            validate_root_issuer_policy_fleet_binding, validate_root_issuer_policy_upsert,
            validate_root_issuer_renewal_template_fleet_binding,
            validate_root_issuer_renewal_template_upsert,
        },
    },
    dto::auth::{
        RootIssuerConfigureRequest, RootIssuerConfigureResponse, RootIssuerRenewalStatusRequest,
        RootIssuerRenewalStatusResponse,
    },
    ops::{
        auth::AuthOps,
        ic::IcOps,
        storage::{StorageOpsError, fleet_activation::FleetActivationOps},
    },
    workflow::runtime::auth::RuntimeAuthWorkflow,
};
use canic_contracts::ids::FleetKey;

impl RuntimeAuthWorkflow {
    /// Configure one issuer's authority and renewal before reconciling its timer.
    pub fn configure_root_issuer(
        request: RootIssuerConfigureRequest,
    ) -> Result<RootIssuerConfigureResponse, InternalError> {
        configure_root_issuer_with_reconcile(
            request,
            protected_fleet()?,
            IcOps::now_nanos(),
            Self::reconcile_root_issuer_renewal,
        )
    }

    /// Project root-managed renewal status for one issuer.
    pub fn root_issuer_renewal_status(
        request: RootIssuerRenewalStatusRequest,
    ) -> RootIssuerRenewalStatusResponse {
        AuthOps::root_issuer_renewal_status(request)
    }
}

fn configure_root_issuer_with_reconcile<F>(
    request: RootIssuerConfigureRequest,
    protected_fleet: FleetKey,
    now_ns: u64,
    reconcile: F,
) -> Result<RootIssuerConfigureResponse, InternalError>
where
    F: FnOnce() -> Result<(), InternalError>,
{
    let configuration = AuthOps::root_issuer_configuration_from_request(request)?;
    let policy = &configuration.policy;
    let template = &configuration.template;
    validate_root_issuer_policy_upsert(policy).map_err(PolicyError::AuthPolicy)?;
    validate_root_issuer_policy_fleet_binding(policy, protected_fleet)
        .map_err(PolicyError::AuthPolicy)?;
    validate_root_issuer_renewal_template_fleet_binding(template, protected_fleet)
        .map_err(PolicyError::AuthPolicy)?;
    validate_root_issuer_renewal_template_upsert(Some(policy), template)
        .map_err(PolicyError::AuthPolicy)?;

    let response = AuthOps::commit_root_issuer_configuration(configuration, now_ns);
    reconcile()?;
    Ok(response)
}

fn protected_fleet() -> Result<FleetKey, InternalError> {
    FleetActivationOps::fleet_binding()
        .map(|binding| binding.fleet)
        .map_err(|error| InternalError::from(StorageOpsError::from(error)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        diagnostics::codes,
        dto::auth::{DelegatedRoleGrant, DelegationAudience},
        ops::storage::auth::RootDelegationStateOps,
    };
    use canic_contracts::ids::CanisterRole;
    use std::cell::Cell;

    fn request(issuer: u8) -> RootIssuerConfigureRequest {
        RootIssuerConfigureRequest {
            issuer_pid: Principal::from_slice(&[issuer; 29]),
            enabled: true,
            aud: DelegationAudience::Fleet(crate::test::support::fleet_key(1)),
            grants: vec![DelegatedRoleGrant {
                target: CanisterRole::owned("project_instance".to_string()),
                scopes: vec!["verify".to_string()],
            }],
            cert_ttl_ns: 60_000_000_000,
            refresh_after_ratio_bps: 8_000,
        }
    }

    fn configure(
        request: RootIssuerConfigureRequest,
    ) -> Result<RootIssuerConfigureResponse, InternalError> {
        configure_root_issuer_with_reconcile(
            request,
            crate::test::support::fleet_key(1),
            90,
            || Ok(()),
        )
    }

    #[test]
    fn issuer_configuration_commits_both_records_before_reconciliation() {
        let request = request(121);
        let issuer_pid = request.issuer_pid;
        let epoch = RootDelegationStateOps::delegated_auth_registry_epoch();
        let reconciled = Cell::new(false);
        let response = configure_root_issuer_with_reconcile(
            request.clone(),
            crate::test::support::fleet_key(1),
            90,
            || {
                let policy = RootDelegationStateOps::root_issuer_policy(issuer_pid).unwrap();
                let template =
                    RootDelegationStateOps::root_issuer_renewal_template(issuer_pid).unwrap();
                assert_eq!(policy.allowed_audiences, vec![template.audience]);
                assert_eq!(policy.allowed_grants, template.grants);
                assert_eq!(policy.max_cert_ttl_ns, template.cert_ttl_ns);
                assert_eq!(policy.enabled, template.enabled);
                reconciled.set(true);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(response.issuer.issuer_pid, issuer_pid);
        assert_eq!(response.template.grants, request.grants);
        assert_eq!(
            RootDelegationStateOps::delegated_auth_registry_epoch(),
            epoch + 1
        );
        assert!(reconciled.get());
    }

    #[test]
    fn identical_configuration_retry_preserves_registry_epoch_and_renewal_state() {
        let request = request(122);
        let first = configure(request.clone()).unwrap();
        let epoch = RootDelegationStateOps::delegated_auth_registry_epoch();
        let state = RootDelegationStateOps::root_issuer_renewal_state(request.issuer_pid);
        assert_eq!(configure(request.clone()).unwrap(), first);
        assert_eq!(
            RootDelegationStateOps::delegated_auth_registry_epoch(),
            epoch
        );
        assert_eq!(
            RootDelegationStateOps::root_issuer_renewal_state(request.issuer_pid),
            state
        );
    }

    #[test]
    fn changed_configuration_updates_both_records_and_advances_epoch_once() {
        let mut request = request(123);
        configure(request.clone()).unwrap();
        let epoch = RootDelegationStateOps::delegated_auth_registry_epoch();
        request.cert_ttl_ns /= 2;
        request.grants[0].scopes = vec!["session".to_string()];
        let response = configure(request.clone()).unwrap();
        assert_eq!(response.issuer.allowed_grants, request.grants);
        assert_eq!(response.template.grants, request.grants);
        assert_eq!(response.issuer.max_cert_ttl_ns, request.cert_ttl_ns);
        assert_eq!(response.template.cert_ttl_ns, request.cert_ttl_ns);
        assert_eq!(
            RootDelegationStateOps::delegated_auth_registry_epoch(),
            epoch + 1
        );
    }

    #[test]
    fn invalid_configuration_preserves_both_records_and_skips_reconciliation() {
        let baseline = request(124);
        configure(baseline.clone()).unwrap();
        let issuer_pid = baseline.issuer_pid;
        let policy = RootDelegationStateOps::root_issuer_policy(issuer_pid);
        let template = RootDelegationStateOps::root_issuer_renewal_template(issuer_pid);
        let epoch = RootDelegationStateOps::delegated_auth_registry_epoch();
        let mut zero_ttl = baseline.clone();
        zero_ttl.cert_ttl_ns = 0;
        let mut zero_ratio = baseline.clone();
        zero_ratio.refresh_after_ratio_bps = 0;
        let mut full_ratio = baseline.clone();
        full_ratio.refresh_after_ratio_bps = 10_000;
        let mut no_grants = baseline.clone();
        no_grants.grants.clear();
        let mut wrong_fleet = baseline;
        wrong_fleet.aud = DelegationAudience::Fleet(crate::test::support::fleet_key(2));
        for (request, expected) in [
            (zero_ttl, codes::CONFIGURATION_INVALID),
            (zero_ratio, codes::CONFIGURATION_INVALID),
            (full_ratio, codes::CONFIGURATION_INVALID),
            (no_grants, codes::CONFIGURATION_INCOMPLETE),
            (wrong_fleet, codes::AUTHORITY_CONFLICT),
        ] {
            let err = configure_root_issuer_with_reconcile(
                request,
                crate::test::support::fleet_key(1),
                90,
                || panic!("rejected setup must not reconcile"),
            )
            .unwrap_err();
            assert_eq!(err.public_error().code(), expected.raw_code());
            assert_eq!(
                RootDelegationStateOps::root_issuer_policy(issuer_pid),
                policy
            );
            assert_eq!(
                RootDelegationStateOps::root_issuer_renewal_template(issuer_pid),
                template
            );
            assert_eq!(
                RootDelegationStateOps::delegated_auth_registry_epoch(),
                epoch
            );
        }
    }

    #[test]
    fn equivalent_grant_order_preserves_authority_and_canonicalizes_configuration() {
        let mut request = request(127);
        request.grants[0].scopes = vec!["verify".to_string(), "session".to_string()];
        request.grants.push(DelegatedRoleGrant {
            target: CanisterRole::owned("other_role".to_string()),
            scopes: vec!["verify".to_string()],
        });
        let first = configure(request.clone()).unwrap();
        let epoch = RootDelegationStateOps::delegated_auth_registry_epoch();
        request.grants.reverse();
        request.grants[1].scopes.reverse();
        assert_eq!(configure(request).unwrap(), first);
        assert_eq!(first.template.grants[0].target.as_str(), "other_role");
        assert_eq!(first.template.grants[1].scopes, vec!["session", "verify"]);
        assert_eq!(
            RootDelegationStateOps::delegated_auth_registry_epoch(),
            epoch
        );
    }

    #[test]
    fn malformed_grants_are_rejected_before_either_record_is_written() {
        let request = request(128);
        let epoch = RootDelegationStateOps::delegated_auth_registry_epoch();
        let mut duplicate_role = request.clone();
        duplicate_role.grants.push(duplicate_role.grants[0].clone());
        let mut duplicate_scope = request.clone();
        duplicate_scope.grants[0].scopes.push("verify".to_string());
        let mut invalid_scope = request.clone();
        invalid_scope.grants[0].scopes = vec!["invalid scope".to_string()];
        let mut too_many = request.clone();
        too_many.grants = vec![request.grants[0].clone(); 17];
        for malformed in [duplicate_role, duplicate_scope, invalid_scope, too_many] {
            let err = configure_root_issuer_with_reconcile(
                malformed,
                crate::test::support::fleet_key(1),
                90,
                || panic!("malformed grants must not reconcile"),
            )
            .unwrap_err();
            assert_eq!(
                err.public_error().code(),
                codes::CONFIGURATION_INVALID.raw_code()
            );
            assert!(RootDelegationStateOps::root_issuer_policy(request.issuer_pid).is_none());
            assert!(
                RootDelegationStateOps::root_issuer_renewal_template(request.issuer_pid).is_none()
            );
            assert_eq!(
                RootDelegationStateOps::delegated_auth_registry_epoch(),
                epoch
            );
        }
    }

    #[test]
    fn disabling_issuer_disables_authority_and_renewal_together() {
        let mut request = request(125);
        configure(request.clone()).unwrap();
        request.enabled = false;
        let response = configure(request).unwrap();
        assert!(!response.issuer.enabled);
        assert!(!response.template.enabled);
    }

    #[test]
    fn reconciliation_failure_retains_complete_configuration_for_effect_free_retry() {
        let request = request(126);
        let err = configure_root_issuer_with_reconcile(
            request.clone(),
            crate::test::support::fleet_key(1),
            90,
            || Err(InternalError::public(codes::STATE_UNAVAILABLE)),
        )
        .unwrap_err();
        assert_eq!(
            err.public_error().code(),
            codes::STATE_UNAVAILABLE.raw_code()
        );
        assert!(RootDelegationStateOps::root_issuer_policy(request.issuer_pid).is_some());
        assert!(RootDelegationStateOps::root_issuer_renewal_template(request.issuer_pid).is_some());
        let epoch = RootDelegationStateOps::delegated_auth_registry_epoch();
        configure(request).unwrap();
        assert_eq!(
            RootDelegationStateOps::delegated_auth_registry_epoch(),
            epoch
        );
    }
}
