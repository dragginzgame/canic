//! Check canonical login rows and protected physical occupancy during restoration.

use super::*;
use crate::model::auth::application_authorization::{
    MAX_APPLICATION_ADMITTED_LOGINS, MAX_APPLICATION_UNKNOWN_LOGOUTS,
};
use ic_auth::{
    portable::{LoginRecordState, LoginRevocationChange},
    session::Session,
};
use ic_auth_protocol_types::{AudienceId, CanonicalId};

pub(super) fn validate_login_records(
    state: &LocalApplicationAuthorizationStateData,
) -> Result<(), ApplicationSessionStateError> {
    let invalid = ApplicationSessionStateError::InvalidLoginRecord;
    if state.login_totals.admitted > MAX_APPLICATION_ADMITTED_LOGINS
        || state.login_totals.unknown > MAX_APPLICATION_UNKNOWN_LOGOUTS
    {
        return Err(invalid);
    }
    if state.logins.len() != state.login_totals.admitted + state.login_totals.unknown {
        return Err(invalid);
    }
    // Disabled authority still retains logout denials. Its rows must share one
    // audience; enabled authority additionally pins that audience to the Fleet.
    let mut audience = match &state.authority_binding {
        Some(LocalApplicationAuthorityBindingRecord::Enabled { fleet, .. }) => Some(AudienceId {
            canonical_network_id: CanonicalId::from_bytes(*fleet.canonical_network_id.as_bytes()),
            fleet_id: CanonicalId::from_bytes(*fleet.fleet_id.as_bytes()),
        }),
        _ => None,
    };
    let mut callers = BTreeSet::new();
    let sessions = state
        .sessions
        .iter()
        .map(|record| {
            Session::decode(&record.shared_session)
                .map(|session| (Principal::from(session.caller()), session))
                .map_err(|_| invalid.clone())
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let mut admitted = 0;
    for (caller, record) in &state.logins {
        let login = LoginRevocationChange::decode(&record.bytes).map_err(|_| invalid.clone())?;
        let expected_audience = audience.get_or_insert_with(|| login.audience());
        if login.audience() != *expected_audience
            || login.caller() != (*caller).into()
            || !callers.insert(*caller)
        {
            return Err(invalid);
        }
        if let Some(record) = sessions.get(caller) {
            if matches!(login.state(), LoginRecordState::Revoked(_)) {
                return Err(invalid);
            }
            record
                .check_login_change(&login)
                .map_err(|_| invalid.clone())?;
        }
        admitted += usize::from(!matches!(login.state(), LoginRecordState::Revoked(None)));
    }
    if admitted != state.login_totals.admitted
        || sessions.keys().any(|caller| !callers.contains(caller))
    {
        return Err(invalid);
    }
    Ok(())
}
