//! Commit a login tombstone and session-only removal without rewriting the ledger.

use super::*;

pub(super) fn commit_logout(caller: Principal, login: PreparedLogin) -> Result<(), SessionError> {
    let (index, count, encoded_bytes) = with_indexes(|indexes| {
        (
            indexes.session_by_caller.get(&caller).copied(),
            indexes.session_by_caller.len(),
            indexes.encoded_bytes,
        )
    })
    .map_err(|_| SessionError::StorageFailure)?;
    let session = index
        .map(|index| {
            LocalApplicationAuthorizationState::application_session_record(index)
                .ok_or(SessionError::StorageFailure)
        })
        .transpose()?;
    let removed_bytes = session
        .as_ref()
        .map(encoded_len)
        .transpose()
        .map_err(|_| SessionError::StorageFailure)?
        .unwrap_or(0);
    let moved = if index.is_some() && index != count.checked_sub(1) {
        Some(
            LocalApplicationAuthorizationState::application_session_record(count - 1)
                .ok_or(SessionError::StorageFailure)?,
        )
    } else {
        None
    };
    let encoded_bytes = encoded_bytes
        .checked_sub(removed_bytes + login.before_bytes)
        .and_then(|n| n.checked_add(login.after_bytes))
        .ok_or(SessionError::StorageFailure)?;
    if encoded_bytes > MAX_APPLICATION_SESSION_STABLE_BYTES {
        return Err(SessionError::ByteCapacity);
    }
    let canonical = session
        .as_ref()
        .map(session_from_record)
        .transpose()
        .map_err(|_| SessionError::InvalidPolicyOrRecord)?;
    let moved_caller = moved
        .as_ref()
        .map(session_from_record)
        .transpose()
        .map_err(|_| SessionError::InvalidPolicyOrRecord)?
        .map(|session| session.caller());
    // All fallible projection and capacity work precedes the synchronous IC commit.
    LocalApplicationAuthorizationState::commit_shared_logout(
        caller,
        index,
        login.record,
        login.totals,
    );
    APPLICATION_SESSION_INDEXES.with_borrow_mut(|slot| {
        let indexes = slot.as_mut().expect("admitted session indexes");
        if canonical.is_some() {
            indexes.session_by_caller.remove(&caller);
        }
        if let Some(moved) = moved_caller {
            indexes
                .session_by_caller
                .insert(moved.into(), index.expect("validated swap index"));
        }
        indexes.encoded_bytes = encoded_bytes;
        indexes.estimated_bytes = estimate_index_bytes(indexes);
    });
    Ok(())
}
