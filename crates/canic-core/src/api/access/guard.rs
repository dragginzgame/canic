use crate::{InternalError, access::guard, dto::error::Error};

///
/// GuardAccessApi
///

pub struct GuardAccessApi;

impl GuardAccessApi {
    pub fn guard_app_query() -> Result<(), Error> {
        guard::guard_app_query()
            .map_err(InternalError::from)
            .map_err(Error::from)
    }

    pub fn guard_app_update() -> Result<(), Error> {
        guard::guard_app_update()
            .map_err(InternalError::from)
            .map_err(Error::from)
    }
}
