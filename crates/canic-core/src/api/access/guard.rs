use crate::{PublicError, access::guard};

///
/// GuardApi
///

pub struct GuardApi;

impl GuardApi {
    pub fn guard_app_query() -> Result<(), PublicError> {
        guard::guard_app_query().map_err(PublicError::from)
    }

    pub fn guard_app_update() -> Result<(), PublicError> {
        guard::guard_app_update().map_err(PublicError::from)
    }
}
