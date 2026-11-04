//! Observe declared policy and live protected enrollment without activating authority.
use super::*;
use canic_contracts::dto::portable_inspection::{
    DeclaredPolicy, EnrollmentObservation, PolicyReport, ReceiverPolicy, Refusal, RuntimeState,
};

pub fn inspection_policy(now_ns: u64) -> Result<PolicyReport, Refusal> {
    let authority =
        AuthOps::local_application_authorization_authority().map_err(|_| Refusal::Unavailable)?;
    let declared = authority.as_ref().map(|authority| DeclaredPolicy {
        fleet: authority.snapshot.fleet(),
        role: authority.snapshot.role().clone(),
        allowed_scopes: authority.config.allowed_scopes.clone(),
        default_session_ttl_secs: authority.config.default_session_ttl_secs,
        maximum_session_ttl_secs: authority.config.maximum_session_ttl_secs,
        receiver: authority
            .config
            .receiver
            .as_ref()
            .map(|profile| ReceiverPolicy {
                clock_skew_ns: profile.clock_skew_ns,
                max_login_ttl_ns: profile.max_login_ttl_ns,
                max_proof_ttl_ns: profile.max_proof_ttl_ns,
                max_certificate_age_ns: profile.max_certificate_age_ns,
            }),
    });
    let runtime = match authority {
        None => RuntimeState::Disabled,
        Some(authority) if authority.config.receiver.is_none() => RuntimeState::Disabled,
        Some(authority) => {
            let network = ConfigOps::delegated_tokens_config()
                .map_err(|_| Refusal::Unavailable)?
                .build_network;
            if require_qualified_clock(network).is_err() {
                RuntimeState::UnqualifiedClock
            } else {
                STATE.with_borrow_mut(|state| {
                    let Some(state) = state.as_mut() else {
                        return RuntimeState::Unavailable;
                    };
                    if check_binding(state, &authority.config, &authority.snapshot).is_err()
                        || state.owner.view(now_ns, |_, _| Ok(())).is_err()
                    {
                        return RuntimeState::Unavailable;
                    }
                    let Some(gate) = state.gate.as_ref() else {
                        return RuntimeState::WaitingForEnrollment;
                    };
                    let Ok(snapshot) = gate.current() else {
                        return RuntimeState::WaitingForEnrollment;
                    };
                    RuntimeState::Ready(EnrollmentObservation {
                        authority: snapshot.authority,
                        revision: snapshot.revision,
                        generation: snapshot.generation,
                        expires_at_ns: snapshot.expires_at_ns,
                        issuers: snapshot.entries.clone(),
                    })
                })
            }
        }
    };
    Ok(PolicyReport {
        observed_at_ns: now_ns,
        authority_generation:
            LocalApplicationAuthorizationStateOps::application_authority_generation(),
        declared,
        runtime,
    })
}
