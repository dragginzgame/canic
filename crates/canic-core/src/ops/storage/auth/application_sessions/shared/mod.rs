//! Module: ops::storage::auth::application_sessions::shared
//!
//! Responsibility: stage a shared session/replay pair against one current authority.
//! Boundary: IC Auth owns admission; Canic owns stable writes, indexes and generation.

#[cfg(test)]
mod tests;

use super::*;
use ic_auth::session::{
    Replay, Session, SessionError, SessionLimits, SessionOccupancy, SessionStore,
    SessionTransaction,
};

/// Target-bound durable backend; no asynchronous work may enter its transaction.
pub struct ApplicationSessionStore {
    pub fleet: canic_contracts::ids::FleetKey,
    pub role: canic_contracts::ids::CanisterRole,
}

impl ApplicationSessionStore {
    pub fn new(
        authority: &crate::model::auth::application_authorization::LocalApplicationAuthoritySnapshot,
    ) -> Self {
        Self {
            fleet: authority.fleet(),
            role: authority.role().clone(),
        }
    }
    #[cfg(feature = "auth-local-application-authorization")]
    pub fn project_committed(
        caller: Principal,
    ) -> Result<LocalApplicationSession, crate::InternalError> {
        LocalApplicationAuthorizationStateOps::application_session(caller)
            .map_err(|_| crate::InternalError::unavailable())?
            .ok_or_else(crate::InternalError::unavailable)
    }
}

#[derive(Eq, PartialEq)]
struct PairAuthority {
    generation: u64,
    subject: Principal,
    fingerprint: [u8; 32],
}

struct Transaction<'a> {
    owner: &'a ApplicationSessionStore,
    generation: u64,
    staged: Option<(LocalApplicationSessionRecord, LocalApplicationReplayRecord)>,
}

impl SessionStore for ApplicationSessionStore {
    fn transaction<T>(
        &mut self,
        operation: impl FnOnce(&mut dyn SessionTransaction) -> Result<T, SessionError>,
    ) -> Result<T, SessionError> {
        let mut tx = Transaction {
            owner: self,
            generation: LocalApplicationAuthorizationStateOps::application_authority_generation(),
            staged: None,
        };
        let result = operation(&mut tx)?;
        if let Some((session, replay)) = tx.staged.take() {
            commit_pair(session, replay, tx.generation)?;
        }
        Ok(result)
    }
}

impl SessionTransaction for Transaction<'_> {
    fn generation(&self) -> u64 {
        self.generation
    }
    fn limits(&self) -> SessionLimits {
        SessionLimits {
            max_sessions: MAX_ACTIVE_APPLICATION_SESSIONS,
            max_replays: MAX_APPLICATION_REPLAY_RECORDS,
            max_replays_per_subject: MAX_APPLICATION_REPLAY_RECORDS_PER_SUBJECT,
            max_encoded_bytes: MAX_APPLICATION_SESSION_STABLE_BYTES,
        }
    }
    fn session(&self, caller: Principal) -> Result<Option<Session>, SessionError> {
        let index = with_indexes(|indexes| indexes.session_by_caller.get(&caller).copied())
            .map_err(|_| SessionError::StorageFailure)?;
        index
            .map(|index| {
                let record = LocalApplicationAuthorizationState::application_session_record(index)
                    .ok_or(SessionError::StorageFailure)?;
                let session = Session::decode(&record.shared_session)?;
                let expected = LocalApplicationSessionRecord {
                    shared_session: record.shared_session.clone(),
                    transport_caller: session.caller(),
                    authenticated_subject: session.subject(),
                    issuer: session.issuer(),
                    fleet: self.owner.fleet,
                    role: self.owner.role.clone(),
                    scopes: session.scopes().to_vec(),
                    authority_generation: session.generation(),
                    established_at_ns: session.established_at_ns(),
                    expires_at_ns: session.expires_at_ns(),
                    proof_fingerprint: session.fingerprint(),
                    establishment_request_hash: session.request_hash(),
                };
                if session.caller() != caller || record != expected {
                    return Err(SessionError::InvalidPolicyOrRecord);
                }
                Ok(session)
            })
            .transpose()
    }
    fn replay_exists(&self, fingerprint: [u8; 32]) -> Result<bool, SessionError> {
        with_indexes(|indexes| indexes.replay_by_fingerprint.contains_key(&fingerprint))
            .map_err(|_| SessionError::StorageFailure)
    }
    fn occupancy(&self, subject: Principal) -> Result<SessionOccupancy, SessionError> {
        LocalApplicationAuthorizationStateOps::application_session_occupancy(subject)
            .map(|counts| SessionOccupancy {
                sessions: counts.active_global,
                replays: counts.replay_global,
                replays_for_subject: counts.replay_for_subject,
            })
            .map_err(|_| SessionError::StorageFailure)
    }
    fn stage(&mut self, session: Session, replay: Replay) -> Result<(), SessionError> {
        let session_binding = PairAuthority {
            generation: session.generation(),
            subject: session.subject(),
            fingerprint: session.fingerprint(),
        };
        let replay_binding = PairAuthority {
            generation: replay.generation(),
            subject: replay.subject(),
            fingerprint: replay.fingerprint(),
        };
        if self.staged.is_some()
            || session_binding != replay_binding
            || session.generation() != self.generation
        {
            return Err(SessionError::InvalidPolicyOrRecord);
        }
        if self.replay_exists(replay.fingerprint())? {
            return Err(SessionError::ReplayConflict);
        }
        let projection = project_session(&session, self.owner, session.encode()?)?;
        let record = session_to_record(&projection);
        if encoded_len(&record).map_err(|_| SessionError::StorageFailure)?
            > MAX_APPLICATION_SESSION_RECORD_BYTES
        {
            return Err(SessionError::ByteCapacity);
        }
        let replay = LocalApplicationReplayRecord {
            proof_fingerprint: replay.fingerprint(),
            transport_caller: session.caller(),
            authenticated_subject: replay.subject(),
            authority_generation: replay.generation(),
            remove_at_ns: replay.remove_at_ns(),
        };
        self.staged = Some((record, replay));
        Ok(())
    }
}

fn commit_pair(
    session: LocalApplicationSessionRecord,
    replay: LocalApplicationReplayRecord,
    generation: u64,
) -> Result<(), SessionError> {
    if generation != LocalApplicationAuthorizationStateOps::application_authority_generation() {
        return Err(SessionError::StaleAuthority);
    }
    let (session_index, replay_index, previous) = with_indexes(|indexes| {
        (
            indexes
                .session_by_caller
                .get(&session.transport_caller)
                .copied(),
            indexes.replay_by_fingerprint.len(),
            indexes
                .replay_by_fingerprint
                .contains_key(&replay.proof_fingerprint),
        )
    })
    .map_err(|_| SessionError::StorageFailure)?;
    if previous {
        return Err(SessionError::ReplayConflict);
    }
    let counts = LocalApplicationAuthorizationStateOps::application_session_occupancy(
        session.authenticated_subject,
    )
    .map_err(|_| SessionError::StorageFailure)?;
    if (session_index.is_none() && counts.active_global >= MAX_ACTIVE_APPLICATION_SESSIONS)
        || counts.replay_global >= MAX_APPLICATION_REPLAY_RECORDS
        || counts.replay_for_subject >= MAX_APPLICATION_REPLAY_RECORDS_PER_SUBJECT
    {
        return Err(SessionError::Capacity);
    }
    let previous_bytes = match session_index {
        Some(index) => encoded_len(
            &LocalApplicationAuthorizationState::application_session_record(index)
                .ok_or(SessionError::StorageFailure)?,
        )
        .map_err(|_| SessionError::StorageFailure)?,
        None => 0,
    };
    let added_bytes = encoded_len(&session).map_err(|_| SessionError::StorageFailure)?
        + encoded_len(&replay).map_err(|_| SessionError::StorageFailure)?;
    let (encoded_bytes, index_bytes) = with_indexes(|indexes| {
        let session_subjects = indexes.session_count_by_subject.len()
            + usize::from(
                !indexes
                    .session_count_by_subject
                    .contains_key(&session.authenticated_subject),
            );
        let replay_subjects = indexes.replay_count_by_subject.len()
            + usize::from(
                !indexes
                    .replay_count_by_subject
                    .contains_key(&replay.authenticated_subject),
            );
        (
            indexes
                .encoded_bytes
                .checked_sub(previous_bytes)
                .and_then(|n| n.checked_add(added_bytes)),
            index_bytes_for_counts(
                counts.active_global + usize::from(session_index.is_none()),
                counts.replay_global + 1,
                session_subjects,
                replay_subjects,
            ),
        )
    })
    .map_err(|_| SessionError::StorageFailure)?;
    let encoded_bytes = encoded_bytes.ok_or(SessionError::StorageFailure)?;
    if encoded_bytes > MAX_APPLICATION_SESSION_STABLE_BYTES
        || index_bytes > MAX_APPLICATION_SESSION_INDEX_BYTES
    {
        return Err(SessionError::ByteCapacity);
    }
    let session_index = session_index.unwrap_or(counts.active_global);
    // All codec, capacity and generation checks precede either stable write.
    LocalApplicationAuthorizationState::commit_shared_session_pair(
        session_index,
        session.clone(),
        replay_index,
        replay,
    );
    update_committed_indexes(
        &session,
        &replay,
        session_index,
        replay_index,
        encoded_bytes,
        index_bytes,
    );
    Ok(())
}

// The stable pair is committed; updating its derived indexes is infallible.
fn update_committed_indexes(
    session: &LocalApplicationSessionRecord,
    replay: &LocalApplicationReplayRecord,
    session_index: usize,
    replay_index: usize,
    encoded_bytes: usize,
    index_bytes: usize,
) {
    APPLICATION_SESSION_INDEXES.with_borrow_mut(|slot| {
        let indexes = slot.as_mut().expect("admitted session indexes");
        indexes.encoded_bytes = encoded_bytes;
        indexes.estimated_bytes = index_bytes;
        let replaced = indexes
            .session_by_caller
            .insert(session.transport_caller, session_index)
            .is_some();
        if !replaced {
            *indexes
                .session_count_by_subject
                .entry(session.authenticated_subject)
                .or_default() += 1;
        }
        indexes
            .replay_by_fingerprint
            .insert(replay.proof_fingerprint, replay_index);
        *indexes
            .replay_count_by_subject
            .entry(replay.authenticated_subject)
            .or_default() += 1;
    });
}
fn project_session(
    session: &Session,
    owner: &ApplicationSessionStore,
    payload: Vec<u8>,
) -> Result<LocalApplicationSession, SessionError> {
    let scopes = session
        .scopes()
        .iter()
        .map(|scope| ApplicationScope::parse(scope.clone()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| SessionError::InvalidPolicyOrRecord)?;
    let scopes = CanonicalApplicationScopes::for_session(scopes)
        .map_err(|_| SessionError::InvalidPolicyOrRecord)?;
    Ok(LocalApplicationSession::new(
        session.caller(),
        session.subject(),
        session.issuer(),
        owner.fleet,
        owner.role.clone(),
        scopes,
        session.generation(),
        session.established_at_ns(),
        session.expires_at_ns(),
        session.fingerprint(),
        session.request_hash(),
    )
    .map_err(|_| SessionError::InvalidPolicyOrRecord)?
    .with_shared_session(payload))
}
