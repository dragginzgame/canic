//! Module: api::application_startup
//!
//! Lifecycle adapter for protected application release and deferred initialization.

use crate::{
    dto::{caller_authority::CallerAuthorityPublication, error::Error},
    workflow::runtime::application_startup,
};
use std::future::Future;

///
/// ApplicationStartupApi
///
/// Hidden adapter used by generated managed lifecycle endpoints.
///

#[doc(hidden)]
pub struct ApplicationStartupApi;

impl ApplicationStartupApi {
    pub fn release(publication: CallerAuthorityPublication) -> Result<(), Error> {
        application_startup::release(publication).map_err(Into::into)
    }

    /// Stop a later lifecycle effect if its retained startup release has been fenced.
    pub fn require_initialization_effect() {
        application_startup::require_initialization_effect().unwrap_or_else(|error| {
            ic_cdk::trap(format!("application initialization fenced: {error}"))
        });
    }

    /// Stop a later upgrade hook if application admission changed during setup.
    pub fn require_upgrade_effect() {
        application_startup::require_started()
            .unwrap_or_else(|error| ic_cdk::trap(format!("application upgrade fenced: {error}")));
    }

    /// Schedule pending original initialization through the retained native claim.
    pub fn schedule_required<F: Future<Output = ()> + 'static>(hook: fn(Option<Vec<u8>>) -> F) {
        application_startup::schedule(hook)
            .unwrap_or_else(|error| ic_cdk::trap(format!("application startup rejected: {error}")));
    }

    /// Application upgrade hooks are admitted only after the original initialization finished.
    #[must_use]
    pub fn is_started() -> bool {
        application_startup::require_started().is_ok()
    }
}
