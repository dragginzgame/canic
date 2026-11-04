//! Read bounded canonical Auth session metadata without changing authority or retention.
use super::*;
use canic_contracts::{
    dto::portable_inspection::{Page, Refusal, Request, SessionObservation},
    protocol::portable_inspection::MAX_PAGE,
};
use std::ops::Bound::{Excluded, Unbounded};

impl LocalApplicationAuthorizationStateOps {
    /// Caller authorization belongs to the endpoint; this projection never admits a session.
    pub(crate) fn inspect_application_sessions(
        request: Request,
        now_ns: u64,
    ) -> Result<Page, Refusal> {
        if request.limit == 0 || request.limit > MAX_PAGE {
            return Err(Refusal::InvalidPage);
        }
        if now_ns < LocalApplicationAuthorizationState::application_observed_at_ns() {
            return Err(Refusal::Clock);
        }
        let after = request.after.map(Principal::from);
        let rows = with_indexes(|indexes| {
            indexes
                .session_by_caller
                .range((after.map_or(Unbounded, Excluded), Unbounded))
                .take(usize::from(request.limit) + 1)
                .map(|(caller, index)| (*caller, *index))
                .collect::<Vec<_>>()
        })
        .map_err(|_| Refusal::Unavailable)?;
        let generation = Self::application_authority_generation();
        let has_more = rows.len() > usize::from(request.limit);
        let sessions = rows
            .into_iter()
            .take(usize::from(request.limit))
            .map(|(caller, index)| {
                let row = LocalApplicationAuthorizationState::application_session_record(index)
                    .ok_or(Refusal::Unavailable)?;
                let session =
                    Session::decode(&row.shared_session).map_err(|_| Refusal::Unavailable)?;
                if session.caller() != caller.into() {
                    return Err(Refusal::Unavailable);
                }
                Ok(SessionObservation {
                    caller: session.caller(),
                    issuer: session.issuer(),
                    subject: session.subject(),
                    scopes: session
                        .scopes()
                        .iter()
                        .map(|scope| scope.as_str().to_owned())
                        .collect(),
                    established_at_ns: session.established_at_ns(),
                    expires_at_ns: session.expires_at_ns(),
                    generation: session.generation(),
                    expired: now_ns >= session.expires_at_ns(),
                    stale_generation: session.generation() != generation,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let next_after = if has_more {
            sessions.last().map(|session| session.caller)
        } else {
            None
        };
        Ok(Page {
            observed_at_ns: now_ns,
            generation,
            sessions,
            next_after,
        })
    }
}
