//! Module: ops::storage::auth::application_sessions
//!
//! Responsibility: own canonical application-session conversion, derived indexes and atomic state mutation.
//! Does not own: proof verification, IC caller acquisition, endpoint DTOs, or authorization policy.
//! Boundary: workflow supplies invariant-bearing model values; stable storage commits one current record set.

#![cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "staged state operations have compiler-version-dependent liveness until the sequenced workflow surface consumes them"
    )
)]

use super::{AuthState, AuthStateOps};
use crate::{
    cdk::types::Principal,
    model::auth::application_authorization::{
        ApplicationScope, CanonicalApplicationScopes, LocalApplicationAuthorityBinding,
        LocalApplicationReplay, LocalApplicationSession, MAX_ACTIVE_APPLICATION_SESSIONS,
        MAX_ACTIVE_APPLICATION_SESSIONS_PER_SUBJECT, MAX_APPLICATION_REPLAY_RECORDS,
        MAX_APPLICATION_REPLAY_RECORDS_PER_SUBJECT, MAX_APPLICATION_SESSION_AUDIT_PAGE_ENTRIES,
        MAX_APPLICATION_SESSION_CLEANUP_REMOVALS, MAX_APPLICATION_SESSION_INDEX_BYTES,
        MAX_APPLICATION_SESSION_RECORD_BYTES, MAX_APPLICATION_SESSION_STABLE_BYTES,
        MAX_LOCAL_APPLICATION_SESSION_TTL_NS,
    },
    ops::runtime::metrics::auth::record_application_session_generation_invalidation,
    storage::stable::auth::{
        LocalApplicationAuthorityBindingRecord, LocalApplicationAuthorizationStateData,
        LocalApplicationReplayRecord, LocalApplicationSessionRecord,
    },
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};
use thiserror::Error;

thread_local! {
    static APPLICATION_SESSION_INDEXES: RefCell<Option<ApplicationSessionIndexes>> =
        const { RefCell::new(None) };
}

#[derive(Clone, Debug, Default)]
struct ApplicationSessionIndexes {
    session_by_caller: BTreeMap<Principal, usize>,
    replay_by_fingerprint: BTreeMap<[u8; 32], usize>,
    session_count_by_subject: BTreeMap<Principal, usize>,
    replay_count_by_subject: BTreeMap<Principal, usize>,
    estimated_bytes: usize,
}

/// Result of resolving one consumed proof before proof verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplicationReplayResolution {
    Absent,
    Conflict,
    ExactActive(Box<LocalApplicationSession>),
}

/// Outcome of one atomic active-session and replay commit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationSessionCommitResult {
    Created,
    Replaced,
}

/// Bounded cleanup result split by canonical record owner.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ApplicationSessionCleanupResult {
    pub sessions_removed: usize,
    pub replays_removed: usize,
}

impl ApplicationSessionCleanupResult {
    #[must_use]
    pub const fn total_removed(self) -> usize {
        self.sessions_removed + self.replays_removed
    }
}

/// Synchronous restore evidence for the reconstructed derived indexes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApplicationSessionRestoreStats {
    pub sessions: usize,
    pub replays: usize,
    pub session_subjects: usize,
    pub replay_subjects: usize,
    pub stable_bytes: usize,
    pub estimated_index_bytes: usize,
}

/// Bounded model-owned page selected from the exact-caller index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationSessionPage {
    pub entries: Vec<LocalApplicationSession>,
    pub total: usize,
}

/// Current physical occupancy projected from the reconstructed storage indexes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApplicationSessionOccupancy {
    pub active_global: usize,
    pub active_for_subject: usize,
    pub replay_global: usize,
    pub replay_for_subject: usize,
}

/// Closed corruption, capacity and atomic-commit failures for application-session state.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ApplicationSessionStateError {
    #[error("application-session derived indexes are unavailable")]
    IndexesUnavailable,

    #[error("application-session state exceeds the active global bound")]
    ActiveGlobalCapacity,

    #[error("application-session state exceeds the active per-subject bound")]
    ActiveSubjectCapacity,

    #[error("application-session state exceeds the replay global bound")]
    ReplayGlobalCapacity,

    #[error("application-session state exceeds the replay per-subject bound")]
    ReplaySubjectCapacity,

    #[error("application-session state contains a duplicate caller")]
    DuplicateCaller,

    #[error("application-session state contains a duplicate proof fingerprint")]
    DuplicateProofFingerprint,

    #[error("application-session record is invalid")]
    InvalidSessionRecord,

    #[error("application replay record is invalid")]
    InvalidReplayRecord,

    #[error("application-session record exceeds its encoded bound")]
    SessionRecordTooLarge,

    #[error("application-session stable state exceeds its encoded bound")]
    StableStateTooLarge,

    #[error("application-session derived indexes exceed their heap bound")]
    IndexStateTooLarge,

    #[error("application-session and replay authority differ")]
    SessionReplayMismatch,

    #[error("application-session generation is newer than local authority")]
    FutureAuthorityGeneration,

    #[error("application-session generation does not match current local authority")]
    AuthorityGenerationMismatch,

    #[error("application authority generation is exhausted")]
    AuthorityGenerationExhausted,

    #[error("application replay already exists")]
    ReplayAlreadyExists,

    #[error("application-session record cannot be encoded")]
    EncodingFailed,

    #[error("application authority binding is invalid")]
    InvalidAuthorityBinding,
}

impl AuthStateOps {
    /// Validate canonical records and synchronously reconstruct every derived index.
    pub fn restore_application_session_state()
    -> Result<ApplicationSessionRestoreStats, ApplicationSessionStateError> {
        let state = AuthState::application_authorization_state();
        let (indexes, restore_report) = build_indexes(&state)?;
        install_indexes(indexes);
        Ok(restore_report)
    }

    /// Return the current target-local application authority generation.
    #[must_use]
    pub fn application_authority_generation() -> u64 {
        AuthState::application_authority_generation()
    }

    /// Return the current locally persisted application authority binding.
    pub fn application_authority_binding()
    -> Result<Option<LocalApplicationAuthorityBinding>, ApplicationSessionStateError> {
        AuthState::application_authorization_state()
            .authority_binding
            .as_ref()
            .map(authority_binding_from_record)
            .transpose()
    }

    /// Persist one binding without changing the current authority generation.
    pub fn set_application_authority_binding(
        current: LocalApplicationAuthorityBinding,
    ) -> Result<(), ApplicationSessionStateError> {
        let mut state = AuthState::application_authorization_state();
        state.authority_binding = Some(authority_binding_to_record(&current));
        commit_validated_state(state)
    }

    /// Persist one binding and atomically advance its authority generation.
    pub fn advance_application_authority_binding_generation(
        current: LocalApplicationAuthorityBinding,
    ) -> Result<(), ApplicationSessionStateError> {
        let mut state = AuthState::application_authorization_state();
        state.authority_generation = state
            .authority_generation
            .checked_add(1)
            .ok_or(ApplicationSessionStateError::AuthorityGenerationExhausted)?;
        state.authority_binding = Some(authority_binding_to_record(&current));
        commit_validated_state(state)?;
        record_application_session_generation_invalidation();
        Ok(())
    }

    /// Resolve one canonical retained session by exact transport caller without cleanup.
    pub fn application_session(
        caller: Principal,
    ) -> Result<Option<LocalApplicationSession>, ApplicationSessionStateError> {
        let index = with_indexes(|indexes| indexes.session_by_caller.get(&caller).copied())?;
        let Some(index) = index else {
            return Ok(None);
        };
        let record = AuthState::application_session_record(index)
            .ok_or(ApplicationSessionStateError::InvalidSessionRecord)?;
        let session = session_from_record(&record)?;
        if session.transport_caller() != caller {
            return Err(ApplicationSessionStateError::InvalidSessionRecord);
        }
        Ok(Some(session))
    }

    /// Return a deterministic bounded page of retained sessions without mutation.
    pub fn application_session_page(
        offset: u64,
        limit: u64,
    ) -> Result<ApplicationSessionPage, ApplicationSessionStateError> {
        let (selected, total) = with_indexes(|indexes| {
            let total = indexes.session_by_caller.len();
            let offset = usize::try_from(offset).unwrap_or(usize::MAX);
            let limit = usize::try_from(limit)
                .unwrap_or(MAX_APPLICATION_SESSION_AUDIT_PAGE_ENTRIES)
                .min(MAX_APPLICATION_SESSION_AUDIT_PAGE_ENTRIES);
            let selected = indexes
                .session_by_caller
                .values()
                .copied()
                .skip(offset)
                .take(limit)
                .collect::<Vec<_>>();
            (selected, total)
        })?;
        let entries = selected
            .into_iter()
            .map(|index| {
                let record = AuthState::application_session_record(index)
                    .ok_or(ApplicationSessionStateError::InvalidSessionRecord)?;
                session_from_record(&record)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ApplicationSessionPage { entries, total })
    }

    /// Classify exact retry or conflicting reuse before expensive proof verification.
    pub fn resolve_application_replay(
        proof_fingerprint: [u8; 32],
        caller: Principal,
        subject: Principal,
        establishment_request_hash: [u8; 32],
        now_ns: u64,
    ) -> Result<ApplicationReplayResolution, ApplicationSessionStateError> {
        let session_index =
            with_indexes(|indexes| indexes.session_by_caller.get(&caller).copied())?;
        if let Some(session_index) = session_index {
            let session = session_from_record(
                &AuthState::application_session_record(session_index)
                    .ok_or(ApplicationSessionStateError::InvalidSessionRecord)?,
            )?;
            if session.proof_fingerprint() == proof_fingerprint {
                let exact = session.authenticated_subject() == subject
                    && session.establishment_request_hash() == establishment_request_hash
                    && session.authority_generation()
                        == AuthState::application_authority_generation()
                    && now_ns < session.expires_at_ns();
                return if exact {
                    Ok(ApplicationReplayResolution::ExactActive(Box::new(session)))
                } else {
                    Ok(ApplicationReplayResolution::Conflict)
                };
            }
        }

        let replay_index = with_indexes(|indexes| {
            indexes
                .replay_by_fingerprint
                .get(&proof_fingerprint)
                .copied()
        })?;
        let Some(replay_index) = replay_index else {
            return Ok(ApplicationReplayResolution::Absent);
        };

        let replay = replay_from_record(
            &AuthState::application_replay_record(replay_index)
                .ok_or(ApplicationSessionStateError::InvalidReplayRecord)?,
        )?;
        if replay.transport_caller() != caller || replay.authenticated_subject() != subject {
            return Ok(ApplicationReplayResolution::Conflict);
        }

        Ok(ApplicationReplayResolution::Conflict)
    }

    /// Return current physical occupancy used by target-local admission policy.
    pub fn application_session_occupancy(
        subject: Principal,
    ) -> Result<ApplicationSessionOccupancy, ApplicationSessionStateError> {
        with_indexes(|indexes| ApplicationSessionOccupancy {
            active_global: indexes.session_by_caller.len(),
            active_for_subject: indexes
                .session_count_by_subject
                .get(&subject)
                .copied()
                .unwrap_or(0),
            replay_global: indexes.replay_by_fingerprint.len(),
            replay_for_subject: indexes
                .replay_count_by_subject
                .get(&subject)
                .copied()
                .unwrap_or(0),
        })
    }

    /// Atomically commit one new replay tombstone and its active session replacement.
    pub fn commit_application_session(
        session: LocalApplicationSession,
        replay: LocalApplicationReplay,
    ) -> Result<ApplicationSessionCommitResult, ApplicationSessionStateError> {
        require_exact_binding(&session, &replay)?;
        let mut state = AuthState::application_authorization_state();
        if session.authority_generation() != state.authority_generation
            || replay.authority_generation() != state.authority_generation
        {
            return Err(ApplicationSessionStateError::AuthorityGenerationMismatch);
        }

        let existing_session = with_indexes(|indexes| {
            indexes
                .session_by_caller
                .get(&session.transport_caller())
                .copied()
        })?;
        let replay_exists = with_indexes(|indexes| {
            indexes
                .replay_by_fingerprint
                .contains_key(&replay.proof_fingerprint())
        })?;
        if replay_exists {
            return Err(ApplicationSessionStateError::ReplayAlreadyExists);
        }

        let capacity = Self::application_session_occupancy(session.authenticated_subject())?;
        if capacity.replay_for_subject >= MAX_APPLICATION_REPLAY_RECORDS_PER_SUBJECT {
            return Err(ApplicationSessionStateError::ReplaySubjectCapacity);
        }
        if capacity.replay_global >= MAX_APPLICATION_REPLAY_RECORDS {
            return Err(ApplicationSessionStateError::ReplayGlobalCapacity);
        }
        if existing_session.is_none()
            && capacity.active_for_subject >= MAX_ACTIVE_APPLICATION_SESSIONS_PER_SUBJECT
        {
            return Err(ApplicationSessionStateError::ActiveSubjectCapacity);
        }
        if existing_session.is_none() && capacity.active_global >= MAX_ACTIVE_APPLICATION_SESSIONS {
            return Err(ApplicationSessionStateError::ActiveGlobalCapacity);
        }

        let session_record = session_to_record(&session);
        let replay_record = replay_to_record(replay);
        let result = if let Some(index) = existing_session {
            state.sessions[index] = session_record;
            ApplicationSessionCommitResult::Replaced
        } else {
            state.sessions.push(session_record);
            ApplicationSessionCommitResult::Created
        };
        state.replays.push(replay_record);
        commit_validated_state(state)?;
        Ok(result)
    }

    /// Remove only the exact caller's retained session and preserve all replay tombstones.
    pub fn clear_application_session(
        caller: Principal,
    ) -> Result<bool, ApplicationSessionStateError> {
        let index = with_indexes(|indexes| indexes.session_by_caller.get(&caller).copied())?;
        let Some(index) = index else {
            return Ok(false);
        };
        let mut state = AuthState::application_authorization_state();
        state.sessions.remove(index);
        commit_validated_state(state)?;
        Ok(true)
    }

    /// Remove at most 128 strictly expired session and replay records.
    pub fn cleanup_application_sessions(
        now_ns: u64,
    ) -> Result<ApplicationSessionCleanupResult, ApplicationSessionStateError> {
        let mut state = AuthState::application_authorization_state();
        let mut remaining = MAX_APPLICATION_SESSION_CLEANUP_REMOVALS;
        let sessions_before = state.sessions.len();
        state.sessions.retain(|session| {
            let remove = remaining > 0 && now_ns >= session.expires_at_ns;
            if remove {
                remaining -= 1;
            }
            !remove
        });
        let replays_before = state.replays.len();
        state.replays.retain(|replay| {
            let remove = remaining > 0 && now_ns >= replay.remove_at_ns;
            if remove {
                remaining -= 1;
            }
            !remove
        });
        let result = ApplicationSessionCleanupResult {
            sessions_removed: sessions_before - state.sessions.len(),
            replays_removed: replays_before - state.replays.len(),
        };
        if result.total_removed() > 0 {
            commit_validated_state(state)?;
        }
        Ok(result)
    }

    /// Return the earliest retained session or replay expiry for native cleanup custody.
    #[must_use]
    pub fn application_session_cleanup_due_at_ns() -> Option<u64> {
        let state = AuthState::application_authorization_state();
        state
            .sessions
            .iter()
            .map(|session| session.expires_at_ns)
            .chain(state.replays.iter().map(|replay| replay.remove_at_ns))
            .min()
    }
}

fn require_exact_binding(
    session: &LocalApplicationSession,
    replay: &LocalApplicationReplay,
) -> Result<(), ApplicationSessionStateError> {
    let session_authority = ApplicationSessionReplayAuthority {
        proof_fingerprint: session.proof_fingerprint(),
        transport_caller: session.transport_caller(),
        authenticated_subject: session.authenticated_subject(),
        authority_generation: session.authority_generation(),
    };
    let replay_authority = ApplicationSessionReplayAuthority {
        proof_fingerprint: replay.proof_fingerprint(),
        transport_caller: replay.transport_caller(),
        authenticated_subject: replay.authenticated_subject(),
        authority_generation: replay.authority_generation(),
    };
    let replay_outlives_establishment = replay.remove_at_ns() > session.established_at_ns();
    if session_authority != replay_authority || !replay_outlives_establishment {
        return Err(ApplicationSessionStateError::SessionReplayMismatch);
    }
    Ok(())
}

#[derive(Eq, PartialEq)]
struct ApplicationSessionReplayAuthority {
    proof_fingerprint: [u8; 32],
    transport_caller: Principal,
    authenticated_subject: Principal,
    authority_generation: u64,
}

fn commit_validated_state(
    state: LocalApplicationAuthorizationStateData,
) -> Result<(), ApplicationSessionStateError> {
    let (indexes, _stats) = build_indexes(&state)?;
    AuthState::replace_application_authorization_state(state);
    install_indexes(indexes);
    Ok(())
}

fn install_indexes(indexes: ApplicationSessionIndexes) {
    APPLICATION_SESSION_INDEXES.with_borrow_mut(|current| *current = Some(indexes));
}

fn with_indexes<T>(
    read: impl FnOnce(&ApplicationSessionIndexes) -> T,
) -> Result<T, ApplicationSessionStateError> {
    APPLICATION_SESSION_INDEXES.with_borrow(|indexes| {
        indexes
            .as_ref()
            .map(read)
            .ok_or(ApplicationSessionStateError::IndexesUnavailable)
    })
}

fn build_indexes(
    state: &LocalApplicationAuthorizationStateData,
) -> Result<(ApplicationSessionIndexes, ApplicationSessionRestoreStats), ApplicationSessionStateError>
{
    if let Some(binding) = &state.authority_binding {
        authority_binding_from_record(binding)?;
    }
    if state.sessions.len() > MAX_ACTIVE_APPLICATION_SESSIONS {
        return Err(ApplicationSessionStateError::ActiveGlobalCapacity);
    }
    if state.replays.len() > MAX_APPLICATION_REPLAY_RECORDS {
        return Err(ApplicationSessionStateError::ReplayGlobalCapacity);
    }
    if encoded_state_len(state)? > MAX_APPLICATION_SESSION_STABLE_BYTES {
        return Err(ApplicationSessionStateError::StableStateTooLarge);
    }

    let mut indexes = ApplicationSessionIndexes::default();
    index_session_records(state, &mut indexes)?;
    index_replay_records(state, &mut indexes)?;
    validate_overlapping_session_replay_bindings(state, &indexes)?;

    indexes.estimated_bytes = estimate_index_bytes(&indexes);
    if indexes.estimated_bytes > MAX_APPLICATION_SESSION_INDEX_BYTES {
        return Err(ApplicationSessionStateError::IndexStateTooLarge);
    }
    let restore_report = ApplicationSessionRestoreStats {
        sessions: indexes.session_by_caller.len(),
        replays: indexes.replay_by_fingerprint.len(),
        session_subjects: indexes.session_count_by_subject.len(),
        replay_subjects: indexes.replay_count_by_subject.len(),
        stable_bytes: encoded_state_len(state)?,
        estimated_index_bytes: indexes.estimated_bytes,
    };
    Ok((indexes, restore_report))
}

fn index_session_records(
    state: &LocalApplicationAuthorizationStateData,
    indexes: &mut ApplicationSessionIndexes,
) -> Result<(), ApplicationSessionStateError> {
    let mut proof_fingerprints = BTreeSet::new();
    for (index, record) in state.sessions.iter().enumerate() {
        if encoded_len(record)? > MAX_APPLICATION_SESSION_RECORD_BYTES {
            return Err(ApplicationSessionStateError::SessionRecordTooLarge);
        }
        let session = session_from_record(record)?;
        if session.authority_generation() > state.authority_generation {
            return Err(ApplicationSessionStateError::FutureAuthorityGeneration);
        }
        let lifetime = session
            .expires_at_ns()
            .checked_sub(session.established_at_ns())
            .ok_or(ApplicationSessionStateError::InvalidSessionRecord)?;
        if lifetime > MAX_LOCAL_APPLICATION_SESSION_TTL_NS {
            return Err(ApplicationSessionStateError::InvalidSessionRecord);
        }
        if indexes
            .session_by_caller
            .insert(session.transport_caller(), index)
            .is_some()
        {
            return Err(ApplicationSessionStateError::DuplicateCaller);
        }
        if !proof_fingerprints.insert(session.proof_fingerprint()) {
            return Err(ApplicationSessionStateError::DuplicateProofFingerprint);
        }
        let count = indexes
            .session_count_by_subject
            .entry(session.authenticated_subject())
            .or_default();
        *count += 1;
        if *count > MAX_ACTIVE_APPLICATION_SESSIONS_PER_SUBJECT {
            return Err(ApplicationSessionStateError::ActiveSubjectCapacity);
        }
    }
    Ok(())
}

fn index_replay_records(
    state: &LocalApplicationAuthorizationStateData,
    indexes: &mut ApplicationSessionIndexes,
) -> Result<(), ApplicationSessionStateError> {
    for (index, record) in state.replays.iter().enumerate() {
        let replay = replay_from_record(record)?;
        if replay.authority_generation() > state.authority_generation {
            return Err(ApplicationSessionStateError::FutureAuthorityGeneration);
        }
        if indexes
            .replay_by_fingerprint
            .insert(replay.proof_fingerprint(), index)
            .is_some()
        {
            return Err(ApplicationSessionStateError::DuplicateProofFingerprint);
        }
        let count = indexes
            .replay_count_by_subject
            .entry(replay.authenticated_subject())
            .or_default();
        *count += 1;
        if *count > MAX_APPLICATION_REPLAY_RECORDS_PER_SUBJECT {
            return Err(ApplicationSessionStateError::ReplaySubjectCapacity);
        }
    }
    Ok(())
}

fn validate_overlapping_session_replay_bindings(
    state: &LocalApplicationAuthorizationStateData,
    indexes: &ApplicationSessionIndexes,
) -> Result<(), ApplicationSessionStateError> {
    for session in &state.sessions {
        let Some(replay_index) = indexes
            .replay_by_fingerprint
            .get(&session.proof_fingerprint)
            .copied()
        else {
            continue;
        };
        let replay = state
            .replays
            .get(replay_index)
            .ok_or(ApplicationSessionStateError::InvalidReplayRecord)?;
        let session_authority = ApplicationSessionReplayAuthority {
            proof_fingerprint: session.proof_fingerprint,
            transport_caller: session.transport_caller,
            authenticated_subject: session.authenticated_subject,
            authority_generation: session.authority_generation,
        };
        let replay_authority = ApplicationSessionReplayAuthority {
            proof_fingerprint: replay.proof_fingerprint,
            transport_caller: replay.transport_caller,
            authenticated_subject: replay.authenticated_subject,
            authority_generation: replay.authority_generation,
        };
        let replay_outlives_establishment = replay.remove_at_ns > session.established_at_ns;
        if session_authority != replay_authority || !replay_outlives_establishment {
            return Err(ApplicationSessionStateError::SessionReplayMismatch);
        }
    }
    Ok(())
}

fn session_from_record(
    record: &LocalApplicationSessionRecord,
) -> Result<LocalApplicationSession, ApplicationSessionStateError> {
    let parsed = record
        .scopes
        .iter()
        .map(|scope| ApplicationScope::parse(scope.clone()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ApplicationSessionStateError::InvalidSessionRecord)?;
    let scopes = CanonicalApplicationScopes::for_session(parsed)
        .map_err(|_| ApplicationSessionStateError::InvalidSessionRecord)?;
    let canonical = scopes
        .as_slice()
        .iter()
        .map(ApplicationScope::as_str)
        .eq(record.scopes.iter().map(String::as_str));
    if !canonical {
        return Err(ApplicationSessionStateError::InvalidSessionRecord);
    }
    LocalApplicationSession::new(
        record.transport_caller,
        record.authenticated_subject,
        record.issuer,
        record.fleet,
        record.role.clone(),
        scopes,
        record.authority_generation,
        record.established_at_ns,
        record.expires_at_ns,
        record.proof_fingerprint,
        record.establishment_request_hash,
    )
    .map_err(|_| ApplicationSessionStateError::InvalidSessionRecord)
}

fn session_to_record(session: &LocalApplicationSession) -> LocalApplicationSessionRecord {
    LocalApplicationSessionRecord {
        transport_caller: session.transport_caller(),
        authenticated_subject: session.authenticated_subject(),
        issuer: session.issuer(),
        fleet: session.fleet(),
        role: session.role().clone(),
        scopes: session
            .scopes()
            .as_slice()
            .iter()
            .map(|scope| scope.as_str().to_string())
            .collect(),
        authority_generation: session.authority_generation(),
        established_at_ns: session.established_at_ns(),
        expires_at_ns: session.expires_at_ns(),
        proof_fingerprint: session.proof_fingerprint(),
        establishment_request_hash: session.establishment_request_hash(),
    }
}

fn authority_binding_from_record(
    record: &LocalApplicationAuthorityBindingRecord,
) -> Result<LocalApplicationAuthorityBinding, ApplicationSessionStateError> {
    match record {
        LocalApplicationAuthorityBindingRecord::Disabled => {
            Ok(LocalApplicationAuthorityBinding::Disabled)
        }
        LocalApplicationAuthorityBindingRecord::Enabled {
            fleet,
            role,
            verifier_root_canister_id,
            minimum_accepted_registry_epoch,
            allowed_scopes,
            maximum_session_ttl_secs,
        } => {
            let parsed = allowed_scopes
                .iter()
                .map(|scope| ApplicationScope::parse(scope.clone()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| ApplicationSessionStateError::InvalidAuthorityBinding)?;
            let scopes = CanonicalApplicationScopes::for_verified_grant(parsed)
                .map_err(|_| ApplicationSessionStateError::InvalidAuthorityBinding)?;
            let canonical = scopes
                .as_slice()
                .iter()
                .map(ApplicationScope::as_str)
                .eq(allowed_scopes.iter().map(String::as_str));
            let maximum_ttl_ns = maximum_session_ttl_secs
                .checked_mul(1_000_000_000)
                .ok_or(ApplicationSessionStateError::InvalidAuthorityBinding)?;
            if !canonical
                || *maximum_session_ttl_secs == 0
                || maximum_ttl_ns > MAX_LOCAL_APPLICATION_SESSION_TTL_NS
            {
                return Err(ApplicationSessionStateError::InvalidAuthorityBinding);
            }
            Ok(LocalApplicationAuthorityBinding::enabled(
                *fleet,
                role.clone(),
                *verifier_root_canister_id,
                *minimum_accepted_registry_epoch,
                scopes,
                *maximum_session_ttl_secs,
            ))
        }
    }
}

fn authority_binding_to_record(
    binding: &LocalApplicationAuthorityBinding,
) -> LocalApplicationAuthorityBindingRecord {
    match binding {
        LocalApplicationAuthorityBinding::Disabled => {
            LocalApplicationAuthorityBindingRecord::Disabled
        }
        LocalApplicationAuthorityBinding::Enabled {
            fleet,
            role,
            verifier_root_canister_id,
            minimum_accepted_registry_epoch,
            allowed_scopes,
            maximum_session_ttl_secs,
        } => LocalApplicationAuthorityBindingRecord::Enabled {
            fleet: *fleet,
            role: role.clone(),
            verifier_root_canister_id: *verifier_root_canister_id,
            minimum_accepted_registry_epoch: *minimum_accepted_registry_epoch,
            allowed_scopes: allowed_scopes
                .as_slice()
                .iter()
                .map(ToString::to_string)
                .collect(),
            maximum_session_ttl_secs: *maximum_session_ttl_secs,
        },
    }
}

fn replay_from_record(
    record: &LocalApplicationReplayRecord,
) -> Result<LocalApplicationReplay, ApplicationSessionStateError> {
    LocalApplicationReplay::new(
        record.proof_fingerprint,
        record.transport_caller,
        record.authenticated_subject,
        record.authority_generation,
        record.remove_at_ns,
    )
    .map_err(|_| ApplicationSessionStateError::InvalidReplayRecord)
}

const fn replay_to_record(replay: LocalApplicationReplay) -> LocalApplicationReplayRecord {
    LocalApplicationReplayRecord {
        proof_fingerprint: replay.proof_fingerprint(),
        transport_caller: replay.transport_caller(),
        authenticated_subject: replay.authenticated_subject(),
        authority_generation: replay.authority_generation(),
        remove_at_ns: replay.remove_at_ns(),
    }
}

fn encoded_state_len(
    state: &LocalApplicationAuthorizationStateData,
) -> Result<usize, ApplicationSessionStateError> {
    encoded_len(&(
        &state.sessions,
        &state.replays,
        state.authority_generation,
        &state.authority_binding,
    ))
}

fn encoded_len(value: &impl serde::Serialize) -> Result<usize, ApplicationSessionStateError> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(value, &mut bytes)
        .map_err(|_| ApplicationSessionStateError::EncodingFailed)?;
    Ok(bytes.len())
}

fn estimate_index_bytes(indexes: &ApplicationSessionIndexes) -> usize {
    const BTREE_ENTRY_OVERHEAD: usize = 128;
    indexes.session_by_caller.len() * (29 + size_of::<usize>() + BTREE_ENTRY_OVERHEAD)
        + indexes.replay_by_fingerprint.len() * (32 + size_of::<usize>() + BTREE_ENTRY_OVERHEAD)
        + indexes.session_count_by_subject.len() * (29 + size_of::<usize>() + BTREE_ENTRY_OVERHEAD)
        + indexes.replay_count_by_subject.len() * (29 + size_of::<usize>() + BTREE_ENTRY_OVERHEAD)
}

/// Test-only owner for resetting and restoring application authorization state.
#[cfg(test)]
pub struct ApplicationSessionTestStateGuard(crate::storage::stable::auth::AuthStateData);

#[cfg(test)]
impl ApplicationSessionTestStateGuard {
    /// Replace application authorization state with one empty current-format value.
    pub fn empty() -> Self {
        let original = AuthState::export();
        AuthState::import(crate::storage::stable::auth::AuthStateData::default());
        invalidate_indexes();
        AuthStateOps::restore_application_session_state().unwrap();
        Self(original)
    }

    /// Install one replay record for a workflow test without exposing stable records upward.
    #[expect(
        clippy::unused_self,
        reason = "the receiver proves the scoped state-restoration guard is held"
    )]
    pub fn install_replay(
        &self,
        proof_fingerprint: [u8; 32],
        caller: Principal,
        remove_at_ns: u64,
    ) {
        AuthState::replace_application_authorization_state(
            LocalApplicationAuthorizationStateData {
                replays: vec![LocalApplicationReplayRecord {
                    proof_fingerprint,
                    transport_caller: caller,
                    authenticated_subject: caller,
                    authority_generation: 0,
                    remove_at_ns,
                }],
                ..LocalApplicationAuthorizationStateData::default()
            },
        );
        invalidate_indexes();
        AuthStateOps::restore_application_session_state().unwrap();
    }
}

#[cfg(test)]
impl Drop for ApplicationSessionTestStateGuard {
    fn drop(&mut self) {
        AuthState::import(self.0.clone());
        invalidate_indexes();
        AuthStateOps::restore_application_session_state().unwrap();
    }
}

#[cfg(test)]
pub fn invalidate_indexes() {
    APPLICATION_SESSION_INDEXES.with_borrow_mut(|indexes| *indexes = None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ids::CanisterRole,
        model::auth::application_authorization::ApplicationScope,
        storage::stable::auth::AuthStateData,
        test::{seams, support::fleet_key},
    };

    struct StateGuard(AuthStateData);

    impl StateGuard {
        fn empty() -> Self {
            let original = AuthState::export();
            AuthState::import(AuthStateData::default());
            invalidate_indexes();
            AuthStateOps::restore_application_session_state().unwrap();
            Self(original)
        }
    }

    impl Drop for StateGuard {
        fn drop(&mut self) {
            AuthState::import(self.0.clone());
            invalidate_indexes();
            AuthStateOps::restore_application_session_state().unwrap();
        }
    }

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn session(caller: u8, proof: u8, request: u8, established: u64) -> LocalApplicationSession {
        let scopes = CanonicalApplicationScopes::for_session(vec![
            ApplicationScope::parse("app:read").unwrap(),
        ])
        .unwrap();
        LocalApplicationSession::new(
            p(caller),
            p(caller),
            p(99),
            fleet_key(1),
            CanisterRole::new("component"),
            scopes,
            0,
            established,
            established + 1_800_000_000_000,
            [proof; 32],
            [request; 32],
        )
        .unwrap()
    }

    fn replay(caller: u8, proof: u8, remove_at: u64) -> LocalApplicationReplay {
        LocalApplicationReplay::new([proof; 32], p(caller), p(caller), 0, remove_at).unwrap()
    }

    fn maximum_state_principal(id: u32) -> Principal {
        let mut bytes = [0_u8; 29];
        bytes[..4].copy_from_slice(&id.to_be_bytes());
        Principal::from_slice(&bytes)
    }

    fn maximum_state_fingerprint(id: u32) -> [u8; 32] {
        let mut fingerprint = [0_u8; 32];
        fingerprint[..4].copy_from_slice(&id.to_be_bytes());
        fingerprint
    }

    fn maximum_state_scopes() -> Vec<String> {
        (0..16)
            .map(|scope| format!("app{scope:02}:{}", "x".repeat(58)))
            .collect()
    }

    fn binding(scopes: &[&str], maximum_session_ttl_secs: u64) -> LocalApplicationAuthorityBinding {
        let scopes = CanonicalApplicationScopes::for_verified_grant(
            scopes
                .iter()
                .map(|scope| ApplicationScope::parse(*scope).unwrap())
                .collect(),
        )
        .unwrap();
        LocalApplicationAuthorityBinding::enabled(
            fleet_key(1),
            CanisterRole::new("component"),
            p(9),
            Some(4),
            scopes,
            maximum_session_ttl_secs,
        )
    }

    #[test]
    fn proof_expiry_does_not_expire_the_committed_session_or_exact_retry() {
        let _lock = seams::lock();
        let _state = StateGuard::empty();
        let current = session(1, 2, 3, 10);
        assert_eq!(
            AuthStateOps::commit_application_session(current.clone(), replay(1, 2, 70)),
            Ok(ApplicationSessionCommitResult::Created)
        );

        assert_eq!(
            AuthStateOps::application_session(p(1)),
            Ok(Some(current.clone()))
        );
        assert_eq!(
            AuthStateOps::resolve_application_replay([2; 32], p(1), p(1), [3; 32], 80),
            Ok(ApplicationReplayResolution::ExactActive(Box::new(
                current.clone()
            )))
        );
        assert_eq!(
            AuthStateOps::cleanup_application_sessions(80),
            Ok(ApplicationSessionCleanupResult {
                sessions_removed: 0,
                replays_removed: 1,
            })
        );
        assert_eq!(
            AuthStateOps::application_session_cleanup_due_at_ns(),
            Some(current.expires_at_ns())
        );
        invalidate_indexes();
        assert_eq!(
            AuthStateOps::restore_application_session_state()
                .map(|report| (report.sessions, report.replays)),
            Ok((1, 0))
        );
        assert_eq!(
            AuthStateOps::application_session(p(1)),
            Ok(Some(current.clone()))
        );
        assert_eq!(
            AuthStateOps::resolve_application_replay([2; 32], p(1), p(1), [3; 32], 80),
            Ok(ApplicationReplayResolution::ExactActive(Box::new(
                current.clone()
            )))
        );
        assert_eq!(
            AuthStateOps::cleanup_application_sessions(current.expires_at_ns()),
            Ok(ApplicationSessionCleanupResult {
                sessions_removed: 1,
                replays_removed: 0,
            })
        );
        assert_eq!(AuthStateOps::application_session_cleanup_due_at_ns(), None);
    }

    #[test]
    fn clear_retains_the_consumed_proof_tombstone() {
        let _lock = seams::lock();
        let _state = StateGuard::empty();
        AuthStateOps::commit_application_session(session(1, 2, 3, 10), replay(1, 2, 70)).unwrap();

        assert_eq!(AuthStateOps::clear_application_session(p(1)), Ok(true));
        assert_eq!(AuthStateOps::application_session(p(1)), Ok(None));
        assert_eq!(
            AuthStateOps::resolve_application_replay([2; 32], p(1), p(1), [3; 32], 20),
            Ok(ApplicationReplayResolution::Conflict)
        );
    }

    #[test]
    fn different_proof_atomically_replaces_without_erasing_old_replay() {
        let _lock = seams::lock();
        let _state = StateGuard::empty();
        AuthStateOps::commit_application_session(session(1, 2, 3, 10), replay(1, 2, 70)).unwrap();
        let replacement = session(1, 4, 5, 20);
        assert_eq!(
            AuthStateOps::commit_application_session(replacement.clone(), replay(1, 4, 80)),
            Ok(ApplicationSessionCommitResult::Replaced)
        );
        assert_eq!(
            AuthStateOps::application_session(p(1)),
            Ok(Some(replacement))
        );
        assert_eq!(
            AuthStateOps::resolve_application_replay([2; 32], p(1), p(1), [3; 32], 30),
            Ok(ApplicationReplayResolution::Conflict)
        );
        assert_eq!(
            AuthStateOps::application_session_occupancy(p(1))
                .unwrap()
                .replay_global,
            2
        );
    }

    #[test]
    fn same_release_restore_rebuilds_indexes_from_canonical_records() {
        let _lock = seams::lock();
        let _state = StateGuard::empty();
        let current = session(1, 2, 3, 10);
        AuthStateOps::commit_application_session(current.clone(), replay(1, 2, 70)).unwrap();
        invalidate_indexes();
        assert_eq!(
            AuthStateOps::application_session(p(1)),
            Err(ApplicationSessionStateError::IndexesUnavailable)
        );
        let stats = AuthStateOps::restore_application_session_state().unwrap();
        assert_eq!(stats.sessions, 1);
        assert_eq!(stats.replays, 1);
        assert_eq!(AuthStateOps::application_session(p(1)), Ok(Some(current)));
    }

    #[test]
    fn operator_page_is_caller_ordered_bounded_and_read_only() {
        let _lock = seams::lock();
        let _state = StateGuard::empty();
        for caller in [3, 1, 2] {
            AuthStateOps::commit_application_session(
                session(caller, caller, caller, 10),
                replay(caller, caller, 70),
            )
            .unwrap();
        }

        let page = AuthStateOps::application_session_page(1, 2).unwrap();
        assert_eq!(page.total, 3);
        assert_eq!(page.entries.len(), 2);
        assert_eq!(page.entries[0].transport_caller(), p(2));
        assert_eq!(page.entries[1].transport_caller(), p(3));
        assert_eq!(
            AuthStateOps::application_session_occupancy(p(1))
                .unwrap()
                .active_global,
            3
        );

        let empty = AuthStateOps::application_session_page(0, 0).unwrap();
        assert_eq!(empty.total, 3);
        assert!(empty.entries.is_empty());
    }

    #[test]
    fn authority_binding_mutations_preserve_or_advance_generation_explicitly() {
        let _lock = seams::lock();
        let _state = StateGuard::empty();
        AuthStateOps::commit_application_session(session(1, 2, 3, 10), replay(1, 2, 70)).unwrap();
        let original = binding(&["app:read", "app:write"], 900);
        assert_eq!(AuthStateOps::application_authority_binding(), Ok(None));
        AuthStateOps::set_application_authority_binding(original.clone()).unwrap();
        assert_eq!(
            AuthStateOps::application_authority_binding(),
            Ok(Some(original))
        );
        assert_eq!(AuthStateOps::application_authority_generation(), 0);
        let narrowed = binding(&["app:read"], 900);
        AuthStateOps::advance_application_authority_binding_generation(narrowed.clone()).unwrap();
        assert_eq!(
            AuthStateOps::application_authority_binding(),
            Ok(Some(narrowed.clone()))
        );
        assert_eq!(AuthStateOps::application_authority_generation(), 1);
        AuthStateOps::set_application_authority_binding(narrowed).unwrap();
        assert_eq!(AuthStateOps::application_authority_generation(), 1);
        assert_eq!(
            AuthStateOps::resolve_application_replay([2; 32], p(1), p(1), [3; 32], 20),
            Ok(ApplicationReplayResolution::Conflict)
        );
    }

    #[test]
    fn authority_binding_generation_overflow_preserves_the_previous_binding() {
        let _lock = seams::lock();
        let _state = StateGuard::empty();
        let original = binding(&["app:read", "app:write"], 900);
        AuthState::replace_application_authorization_state(
            LocalApplicationAuthorizationStateData {
                authority_generation: u64::MAX,
                authority_binding: Some(authority_binding_to_record(&original)),
                ..LocalApplicationAuthorizationStateData::default()
            },
        );
        invalidate_indexes();
        AuthStateOps::restore_application_session_state().unwrap();

        assert_eq!(
            AuthStateOps::advance_application_authority_binding_generation(binding(
                &["app:read"],
                900,
            )),
            Err(ApplicationSessionStateError::AuthorityGenerationExhausted)
        );
        let retained = AuthState::application_authorization_state();
        assert_eq!(retained.authority_generation, u64::MAX);
        assert_eq!(
            retained.authority_binding,
            Some(authority_binding_to_record(&original))
        );
    }

    #[test]
    fn restore_rejects_noncanonical_scope_order_and_duplicate_callers() {
        let _lock = seams::lock();
        let _state = StateGuard::empty();
        let session_record = session_to_record(&session(1, 2, 3, 10));
        let replay = replay_to_record(replay(1, 2, 70));
        let mut noncanonical = session_record.clone();
        noncanonical.scopes = vec!["app:write".to_string(), "app:read".to_string()];
        AuthState::replace_application_authorization_state(
            LocalApplicationAuthorizationStateData {
                sessions: vec![noncanonical],
                replays: vec![replay],
                authority_generation: 0,
                authority_binding: None,
            },
        );
        invalidate_indexes();
        assert_eq!(
            AuthStateOps::restore_application_session_state(),
            Err(ApplicationSessionStateError::InvalidSessionRecord)
        );

        AuthState::replace_application_authorization_state(
            LocalApplicationAuthorizationStateData {
                sessions: vec![session_record.clone(), session_record],
                replays: vec![replay],
                authority_generation: 0,
                authority_binding: None,
            },
        );
        invalidate_indexes();
        assert_eq!(
            AuthStateOps::restore_application_session_state(),
            Err(ApplicationSessionStateError::DuplicateCaller)
        );

        AuthState::replace_application_authorization_state(
            LocalApplicationAuthorizationStateData {
                sessions: vec![
                    session_to_record(&session(1, 2, 3, 10)),
                    session_to_record(&session(2, 2, 4, 10)),
                ],
                replays: Vec::new(),
                authority_generation: 0,
                authority_binding: None,
            },
        );
        invalidate_indexes();
        assert_eq!(
            AuthStateOps::restore_application_session_state(),
            Err(ApplicationSessionStateError::DuplicateProofFingerprint)
        );
    }

    #[test]
    fn cleanup_is_strictly_expired_and_bounded_to_128_records() {
        let _lock = seams::lock();
        let _state = StateGuard::empty();
        let mut state = LocalApplicationAuthorizationStateData::default();
        for id in 0..129_u16 {
            let caller = Principal::from_slice(&id.to_be_bytes());
            let mut fingerprint = [0_u8; 32];
            fingerprint[..2].copy_from_slice(&id.to_be_bytes());
            state.replays.push(LocalApplicationReplayRecord {
                proof_fingerprint: fingerprint,
                transport_caller: caller,
                authenticated_subject: caller,
                authority_generation: 0,
                remove_at_ns: 10,
            });
        }
        AuthState::replace_application_authorization_state(state);
        invalidate_indexes();
        AuthStateOps::restore_application_session_state().unwrap();

        assert_eq!(
            AuthStateOps::cleanup_application_sessions(10).unwrap(),
            ApplicationSessionCleanupResult {
                sessions_removed: 0,
                replays_removed: 128,
            }
        );
        assert_eq!(
            AuthStateOps::application_session_occupancy(p(1))
                .unwrap()
                .replay_global,
            1
        );
    }

    #[test]
    fn replay_subject_capacity_rejects_without_mutating_existing_state() {
        let _lock = seams::lock();
        let _state = StateGuard::empty();
        let mut state = LocalApplicationAuthorizationStateData::default();
        let retained = session(1, 0, 3, 10);
        state.sessions.push(session_to_record(&retained));
        for id in 0..256_u16 {
            let mut fingerprint = [0_u8; 32];
            fingerprint[..2].copy_from_slice(&id.to_be_bytes());
            state.replays.push(LocalApplicationReplayRecord {
                proof_fingerprint: fingerprint,
                transport_caller: p(1),
                authenticated_subject: p(1),
                authority_generation: 0,
                remove_at_ns: 100,
            });
        }
        AuthState::replace_application_authorization_state(state);
        invalidate_indexes();
        AuthStateOps::restore_application_session_state().unwrap();

        assert_eq!(
            AuthStateOps::commit_application_session(session(1, 9, 8, 10), replay(1, 9, 70)),
            Err(ApplicationSessionStateError::ReplaySubjectCapacity)
        );
        assert_eq!(AuthStateOps::application_session(p(1)), Ok(Some(retained)));
        let capacity = AuthStateOps::application_session_occupancy(p(1)).unwrap();
        assert_eq!(capacity.replay_global, 256);
        assert_eq!(capacity.replay_for_subject, 256);
    }

    #[test]
    fn restore_rejects_global_over_capacity_before_admitting_records() {
        let _lock = seams::lock();
        let _state = StateGuard::empty();
        let invalid_session = session_to_record(&session(1, 2, 3, 10));
        AuthState::replace_application_authorization_state(
            LocalApplicationAuthorizationStateData {
                sessions: vec![invalid_session; MAX_ACTIVE_APPLICATION_SESSIONS + 1],
                ..LocalApplicationAuthorizationStateData::default()
            },
        );
        invalidate_indexes();
        assert_eq!(
            AuthStateOps::restore_application_session_state(),
            Err(ApplicationSessionStateError::ActiveGlobalCapacity)
        );

        AuthState::replace_application_authorization_state(
            LocalApplicationAuthorizationStateData {
                replays: vec![
                    replay_to_record(replay(1, 2, 70));
                    MAX_APPLICATION_REPLAY_RECORDS + 1
                ],
                ..LocalApplicationAuthorizationStateData::default()
            },
        );
        invalidate_indexes();
        assert_eq!(
            AuthStateOps::restore_application_session_state(),
            Err(ApplicationSessionStateError::ReplayGlobalCapacity)
        );
    }

    #[test]
    fn maximum_admitted_state_reconstructs_within_stable_and_heap_bounds() {
        let _lock = seams::lock();
        let _state = StateGuard::empty();
        let sessions = (0..u32::try_from(MAX_ACTIVE_APPLICATION_SESSIONS).unwrap())
            .map(|id| LocalApplicationSessionRecord {
                transport_caller: maximum_state_principal(id),
                authenticated_subject: maximum_state_principal(id),
                issuer: maximum_state_principal(9_000),
                fleet: fleet_key(1),
                role: CanisterRole::new("component"),
                scopes: maximum_state_scopes(),
                authority_generation: 0,
                established_at_ns: 1,
                expires_at_ns: MAX_LOCAL_APPLICATION_SESSION_TTL_NS + 1,
                proof_fingerprint: maximum_state_fingerprint(id),
                establishment_request_hash: [2; 32],
            })
            .collect();
        let replays = (0..u32::try_from(MAX_APPLICATION_REPLAY_RECORDS).unwrap())
            .map(|id| LocalApplicationReplayRecord {
                proof_fingerprint: maximum_state_fingerprint(id),
                transport_caller: maximum_state_principal(id),
                authenticated_subject: maximum_state_principal(id),
                authority_generation: 0,
                remove_at_ns: 60_000_000_001,
            })
            .collect();
        AuthState::replace_application_authorization_state(
            LocalApplicationAuthorizationStateData {
                sessions,
                replays,
                authority_generation: 0,
                authority_binding: Some(authority_binding_to_record(&binding(
                    &maximum_state_scopes()
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
                    1_800,
                ))),
            },
        );
        invalidate_indexes();

        let stats = AuthStateOps::restore_application_session_state().unwrap();
        assert_eq!(stats.sessions, MAX_ACTIVE_APPLICATION_SESSIONS);
        assert_eq!(stats.replays, MAX_APPLICATION_REPLAY_RECORDS);
        assert_eq!(stats.session_subjects, MAX_ACTIVE_APPLICATION_SESSIONS);
        assert_eq!(stats.replay_subjects, MAX_APPLICATION_REPLAY_RECORDS);
        assert_eq!(stats.stable_bytes, 4_025_139);
        assert_eq!(stats.estimated_index_bytes, 2_039_808);
        assert!(stats.stable_bytes <= MAX_APPLICATION_SESSION_STABLE_BYTES);
        assert!(stats.estimated_index_bytes <= MAX_APPLICATION_SESSION_INDEX_BYTES);
    }
}
