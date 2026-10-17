//! Check canonical login rows and protected physical occupancy during restoration.

use super::*;
use crate::model::auth::application_authorization::{
    MAX_APPLICATION_ADMITTED_LOGINS, MAX_APPLICATION_UNKNOWN_LOGOUTS,
};
use ic_auth::{portable::LoginRevocationChange, session::Session};

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
    let mut callers = BTreeSet::new();
    let sessions = state
        .sessions
        .iter()
        .map(|record| (record.transport_caller, record))
        .collect::<BTreeMap<_, _>>();
    let mut admitted = 0;
    for (caller, record) in &state.logins {
        let login = LoginRevocationChange::decode(&record.bytes).map_err(|_| invalid.clone())?;
        if login.caller() != *caller || !callers.insert(*caller) {
            return Err(invalid);
        }
        if let Some(record) = sessions.get(caller) {
            if login.revoked() {
                return Err(invalid);
            }
            let session = Session::decode(&record.shared_session).map_err(|_| invalid.clone())?;
            session
                .check_login_change(&login)
                .map_err(|_| invalid.clone())?;
        }
        admitted += usize::from(login.admitted_pool());
    }
    if admitted != state.login_totals.admitted {
        return Err(invalid);
    }
    Ok(())
}
