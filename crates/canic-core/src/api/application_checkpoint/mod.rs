//! Module: api::application_checkpoint
//!
//! Responsibility: expose participant invariants and exact installed-Root endpoint admission.
//! Does not own: business draining, storage, Root orchestration or IC effects.
//! Boundary: applications commit transitions before hooks and authenticate the installed Root.

use crate::access::{
    AccessError,
    expr::{AccessCheck, AccessContext},
};
use canic_contracts::ids::ApplicationCheckpointBinding;

pub use crate::model::application_checkpoint::{
    ApplicationCheckpointError, ApplicationCheckpointRecord, ApplicationCheckpointTransition,
};
pub use canic_contracts::ids::ApplicationCheckpointPhase;

/// Endpoint predicate binding the request to Canic's independently installed audience.
///
/// Root recovery authentication is independent from ordinary application admission.
/// Successful authentication proves neither application quiescence nor IC effect authority.
pub struct ApplicationCheckpointRootPredicate {
    binding: ApplicationCheckpointBinding,
}

impl ApplicationCheckpointRootPredicate {
    /// Retain the exact request audience for synchronous endpoint evaluation.
    #[must_use]
    pub const fn new(binding: ApplicationCheckpointBinding) -> Self {
        Self { binding }
    }
}

impl AccessCheck for ApplicationCheckpointRootPredicate {
    fn eval(&self, context: &AccessContext) -> Result<(), AccessError> {
        crate::workflow::application_checkpoint::authorize_root(
            &self.binding,
            context.transport_caller(),
        )
        .map_err(Into::into)
    }

    fn name(&self) -> &'static str {
        "application_checkpoint_root"
    }
}

/// Resolve an already endpoint-authenticated command for application-owned durable hooks.
pub fn binding_from_command(
    request: &canic_contracts::dto::application_checkpoint::ApplicationCheckpointCommand,
) -> Result<ApplicationCheckpointBinding, canic_contracts::dto::error::Error> {
    crate::ops::application_checkpoint::binding(request).map_err(Into::into)
}

/// Authenticate a Wire command against local installation authority before invoking hooks.
pub struct ApplicationCheckpointCommandPredicate {
    command: canic_contracts::dto::application_checkpoint::ApplicationCheckpointCommand,
}

impl ApplicationCheckpointCommandPredicate {
    /// Retain the exact command for endpoint admission.
    #[must_use]
    pub const fn new(
        command: canic_contracts::dto::application_checkpoint::ApplicationCheckpointCommand,
    ) -> Self {
        Self { command }
    }
}

impl AccessCheck for ApplicationCheckpointCommandPredicate {
    fn eval(&self, context: &AccessContext) -> Result<(), AccessError> {
        let binding = crate::ops::application_checkpoint::binding(&self.command)
            .map_err(AccessError::Internal)?;
        crate::workflow::application_checkpoint::authorize_root(
            &binding,
            context.transport_caller(),
        )
        .map_err(Into::into)
    }

    fn name(&self) -> &'static str {
        "application_checkpoint_command"
    }
}
