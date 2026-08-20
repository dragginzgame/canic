//! Module: api::runtime_whitelist
//!
//! Responsibility: map managed-role whitelist endpoint calls into workflow results.
//! Does not own: authentication, mutation policy, stable state, or Candid type generation.
//! Boundary: macro endpoints authenticate first and call this synchronous facade.

use crate::{
    dto::{
        error::Error,
        page::PageRequest,
        runtime_whitelist::{
            RuntimeWhitelistCommand, RuntimeWhitelistMutationResponse,
            RuntimeWhitelistStatusResponse,
        },
    },
    workflow::runtime_whitelist::RuntimeWhitelistWorkflow,
};

/// Synchronous managed-role runtime-whitelist facade.
pub struct RuntimeWhitelistApi;

impl RuntimeWhitelistApi {
    pub fn command(
        command: RuntimeWhitelistCommand,
    ) -> Result<RuntimeWhitelistMutationResponse, Error> {
        RuntimeWhitelistWorkflow::command(command).map_err(Into::into)
    }

    pub fn status(request: PageRequest) -> Result<RuntimeWhitelistStatusResponse, Error> {
        RuntimeWhitelistWorkflow::status(request).map_err(Into::into)
    }
}
