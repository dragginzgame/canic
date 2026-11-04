//! Prepare one canonical login row and protected occupancy before any stable write.

use super::*;
use crate::storage::stable::auth::{
    LocalApplicationLoginRecord, LocalApplicationLoginTotalsRecord,
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
    let previous =
        LocalApplicationAuthorizationState::application_login_record(change.caller().into());
    if let Some(previous) = previous {
        before_bytes +=
            encoded_len(&(change.caller(), &previous)).map_err(|_| SessionError::StorageFailure)?;
        let retained = LoginRevocationChange::decode(&previous.bytes)
            .map_err(|_| SessionError::StorageFailure)?;
        change
            .check_replacement(
                &retained,
                LocalApplicationAuthorizationState::application_observed_at_ns(),
            )
            .map_err(|_| SessionError::InvalidPolicyOrRecord)?;
        let previous_unknown = matches!(retained.state(), LoginRecordState::Revoked(None));
        let next_unknown = matches!(change.state(), LoginRecordState::Revoked(None));
        let previous_count = if previous_unknown {
            &mut totals.unknown
        } else {
            &mut totals.admitted
        };
        if *previous_count == 0 {
            return Err(SessionError::StorageFailure);
        }
        if previous_unknown != next_unknown {
            *previous_count -= 1;
            let next_count = if next_unknown {
                &mut totals.unknown
            } else {
                &mut totals.admitted
            };
            *next_count = next_count.checked_add(1).ok_or(SessionError::Capacity)?;
        }
    } else {
        let count = if matches!(change.state(), LoginRecordState::Revoked(None)) {
            &mut totals.unknown
        } else {
            &mut totals.admitted
        };
        *count = count.checked_add(1).ok_or(SessionError::Capacity)?;
    }
    if totals.admitted > capacity().admitted_logins || totals.unknown > capacity().unknown_logouts {
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
