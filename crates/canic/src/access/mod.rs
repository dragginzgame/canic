//! Public access helpers re-exported from the core access layer.

pub use crate::__internal::core::access::{AccessError, AccessErrorKind, app, auth, env};

pub fn require_local() -> Result<(), crate::Error> {
    env::build_network_local().map_err(|err| crate::Error::forbidden(err.to_string()))
}
