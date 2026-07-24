//! Module: workflow::runtime::fleet_activation
//!
//! Responsibility: expose the canonical protected Fleet activation status.
//! Does not own: stable conversion, endpoint authorization, or activation mutation.
//! Boundary: the runtime role selects root-only projection before ops validates the record.

use crate::{
    InternalError,
    domain::policy::pure::{PolicyError, fleet_activation::require_prepared_root_endpoint},
    dto::fleet_activation::{FleetActivationPhase, FleetActivationStatusResponse},
    ids::EndpointCall,
    ops::{
        runtime::env::EnvOps,
        storage::{StorageOpsError, fleet_activation::FleetActivationOps},
    },
};

///
/// FleetActivationWorkflow
///

pub struct FleetActivationWorkflow;

impl FleetActivationWorkflow {
    pub fn status() -> Result<FleetActivationStatusResponse, InternalError> {
        FleetActivationOps::status(EnvOps::is_root())
            .map_err(StorageOpsError::from)
            .map_err(Into::into)
    }

    pub fn require_active() -> Result<(), InternalError> {
        FleetActivationOps::require_active(EnvOps::is_root())
            .map_err(StorageOpsError::from)
            .map_err(Into::into)
    }

    /// Enforce the activation phase before a managed endpoint handler runs.
    pub fn require_endpoint_allowed(call: EndpointCall) -> Result<(), InternalError> {
        if !EnvOps::canister_role()?.is_root() {
            return Ok(());
        }
        let status = FleetActivationOps::status(true)
            .map_err(StorageOpsError::from)
            .map_err(InternalError::from)?;

        require_root_endpoint_for_phase(status.phase, call).map_err(InternalError::from)
    }
}

fn require_root_endpoint_for_phase(
    phase: FleetActivationPhase,
    call: EndpointCall,
) -> Result<(), PolicyError> {
    match phase {
        FleetActivationPhase::Prepared => {
            require_prepared_root_endpoint(call).map_err(PolicyError::from)
        }
        FleetActivationPhase::Active => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{EndpointCallKind, EndpointId};

    fn call(name: &'static str, kind: EndpointCallKind) -> EndpointCall {
        EndpointCall {
            endpoint: EndpointId::new(name),
            kind,
        }
    }

    #[test]
    fn active_admits_ordinary_handlers_but_prepared_delegates_to_exact_policy() {
        let ordinary = call("application_update", EndpointCallKind::Update);

        assert!(require_root_endpoint_for_phase(FleetActivationPhase::Active, ordinary).is_ok());
        assert!(matches!(
            require_root_endpoint_for_phase(FleetActivationPhase::Prepared, ordinary),
            Err(PolicyError::FleetActivationPolicy(_))
        ));
    }
}
