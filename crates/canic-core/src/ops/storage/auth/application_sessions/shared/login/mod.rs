//! Prepare one canonical login row and protected occupancy before any stable write.

use super::*;
use crate::{
    model::auth::application_authorization::{
        MAX_APPLICATION_ADMITTED_LOGINS, MAX_APPLICATION_UNKNOWN_LOGOUTS,
    },
    storage::stable::auth::{LocalApplicationLoginRecord, LocalApplicationLoginTotalsRecord},
};
use ic_auth_protocol_types::{AudienceId, CanonicalId};

/// Validated touched row and the exact encoded accounting delta, including totals.
pub(super) struct PreparedLogin {
    pub record: LocalApplicationLoginRecord,
    pub totals: LocalApplicationLoginTotalsRecord,
    pub before_bytes: usize,
    pub after_bytes: usize,
}

pub(super) fn prepare_login(
    owner: &ApplicationSessionStore,
    change: &LoginRevocationChange,
) -> Result<PreparedLogin, SessionError> {
    let audience = AudienceId {
        canonical_network_id: CanonicalId::from_bytes(*owner.fleet.canonical_network_id.as_bytes()),
        fleet_id: CanonicalId::from_bytes(*owner.fleet.fleet_id.as_bytes()),
    };
    if change.audience() != audience {
        return Err(SessionError::InvalidPolicyOrRecord);
    }
    let mut totals = LocalApplicationAuthorizationState::application_login_totals();
    let mut before_bytes = encoded_len(&totals).map_err(|_| SessionError::StorageFailure)?;
    let previous = LocalApplicationAuthorizationState::application_login_record(change.caller());
    if let Some(previous) = previous {
        before_bytes +=
            encoded_len(&(change.caller(), &previous)).map_err(|_| SessionError::StorageFailure)?;
        let retained = LoginRevocationChange::decode(&previous.bytes)
            .map_err(|_| SessionError::StorageFailure)?;
        let same_authority = retained.audience() == change.audience()
            && retained.caller() == change.caller()
            && retained.admitted_pool() == change.admitted_pool();
        let preserves_denial = !retained.revoked() || change.revoked();
        let preserves_binding = retained.binding() == change.binding();
        if !same_authority
            || !preserves_denial
            || !preserves_binding
            || change.until_ns() < retained.until_ns()
        {
            return Err(SessionError::InvalidPolicyOrRecord);
        }
        let count = if change.admitted_pool() {
            totals.admitted
        } else {
            totals.unknown
        };
        if count == 0 {
            return Err(SessionError::StorageFailure);
        }
    } else {
        let count = if change.admitted_pool() {
            &mut totals.admitted
        } else {
            &mut totals.unknown
        };
        *count = count.checked_add(1).ok_or(SessionError::Capacity)?;
    }
    if totals.admitted > MAX_APPLICATION_ADMITTED_LOGINS
        || totals.unknown > MAX_APPLICATION_UNKNOWN_LOGOUTS
    {
        return Err(SessionError::Capacity);
    }
    let record = LocalApplicationLoginRecord {
        bytes: change
            .encode()
            .map_err(|_| SessionError::InvalidPolicyOrRecord)?,
    };
    let after_bytes = encoded_len(&(change.caller(), &record))
        .map_err(|_| SessionError::StorageFailure)?
        + encoded_len(&totals).map_err(|_| SessionError::StorageFailure)?;
    Ok(PreparedLogin {
        record,
        totals,
        before_bytes,
        after_bytes,
    })
}
