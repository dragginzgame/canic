use crate::{Error, access::guard};

///
/// GuardApi
///

pub struct GuardApi;

impl GuardApi {
    pub fn guard_app_query() -> Result<(), Error> {
        guard::guard_app_query().map_err(Error::from)
    }

    pub fn guard_app_update() -> Result<(), Error> {
        guard::guard_app_update().map_err(Error::from)
    }
}
