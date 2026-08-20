//! Module: domain::policy::pure::auth::application_authorization
//!
//! Responsibility: make closed local-application authorization and admission decisions.
//! Does not own: caller/time acquisition, storage, proof cryptography, serialization, or mutation.
//! Boundary: access and workflow supply model values; policy returns value-only decisions.

use crate::{
    domain::value::Principal,
    model::auth::application_authorization::{
        ApplicationScopeRef, CanonicalApplicationScopes, LocalApplicationAuthorityBinding,
        LocalApplicationAuthoritySnapshot, LocalApplicationSession,
        MAX_ACTIVE_APPLICATION_SESSIONS, MAX_ACTIVE_APPLICATION_SESSIONS_PER_SUBJECT,
        MAX_APPLICATION_PROOF_LIFETIME_NS, MAX_APPLICATION_REPLAY_RECORDS,
        MAX_APPLICATION_REPLAY_RECORDS_PER_SUBJECT, MAX_LOCAL_APPLICATION_SESSION_TTL_NS,
        VerifiedApplicationAuthority,
    },
};

/// Stable-binding action required by one locally activated protected-policy change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationAuthorityBindingTransition {
    AdvanceGeneration,
    Initialize,
    Unchanged,
    UpdateWithoutGeneration,
}

/// Decide whether one protected local policy transition invalidates retained sessions.
#[must_use]
pub fn decide_application_authority_binding_transition(
    previous: Option<&LocalApplicationAuthorityBinding>,
    current: &LocalApplicationAuthorityBinding,
) -> ApplicationAuthorityBindingTransition {
    let Some(previous) = previous else {
        return ApplicationAuthorityBindingTransition::Initialize;
    };
    if previous == current {
        return ApplicationAuthorityBindingTransition::Unchanged;
    }
    match (previous, current) {
        (
            LocalApplicationAuthorityBinding::Disabled,
            LocalApplicationAuthorityBinding::Enabled { .. },
        ) => ApplicationAuthorityBindingTransition::AdvanceGeneration,
        (
            LocalApplicationAuthorityBinding::Enabled { .. }
            | LocalApplicationAuthorityBinding::Disabled,
            LocalApplicationAuthorityBinding::Disabled,
        ) => ApplicationAuthorityBindingTransition::UpdateWithoutGeneration,
        (
            LocalApplicationAuthorityBinding::Enabled {
                fleet: previous_fleet,
                role: previous_role,
                verifier_root_canister_id: previous_root,
                minimum_accepted_registry_epoch: previous_registry_epoch,
                allowed_scopes: previous_scopes,
                maximum_session_ttl_secs: previous_maximum_ttl,
            },
            LocalApplicationAuthorityBinding::Enabled {
                fleet: current_fleet,
                role: current_role,
                verifier_root_canister_id: current_root,
                minimum_accepted_registry_epoch: current_registry_epoch,
                allowed_scopes: current_scopes,
                maximum_session_ttl_secs: current_maximum_ttl,
            },
        ) => {
            let scope_removed = previous_scopes
                .as_slice()
                .iter()
                .any(|scope| !current_scopes.contains(scope.as_scope_ref()));
            if previous_fleet != current_fleet
                || previous_role != current_role
                || previous_root != current_root
                || previous_registry_epoch != current_registry_epoch
                || scope_removed
                || current_maximum_ttl < previous_maximum_ttl
            {
                ApplicationAuthorityBindingTransition::AdvanceGeneration
            } else {
                ApplicationAuthorityBindingTransition::UpdateWithoutGeneration
            }
        }
    }
}

/// Pure authorization inputs acquired once by the synchronous access boundary.
pub struct LocalApplicationAuthorizationPolicyInput<'a> {
    pub actual_caller: Principal,
    pub observed_caller: Principal,
    pub now_ns: u64,
    pub capability_enabled: bool,
    pub authority: Option<&'a LocalApplicationAuthoritySnapshot>,
    pub session: Option<&'a LocalApplicationSession>,
    pub subject_admissible: bool,
    pub required_scope: ApplicationScopeRef<'a>,
}

/// Successful local application identity and its strict session expiry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthorizedApplicationSubject {
    pub subject: Principal,
    pub expires_at_ns: u64,
}

/// Closed synchronous local application authorization result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalApplicationAuthorizationDecision {
    Allow(AuthorizedApplicationSubject),
    Deny(LocalApplicationAuthorizationDenial),
}

/// Closed denial reasons in their normative evaluation precedence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalApplicationAuthorizationDenial {
    Anonymous,
    CallerMismatch,
    Disabled,
    AuthorityUnavailable,
    MissingSession,
    Expired,
    StaleAuthority,
    InadmissibleSubject,
    MissingScope,
}

/// Authorize one already-validated scope using only supplied values.
#[must_use]
pub fn authorize_local_application(
    input: LocalApplicationAuthorizationPolicyInput<'_>,
) -> LocalApplicationAuthorizationDecision {
    let denial = if input.actual_caller == Principal::anonymous() {
        Some(LocalApplicationAuthorizationDenial::Anonymous)
    } else if input.actual_caller != input.observed_caller {
        Some(LocalApplicationAuthorizationDenial::CallerMismatch)
    } else if !input.capability_enabled {
        Some(LocalApplicationAuthorizationDenial::Disabled)
    } else if input.authority.is_none() {
        Some(LocalApplicationAuthorizationDenial::AuthorityUnavailable)
    } else if input.session.is_none() {
        Some(LocalApplicationAuthorizationDenial::MissingSession)
    } else {
        None
    };
    if let Some(denial) = denial {
        return LocalApplicationAuthorizationDecision::Deny(denial);
    }

    let authority = input
        .authority
        .expect("checked protected authority presence");
    let session = input.session.expect("checked canonical session presence");
    if input.now_ns >= session.expires_at_ns() {
        return LocalApplicationAuthorizationDecision::Deny(
            LocalApplicationAuthorizationDenial::Expired,
        );
    }
    if session.transport_caller() != input.actual_caller
        || session.fleet() != authority.fleet()
        || session.role() != authority.role()
        || session.authority_generation() != authority.generation()
    {
        return LocalApplicationAuthorizationDecision::Deny(
            LocalApplicationAuthorizationDenial::StaleAuthority,
        );
    }
    if !input.subject_admissible {
        return LocalApplicationAuthorizationDecision::Deny(
            LocalApplicationAuthorizationDenial::InadmissibleSubject,
        );
    }
    if !session.scopes().contains(input.required_scope) {
        return LocalApplicationAuthorizationDecision::Deny(
            LocalApplicationAuthorizationDenial::MissingScope,
        );
    }
    LocalApplicationAuthorizationDecision::Allow(AuthorizedApplicationSubject {
        subject: session.authenticated_subject(),
        expires_at_ns: session.expires_at_ns(),
    })
}

/// Validate caller binding, current validity and the complete proof lifetime.
pub fn validate_application_proof_eligibility(
    authority: &VerifiedApplicationAuthority,
    caller: Principal,
    now_ns: u64,
) -> Result<(), ApplicationProofEligibilityError> {
    if authority.presenter() != caller || authority.subject() != caller {
        return Err(ApplicationProofEligibilityError::CallerMismatch);
    }
    if now_ns < authority.proof_not_before_ns() || now_ns >= authority.proof_expires_at_ns() {
        return Err(ApplicationProofEligibilityError::NotCurrentlyValid);
    }
    let lifetime_ns = authority
        .proof_expires_at_ns()
        .checked_sub(authority.proof_issued_at_ns())
        .ok_or(ApplicationProofEligibilityError::InvalidWindow)?;
    if lifetime_ns > MAX_APPLICATION_PROOF_LIFETIME_NS {
        return Err(ApplicationProofEligibilityError::LifetimeTooLong {
            lifetime_ns,
            max_ns: MAX_APPLICATION_PROOF_LIFETIME_NS,
        });
    }
    Ok(())
}

/// Typed application-session proof eligibility failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationProofEligibilityError {
    CallerMismatch,
    InvalidWindow,
    LifetimeTooLong { lifetime_ns: u64, max_ns: u64 },
    NotCurrentlyValid,
}

/// Require the requested canonical scopes to be a non-empty verified subset.
pub fn narrow_application_session_scopes(
    verified: &CanonicalApplicationScopes,
    requested: CanonicalApplicationScopes,
) -> Result<CanonicalApplicationScopes, ApplicationSessionAdmissionError> {
    if let Some(scope) = requested
        .as_slice()
        .iter()
        .find(|scope| !verified.contains(scope.as_scope_ref()))
    {
        return Err(ApplicationSessionAdmissionError::ScopeNotGranted {
            scope: scope.to_string(),
        });
    }
    Ok(requested)
}

/// Resolve one strict session expiry from protected and requested TTL values.
pub fn resolve_application_session_expiry(
    established_at_ns: u64,
    default_ttl_ns: u64,
    maximum_ttl_ns: u64,
    requested_ttl_ns: Option<u64>,
) -> Result<u64, ApplicationSessionTtlError> {
    if default_ttl_ns == 0 {
        return Err(ApplicationSessionTtlError::DefaultZero);
    }
    if maximum_ttl_ns == 0 {
        return Err(ApplicationSessionTtlError::MaximumZero);
    }
    if default_ttl_ns > maximum_ttl_ns {
        return Err(ApplicationSessionTtlError::DefaultExceedsMaximum);
    }
    if maximum_ttl_ns > MAX_LOCAL_APPLICATION_SESSION_TTL_NS {
        return Err(ApplicationSessionTtlError::MaximumExceedsHardLimit {
            maximum_ttl_ns,
            hard_limit_ns: MAX_LOCAL_APPLICATION_SESSION_TTL_NS,
        });
    }
    let selected_ttl_ns = requested_ttl_ns.unwrap_or(default_ttl_ns);
    if selected_ttl_ns == 0 {
        return Err(ApplicationSessionTtlError::RequestedZero);
    }
    if selected_ttl_ns > maximum_ttl_ns {
        return Err(ApplicationSessionTtlError::RequestedExceedsMaximum);
    }
    established_at_ns
        .checked_add(selected_ttl_ns)
        .ok_or(ApplicationSessionTtlError::ExpiryOverflow)
}

/// Typed protected/requested local session TTL failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationSessionTtlError {
    DefaultExceedsMaximum,
    DefaultZero,
    ExpiryOverflow,
    MaximumExceedsHardLimit {
        maximum_ttl_ns: u64,
        hard_limit_ns: u64,
    },
    MaximumZero,
    RequestedExceedsMaximum,
    RequestedZero,
}

/// Existing replay disposition resolved before expensive proof verification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationReplayDisposition {
    Absent,
    Conflict,
    ExactActiveReceipt,
}

/// Target-local post-cleanup occupancy used by pure admission policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApplicationSessionCapacity {
    pub active_global: usize,
    pub active_for_subject: usize,
    pub replay_global: usize,
    pub replay_for_subject: usize,
}

/// Pure replay, replacement and capacity decision input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApplicationSessionAdmissionInput {
    pub replay: ApplicationReplayDisposition,
    pub replacing_existing_session: bool,
    pub capacity: ApplicationSessionCapacity,
}

/// Pure admission result; commit remains an ops/workflow responsibility.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationSessionAdmissionDecision {
    CommitNew,
    CommitReplacement,
    ReturnExactReceipt,
}

/// Typed admission failure that leaves existing state unchanged.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplicationSessionAdmissionError {
    Capacity(ApplicationCapacityLimit),
    ReplayConflict,
    ScopeNotGranted { scope: String },
}

/// Exact target-local capacity bound that denied fresh growth.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationCapacityLimit {
    ActiveGlobal,
    ActiveSubject,
    ReplayGlobal,
    ReplaySubject,
}

/// Decide exact retry, conflict, replacement and target-local capacity.
pub const fn decide_application_session_admission(
    input: ApplicationSessionAdmissionInput,
) -> Result<ApplicationSessionAdmissionDecision, ApplicationSessionAdmissionError> {
    match input.replay {
        ApplicationReplayDisposition::ExactActiveReceipt => {
            return Ok(ApplicationSessionAdmissionDecision::ReturnExactReceipt);
        }
        ApplicationReplayDisposition::Conflict => {
            return Err(ApplicationSessionAdmissionError::ReplayConflict);
        }
        ApplicationReplayDisposition::Absent => {}
    }

    if input.capacity.replay_for_subject >= MAX_APPLICATION_REPLAY_RECORDS_PER_SUBJECT {
        return Err(ApplicationSessionAdmissionError::Capacity(
            ApplicationCapacityLimit::ReplaySubject,
        ));
    }
    if input.capacity.replay_global >= MAX_APPLICATION_REPLAY_RECORDS {
        return Err(ApplicationSessionAdmissionError::Capacity(
            ApplicationCapacityLimit::ReplayGlobal,
        ));
    }
    if !input.replacing_existing_session {
        if input.capacity.active_for_subject >= MAX_ACTIVE_APPLICATION_SESSIONS_PER_SUBJECT {
            return Err(ApplicationSessionAdmissionError::Capacity(
                ApplicationCapacityLimit::ActiveSubject,
            ));
        }
        if input.capacity.active_global >= MAX_ACTIVE_APPLICATION_SESSIONS {
            return Err(ApplicationSessionAdmissionError::Capacity(
                ApplicationCapacityLimit::ActiveGlobal,
            ));
        }
        return Ok(ApplicationSessionAdmissionDecision::CommitNew);
    }
    Ok(ApplicationSessionAdmissionDecision::CommitReplacement)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ids::CanisterRole, model::auth::application_authorization::ApplicationScope,
        test::support::fleet_key,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn scopes(values: &[&str]) -> CanonicalApplicationScopes {
        CanonicalApplicationScopes::for_session(
            values
                .iter()
                .map(|value| ApplicationScope::parse(*value).unwrap())
                .collect(),
        )
        .unwrap()
    }

    fn session() -> LocalApplicationSession {
        LocalApplicationSession::new(
            p(1),
            p(1),
            p(2),
            fleet_key(3),
            CanisterRole::new("component"),
            scopes(&["app:read"]),
            4,
            10,
            100,
            [5; 32],
            [6; 32],
        )
        .unwrap()
    }

    fn authority() -> LocalApplicationAuthoritySnapshot {
        LocalApplicationAuthoritySnapshot::new(fleet_key(3), CanisterRole::new("component"), 4)
    }

    fn binding(
        fleet: u8,
        role: &'static str,
        root: u8,
        registry_epoch: u64,
        allowed_scopes: &[&str],
        maximum_ttl_secs: u64,
    ) -> LocalApplicationAuthorityBinding {
        LocalApplicationAuthorityBinding::enabled(
            fleet_key(fleet),
            CanisterRole::new(role),
            p(root),
            Some(registry_epoch),
            scopes(allowed_scopes),
            maximum_ttl_secs,
        )
    }

    fn authorization_input<'a>(
        authority: Option<&'a LocalApplicationAuthoritySnapshot>,
        session: Option<&'a LocalApplicationSession>,
    ) -> LocalApplicationAuthorizationPolicyInput<'a> {
        LocalApplicationAuthorizationPolicyInput {
            actual_caller: p(1),
            observed_caller: p(1),
            now_ns: 50,
            capability_enabled: true,
            authority,
            session,
            subject_admissible: true,
            required_scope: ApplicationScopeRef::from_static("app:read"),
        }
    }

    #[test]
    fn authorization_allows_exact_current_session_scope() {
        let authority = authority();
        let session = session();
        assert_eq!(
            authorize_local_application(authorization_input(Some(&authority), Some(&session))),
            LocalApplicationAuthorizationDecision::Allow(AuthorizedApplicationSubject {
                subject: p(1),
                expires_at_ns: 100,
            })
        );
    }

    #[test]
    fn authority_binding_transition_applies_the_frozen_generation_table() {
        let original = binding(1, "component", 9, 4, &["app:read"], 900);
        assert_eq!(
            decide_application_authority_binding_transition(None, &original),
            ApplicationAuthorityBindingTransition::Initialize
        );
        assert_eq!(
            decide_application_authority_binding_transition(Some(&original), &original),
            ApplicationAuthorityBindingTransition::Unchanged
        );

        for future_only in [
            binding(1, "component", 9, 4, &["app:read", "app:write"], 900),
            binding(1, "component", 9, 4, &["app:read"], 1_800),
        ] {
            assert_eq!(
                decide_application_authority_binding_transition(Some(&original), &future_only),
                ApplicationAuthorityBindingTransition::UpdateWithoutGeneration
            );
        }

        for invalidating in [
            binding(2, "component", 9, 4, &["app:read"], 900),
            binding(1, "other", 9, 4, &["app:read"], 900),
            binding(1, "component", 8, 4, &["app:read"], 900),
            binding(1, "component", 9, 5, &["app:read"], 900),
            binding(1, "component", 9, 4, &["app:other"], 900),
            binding(1, "component", 9, 4, &["app:read"], 899),
        ] {
            assert_eq!(
                decide_application_authority_binding_transition(Some(&original), &invalidating),
                ApplicationAuthorityBindingTransition::AdvanceGeneration
            );
        }

        assert_eq!(
            decide_application_authority_binding_transition(
                Some(&original),
                &LocalApplicationAuthorityBinding::Disabled,
            ),
            ApplicationAuthorityBindingTransition::UpdateWithoutGeneration
        );
        assert_eq!(
            decide_application_authority_binding_transition(
                Some(&LocalApplicationAuthorityBinding::Disabled),
                &original,
            ),
            ApplicationAuthorityBindingTransition::AdvanceGeneration
        );
    }

    #[test]
    fn authorization_denial_precedence_is_closed() {
        let authority = authority();
        let session = session();
        let mut input = authorization_input(Some(&authority), Some(&session));
        input.actual_caller = Principal::anonymous();
        input.observed_caller = p(9);
        input.capability_enabled = false;
        input.authority = None;
        input.session = None;
        assert_eq!(
            authorize_local_application(input),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::Anonymous
            )
        );

        let mut input = authorization_input(Some(&authority), Some(&session));
        input.observed_caller = p(9);
        input.capability_enabled = false;
        assert_eq!(
            authorize_local_application(input),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::CallerMismatch
            )
        );

        let mut input = authorization_input(None, None);
        input.capability_enabled = false;
        assert_eq!(
            authorize_local_application(input),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::Disabled
            )
        );
        assert_eq!(
            authorize_local_application(authorization_input(None, None)),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::AuthorityUnavailable
            )
        );
        assert_eq!(
            authorize_local_application(authorization_input(Some(&authority), None)),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::MissingSession
            )
        );
    }

    #[test]
    fn authorization_applies_expiry_staleness_admissibility_and_scope_order() {
        let authority = authority();
        let session = session();

        let mut input = authorization_input(Some(&authority), Some(&session));
        input.now_ns = session.expires_at_ns();
        input.subject_admissible = false;
        input.required_scope = ApplicationScopeRef::from_static("app:write");
        assert_eq!(
            authorize_local_application(input),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::Expired
            )
        );

        let stale_authority =
            LocalApplicationAuthoritySnapshot::new(fleet_key(3), CanisterRole::new("component"), 5);
        let mut input = authorization_input(Some(&stale_authority), Some(&session));
        input.subject_admissible = false;
        assert_eq!(
            authorize_local_application(input),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::StaleAuthority
            )
        );

        let mut input = authorization_input(Some(&authority), Some(&session));
        input.subject_admissible = false;
        input.required_scope = ApplicationScopeRef::from_static("app:write");
        assert_eq!(
            authorize_local_application(input),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::InadmissibleSubject
            )
        );

        let mut input = authorization_input(Some(&authority), Some(&session));
        input.required_scope = ApplicationScopeRef::from_static("app:write");
        assert_eq!(
            authorize_local_application(input),
            LocalApplicationAuthorizationDecision::Deny(
                LocalApplicationAuthorizationDenial::MissingScope
            )
        );
    }

    #[test]
    fn proof_eligibility_accepts_exact_sixty_seconds_and_rejects_longer() {
        let build_authority = |expires_at_ns| {
            VerifiedApplicationAuthority::new(
                p(1),
                p(1),
                p(2),
                fleet_key(3),
                CanisterRole::new("component"),
                scopes(&["app:read"]),
                10,
                10,
                expires_at_ns,
                [5; 32],
            )
            .unwrap()
        };
        let exact = build_authority(10 + MAX_APPLICATION_PROOF_LIFETIME_NS);
        assert_eq!(
            validate_application_proof_eligibility(&exact, p(1), 20),
            Ok(())
        );

        let too_long = build_authority(11 + MAX_APPLICATION_PROOF_LIFETIME_NS);
        assert_eq!(
            validate_application_proof_eligibility(&too_long, p(1), 20),
            Err(ApplicationProofEligibilityError::LifetimeTooLong {
                lifetime_ns: MAX_APPLICATION_PROOF_LIFETIME_NS + 1,
                max_ns: MAX_APPLICATION_PROOF_LIFETIME_NS,
            })
        );
        assert_eq!(
            validate_application_proof_eligibility(&exact, p(9), 20),
            Err(ApplicationProofEligibilityError::CallerMismatch)
        );
    }

    #[test]
    fn scope_narrowing_is_all_or_nothing() {
        let verified = scopes(&["app:read", "app:write"]);
        let requested = scopes(&["app:write"]);
        assert_eq!(
            narrow_application_session_scopes(&verified, requested.clone()),
            Ok(requested)
        );
        assert_eq!(
            narrow_application_session_scopes(&verified, scopes(&["app:admin"])),
            Err(ApplicationSessionAdmissionError::ScopeNotGranted {
                scope: "app:admin".to_string()
            })
        );
    }

    #[test]
    fn ttl_resolution_uses_independent_protected_session_clock() {
        assert_eq!(resolve_application_session_expiry(10, 20, 30, None), Ok(30));
        assert_eq!(
            resolve_application_session_expiry(10, 20, 30, Some(30)),
            Ok(40)
        );
        assert_eq!(
            resolve_application_session_expiry(10, 20, 30, Some(31)),
            Err(ApplicationSessionTtlError::RequestedExceedsMaximum)
        );
        assert_eq!(
            resolve_application_session_expiry(
                10,
                20,
                MAX_LOCAL_APPLICATION_SESSION_TTL_NS + 1,
                None
            ),
            Err(ApplicationSessionTtlError::MaximumExceedsHardLimit {
                maximum_ttl_ns: MAX_LOCAL_APPLICATION_SESSION_TTL_NS + 1,
                hard_limit_ns: MAX_LOCAL_APPLICATION_SESSION_TTL_NS,
            })
        );
    }

    #[test]
    fn replay_and_capacity_policy_never_evicts_live_authority() {
        let full = ApplicationSessionCapacity {
            active_global: MAX_ACTIVE_APPLICATION_SESSIONS,
            active_for_subject: MAX_ACTIVE_APPLICATION_SESSIONS_PER_SUBJECT,
            replay_global: MAX_APPLICATION_REPLAY_RECORDS,
            replay_for_subject: MAX_APPLICATION_REPLAY_RECORDS_PER_SUBJECT,
        };
        assert_eq!(
            decide_application_session_admission(ApplicationSessionAdmissionInput {
                replay: ApplicationReplayDisposition::ExactActiveReceipt,
                replacing_existing_session: false,
                capacity: full,
            }),
            Ok(ApplicationSessionAdmissionDecision::ReturnExactReceipt)
        );
        assert_eq!(
            decide_application_session_admission(ApplicationSessionAdmissionInput {
                replay: ApplicationReplayDisposition::Conflict,
                replacing_existing_session: false,
                capacity: full,
            }),
            Err(ApplicationSessionAdmissionError::ReplayConflict)
        );
        assert_eq!(
            decide_application_session_admission(ApplicationSessionAdmissionInput {
                replay: ApplicationReplayDisposition::Absent,
                replacing_existing_session: true,
                capacity: full,
            }),
            Err(ApplicationSessionAdmissionError::Capacity(
                ApplicationCapacityLimit::ReplaySubject
            ))
        );

        let available_replay = ApplicationSessionCapacity {
            replay_global: 0,
            replay_for_subject: 0,
            ..full
        };
        assert_eq!(
            decide_application_session_admission(ApplicationSessionAdmissionInput {
                replay: ApplicationReplayDisposition::Absent,
                replacing_existing_session: true,
                capacity: available_replay,
            }),
            Ok(ApplicationSessionAdmissionDecision::CommitReplacement)
        );
    }
}
