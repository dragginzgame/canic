use super::AuthPolicyError;
use crate::{domain::value::Principal, ids::CanisterRole};

///
/// RootDelegationAudiencePolicy
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RootDelegationAudiencePolicy {
    Canister(Principal),
    CanicSubnet(Principal),
    Project(String),
}

///
/// RootDelegatedRoleGrantPolicy
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootDelegatedRoleGrantPolicy {
    pub target: CanisterRole,
    pub scopes: Vec<String>,
}

///
/// RootIssuerPolicy
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootIssuerPolicy {
    pub issuer_pid: Principal,
    pub enabled: bool,
    pub allowed_audiences: Vec<RootDelegationAudiencePolicy>,
    pub allowed_grants: Vec<RootDelegatedRoleGrantPolicy>,
    pub max_cert_ttl_ns: u64,
    pub refresh_after_ratio_bps: u16,
}

///
/// RootIssuerRenewalTemplate
///
/// Root-managed desired renewal shape for one delegated-token issuer.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootIssuerRenewalTemplate {
    pub issuer_pid: Principal,
    pub enabled: bool,
    pub audience: RootDelegationAudiencePolicy,
    pub grants: Vec<RootDelegatedRoleGrantPolicy>,
    pub cert_ttl_ns: u64,
}

///
/// RootIssuerRenewalOutcome
///
/// Last root-managed renewal outcome for one delegated-token issuer.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootIssuerRenewalOutcome {
    AlreadyInstalled,
    DriftDetected,
    InstallDeadlineExpired,
    Installed,
    IssuerCallFailed,
    NeverRun,
    PolicyRejected,
    ProofMismatch,
    QuotaExceeded,
    RejectedByIssuer,
    RetrievalExpired,
    TemplateChanged,
    TemplateDisabled,
}

///
/// RootIssuerRenewalState
///
/// Root-owned scheduling state for one delegated-token issuer.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootIssuerRenewalState {
    pub issuer_pid: Principal,
    pub template_fingerprint: [u8; 32],
    pub last_installed_cert_hash: Option<[u8; 32]>,
    pub last_installed_expires_at_ns: Option<u64>,
    pub last_installed_refresh_after_ns: Option<u64>,
    pub active_attempt_id: Option<[u8; 32]>,
    pub last_outcome: RootIssuerRenewalOutcome,
    pub consecutive_failures: u32,
    pub next_attempt_after_ns: u64,
    pub updated_at_ns: u64,
}

///
/// RootIssuerRenewalProofRef
///
/// Root-owned pointer to one prepared renewal proof.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootIssuerRenewalProofRef {
    pub issuer_pid: Principal,
    pub cert_hash: [u8; 32],
}

///
/// RootIssuerRenewalAttemptStatus
///
/// Per-issuer lifecycle state for one scheduled renewal attempt.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootIssuerRenewalAttemptStatus {
    Prepared,
    Installing,
    Installed,
    FailedRetryable,
    FailedTerminal,
    Disabled,
    Expired,
}

///
/// RootIssuerRenewalAttempt
///
/// Root-owned issuer-level scheduled renewal attempt.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootIssuerRenewalAttempt {
    pub attempt_id: [u8; 32],
    pub issuer_pid: Principal,
    pub template_fingerprint: [u8; 32],
    pub batch_id: [u8; 32],
    pub proof_ref: RootIssuerRenewalProofRef,
    pub status: RootIssuerRenewalAttemptStatus,
    pub prepared_at_ns: u64,
    pub retrieval_expires_at_ns: u64,
    pub install_deadline_ns: u64,
    pub prepared_cert_hash: [u8; 32],
    pub prepared_expires_at_ns: u64,
    pub prepared_refresh_after_ns: u64,
    pub failure: Option<RootIssuerRenewalOutcome>,
}

///
/// RootDelegationProofPreparePolicyInput
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootDelegationProofPreparePolicyInput<'a> {
    pub issuer_pid: Principal,
    pub audience: &'a RootDelegationAudiencePolicy,
    pub grants: &'a [RootDelegatedRoleGrantPolicy],
    pub cert_ttl_ns: u64,
    pub issued_at_ns: u64,
}

///
/// RootDelegationProofPreparePolicyDecision
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootDelegationProofPreparePolicyDecision {
    pub expires_at_ns: u64,
    pub refresh_after_ns: u64,
}

/// Validate a root delegation proof prepare request against an issuer policy.
pub fn validate_root_delegation_proof_prepare_policy(
    issuer_policy: Option<&RootIssuerPolicy>,
    input: RootDelegationProofPreparePolicyInput<'_>,
) -> Result<RootDelegationProofPreparePolicyDecision, AuthPolicyError> {
    let policy = issuer_policy.ok_or(AuthPolicyError::RootIssuerUnregistered)?;
    if policy.issuer_pid != input.issuer_pid {
        return Err(AuthPolicyError::RootIssuerPolicyMismatch {
            expected: policy.issuer_pid,
            found: input.issuer_pid,
        });
    }
    if !policy.enabled {
        return Err(AuthPolicyError::RootIssuerDisabled {
            issuer_pid: input.issuer_pid,
        });
    }
    if !policy.allowed_audiences.contains(input.audience) {
        return Err(AuthPolicyError::RootIssuerAudienceNotAllowed {
            issuer_pid: input.issuer_pid,
        });
    }
    validate_root_issuer_grants(&policy.allowed_grants, input.grants)?;
    validate_root_issuer_ttl(input.cert_ttl_ns, policy.max_cert_ttl_ns)?;
    validate_root_issuer_refresh_ratio(policy.refresh_after_ratio_bps)?;

    let expires_at_ns = input
        .issued_at_ns
        .checked_add(input.cert_ttl_ns)
        .ok_or(AuthPolicyError::RootIssuerRefreshAfterOverflow)?;
    let refresh_after_ns = root_issuer_refresh_after_ns(
        input.issued_at_ns,
        input.cert_ttl_ns,
        policy.refresh_after_ratio_bps,
    )?;

    Ok(RootDelegationProofPreparePolicyDecision {
        expires_at_ns,
        refresh_after_ns,
    })
}

/// Validate an enabled root issuer renewal template against issuer policy.
pub fn validate_root_issuer_renewal_template_policy(
    issuer_policy: Option<&RootIssuerPolicy>,
    template: &RootIssuerRenewalTemplate,
) -> Result<(), AuthPolicyError> {
    if !template.enabled {
        return Ok(());
    }

    validate_root_delegation_proof_prepare_policy(
        issuer_policy,
        RootDelegationProofPreparePolicyInput {
            issuer_pid: template.issuer_pid,
            audience: &template.audience,
            grants: &template.grants,
            cert_ttl_ns: template.cert_ttl_ns,
            issued_at_ns: 0,
        },
    )
    .map(|_| ())
}

fn validate_root_issuer_grants(
    allowed: &[RootDelegatedRoleGrantPolicy],
    requested: &[RootDelegatedRoleGrantPolicy],
) -> Result<(), AuthPolicyError> {
    for grant in requested {
        for scope in &grant.scopes {
            if !root_issuer_scope_allowed(allowed, &grant.target, scope) {
                return Err(AuthPolicyError::RootIssuerGrantNotAllowed {
                    role: grant.target.clone(),
                    scope: scope.clone(),
                });
            }
        }
    }
    Ok(())
}

fn root_issuer_scope_allowed(
    allowed: &[RootDelegatedRoleGrantPolicy],
    role: &CanisterRole,
    scope: &str,
) -> bool {
    allowed
        .iter()
        .any(|grant| grant.target == *role && grant.scopes.iter().any(|allowed| allowed == scope))
}

const fn validate_root_issuer_ttl(
    cert_ttl_ns: u64,
    max_cert_ttl_ns: u64,
) -> Result<(), AuthPolicyError> {
    if cert_ttl_ns == 0 {
        return Err(AuthPolicyError::RootIssuerCertTtlZero);
    }
    if cert_ttl_ns > max_cert_ttl_ns {
        return Err(AuthPolicyError::RootIssuerCertTtlExceedsMax {
            cert_ttl_ns,
            max_cert_ttl_ns,
        });
    }
    Ok(())
}

const fn validate_root_issuer_refresh_ratio(
    refresh_after_ratio_bps: u16,
) -> Result<(), AuthPolicyError> {
    if refresh_after_ratio_bps == 0 || refresh_after_ratio_bps >= 10_000 {
        return Err(AuthPolicyError::RootIssuerRefreshRatioInvalid {
            refresh_after_ratio_bps,
        });
    }
    Ok(())
}

fn root_issuer_refresh_after_ns(
    issued_at_ns: u64,
    cert_ttl_ns: u64,
    refresh_after_ratio_bps: u16,
) -> Result<u64, AuthPolicyError> {
    let refresh_offset_ns =
        u64::try_from((u128::from(cert_ttl_ns) * u128::from(refresh_after_ratio_bps)) / 10_000)
            .map_err(|_| AuthPolicyError::RootIssuerRefreshAfterOverflow)?;
    if refresh_offset_ns == 0 || refresh_offset_ns >= cert_ttl_ns {
        return Err(AuthPolicyError::RootIssuerRefreshAfterInvalid);
    }
    issued_at_ns
        .checked_add(refresh_offset_ns)
        .ok_or(AuthPolicyError::RootIssuerRefreshAfterOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::cap;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn issuer_policy() -> RootIssuerPolicy {
        RootIssuerPolicy {
            issuer_pid: p(2),
            enabled: true,
            allowed_audiences: vec![
                RootDelegationAudiencePolicy::Canister(p(4)),
                RootDelegationAudiencePolicy::CanicSubnet(p(5)),
                RootDelegationAudiencePolicy::Project("test".to_string()),
            ],
            allowed_grants: vec![
                root_grant("user_shard", &[cap::SESSION, cap::VERIFY]),
                root_grant("project_instance", &[cap::READ]),
            ],
            max_cert_ttl_ns: 120_000_000_000,
            refresh_after_ratio_bps: 8_000,
        }
    }

    fn root_grant(role: &str, scopes: &[&str]) -> RootDelegatedRoleGrantPolicy {
        RootDelegatedRoleGrantPolicy {
            target: CanisterRole::owned(role.to_string()),
            scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
        }
    }

    fn prepare_input<'a>(
        audience: &'a RootDelegationAudiencePolicy,
        grants: &'a [RootDelegatedRoleGrantPolicy],
    ) -> RootDelegationProofPreparePolicyInput<'a> {
        RootDelegationProofPreparePolicyInput {
            issuer_pid: p(2),
            audience,
            grants,
            cert_ttl_ns: 100_000_000_000,
            issued_at_ns: 10,
        }
    }

    fn renewal_template() -> RootIssuerRenewalTemplate {
        RootIssuerRenewalTemplate {
            issuer_pid: p(2),
            enabled: true,
            audience: RootDelegationAudiencePolicy::Project("test".to_string()),
            grants: vec![root_grant("project_instance", &[cap::READ])],
            cert_ttl_ns: 100_000_000_000,
        }
    }

    #[test]
    fn root_prepare_policy_accepts_registered_enabled_issuer() {
        let policy = issuer_policy();
        let audience = RootDelegationAudiencePolicy::Project("test".to_string());
        let grants = vec![
            root_grant("user_shard", &[cap::SESSION]),
            root_grant("project_instance", &[cap::READ]),
        ];

        let decision = validate_root_delegation_proof_prepare_policy(
            Some(&policy),
            prepare_input(&audience, &grants),
        )
        .expect("registered issuer policy should accept request");

        assert_eq!(
            decision,
            RootDelegationProofPreparePolicyDecision {
                expires_at_ns: 100_000_000_010,
                refresh_after_ns: 80_000_000_010
            }
        );
    }

    #[test]
    fn root_prepare_policy_rejects_unregistered_or_disabled_issuer() {
        let audience = RootDelegationAudiencePolicy::Project("test".to_string());
        let grants = vec![root_grant("user_shard", &[cap::SESSION])];
        let input = prepare_input(&audience, &grants);

        assert_eq!(
            validate_root_delegation_proof_prepare_policy(None, input.clone()),
            Err(AuthPolicyError::RootIssuerUnregistered)
        );

        let mut policy = issuer_policy();
        policy.enabled = false;
        assert_eq!(
            validate_root_delegation_proof_prepare_policy(Some(&policy), input),
            Err(AuthPolicyError::RootIssuerDisabled { issuer_pid: p(2) })
        );
    }

    #[test]
    fn root_prepare_policy_rejects_policy_issuer_mismatch() {
        let mut policy = issuer_policy();
        policy.issuer_pid = p(3);
        let audience = RootDelegationAudiencePolicy::Project("test".to_string());
        let grants = vec![root_grant("user_shard", &[cap::SESSION])];

        assert_eq!(
            validate_root_delegation_proof_prepare_policy(
                Some(&policy),
                prepare_input(&audience, &grants),
            ),
            Err(AuthPolicyError::RootIssuerPolicyMismatch {
                expected: p(3),
                found: p(2),
            })
        );
    }

    #[test]
    fn root_prepare_policy_rejects_audience_or_grant_outside_policy() {
        let policy = issuer_policy();
        let denied_audience = RootDelegationAudiencePolicy::Project("other".to_string());
        let grants = vec![root_grant("user_shard", &[cap::SESSION])];

        assert_eq!(
            validate_root_delegation_proof_prepare_policy(
                Some(&policy),
                prepare_input(&denied_audience, &grants),
            ),
            Err(AuthPolicyError::RootIssuerAudienceNotAllowed { issuer_pid: p(2) })
        );

        let audience = RootDelegationAudiencePolicy::Project("test".to_string());
        let denied_grants = vec![root_grant("project_instance", &[cap::ADMIN])];
        assert_eq!(
            validate_root_delegation_proof_prepare_policy(
                Some(&policy),
                prepare_input(&audience, &denied_grants),
            ),
            Err(AuthPolicyError::RootIssuerGrantNotAllowed {
                role: CanisterRole::owned("project_instance".to_string()),
                scope: cap::ADMIN.to_string(),
            })
        );
    }

    #[test]
    fn root_prepare_policy_rejects_invalid_ttl_or_refresh_policy() {
        let audience = RootDelegationAudiencePolicy::Project("test".to_string());
        let grants = vec![root_grant("user_shard", &[cap::SESSION])];
        let mut input = prepare_input(&audience, &grants);
        input.cert_ttl_ns = 0;

        assert_eq!(
            validate_root_delegation_proof_prepare_policy(Some(&issuer_policy()), input),
            Err(AuthPolicyError::RootIssuerCertTtlZero)
        );

        let mut input = prepare_input(&audience, &grants);
        input.cert_ttl_ns = 121_000_000_000;
        assert_eq!(
            validate_root_delegation_proof_prepare_policy(Some(&issuer_policy()), input),
            Err(AuthPolicyError::RootIssuerCertTtlExceedsMax {
                cert_ttl_ns: 121_000_000_000,
                max_cert_ttl_ns: 120_000_000_000,
            })
        );

        let mut policy = issuer_policy();
        policy.refresh_after_ratio_bps = 10_000;
        assert_eq!(
            validate_root_delegation_proof_prepare_policy(
                Some(&policy),
                prepare_input(&audience, &grants),
            ),
            Err(AuthPolicyError::RootIssuerRefreshRatioInvalid {
                refresh_after_ratio_bps: 10_000,
            })
        );
    }

    #[test]
    fn renewal_template_policy_accepts_registered_enabled_template() {
        let policy = issuer_policy();
        let template = renewal_template();

        assert_eq!(
            validate_root_issuer_renewal_template_policy(Some(&policy), &template),
            Ok(())
        );
    }

    #[test]
    fn renewal_template_policy_rejects_policy_violations_when_enabled() {
        let policy = issuer_policy();
        let mut template = renewal_template();
        template.grants = vec![root_grant("project_instance", &[cap::ADMIN])];

        assert_eq!(
            validate_root_issuer_renewal_template_policy(Some(&policy), &template),
            Err(AuthPolicyError::RootIssuerGrantNotAllowed {
                role: CanisterRole::owned("project_instance".to_string()),
                scope: cap::ADMIN.to_string(),
            })
        );
    }

    #[test]
    fn disabled_renewal_template_does_not_require_registered_policy() {
        let mut template = renewal_template();
        template.enabled = false;

        assert_eq!(
            validate_root_issuer_renewal_template_policy(None, &template),
            Ok(())
        );
    }
}
