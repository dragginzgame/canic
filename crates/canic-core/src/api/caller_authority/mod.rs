//! Module: api::caller_authority
//!
//! Facade for protected receiver publication and receipt observation.

use crate::{
    dto::{
        caller_authority::{CallerAuthorityCommand, CallerAuthorityReceipt, CallerAuthorityStatus},
        error::Error,
    },
    workflow::caller_authority,
};

/// Managed endpoint adapter; endpoint guards authenticate the owning Root.
pub struct CallerAuthorityApi;

impl CallerAuthorityApi {
    /// Build the exact bounded request persisted by an issuing Root's membership operation.
    pub fn publication(
        authority: crate::ids::CallerReceiverAuthority,
        operation_id: [u8; 32],
        previous_generation: u64,
        change: crate::dto::caller_authority::CallerAuthorityChange,
    ) -> Result<crate::dto::caller_authority::CallerAuthorityPublication, Error> {
        caller_authority::publication(authority, operation_id, previous_generation, change)
            .map_err(Error::from)
    }
    pub fn apply(command: CallerAuthorityCommand) -> Result<CallerAuthorityReceipt, Error> {
        caller_authority::apply(command).map_err(Error::from)
    }

    pub fn status(operation_id: [u8; 32]) -> Result<CallerAuthorityStatus, Error> {
        caller_authority::status(operation_id).map_err(Error::from)
    }
}
