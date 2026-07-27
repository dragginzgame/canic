//! Module: ops::fleet_coordinator
//!
//! Responsibility: validate, compile, commit, and read Fleet Coordinator Registry state.
//! Does not own: endpoint authorization, multi-step lifecycle orchestration, or root effects.
//! Boundary: workflow supplies protected init facts and receives canonical Registry projections.

use crate::{
    dto::fleet_coordinator::FleetCoordinatorInitArgs,
    storage::stable::fleet_coordinator::{
        FleetCoordinatorCommitOutcome, FleetCoordinatorRegistryRecord,
        FleetCoordinatorRegistryStore,
    },
};
use candid::Principal;
use canic_core::{
    control_plane_support::{
        error::{InternalError, InternalErrorOrigin},
        ops::fleet_registry::FleetRegistryOps,
    },
    dto::fleet_registry::{FleetRegistry, FleetRegistryManifest, FleetRegistryVersion},
};

///
/// FleetCoordinatorOps
///
/// Single-step Coordinator state and canonical Registry operations.
///

pub struct FleetCoordinatorOps;

impl FleetCoordinatorOps {
    pub(crate) fn compile_genesis(
        args: FleetCoordinatorInitArgs,
        coordinator_canister: Principal,
    ) -> Result<FleetCoordinatorRegistryRecord, InternalError> {
        if args.authority.binding.coordinator != coordinator_canister {
            return Err(InternalError::invalid_input(
                "Fleet Coordinator authority principal does not match the installed canister",
            ));
        }
        args.component_topology
            .canonical_bytes()
            .map_err(|error| InternalError::invalid_input(error.to_string()))?;
        let registry = FleetRegistryOps::compile_genesis(
            &args.configured_app,
            args.authority.clone(),
            &args.component_topology,
        )?;
        Ok(FleetCoordinatorRegistryRecord {
            configured_app: args.configured_app,
            authority: args.authority,
            component_topology: args.component_topology,
            registry,
        })
    }

    pub(crate) fn commit_genesis(
        record: FleetCoordinatorRegistryRecord,
    ) -> Result<FleetCoordinatorCommitOutcome, InternalError> {
        FleetCoordinatorRegistryStore::commit_genesis(record).map_err(|_| {
            InternalError::conflict(
                "Fleet Coordinator already contains different protected Registry state",
            )
        })
    }

    pub(crate) fn registry() -> Result<FleetRegistry, InternalError> {
        Ok(Self::current()?.registry)
    }

    pub(crate) fn manifest() -> Result<FleetRegistryManifest, InternalError> {
        let current = Self::current()?;
        FleetRegistryOps::manifest(
            &current.authority,
            &current.component_topology,
            &current.registry,
        )
    }

    pub(crate) fn version() -> Result<FleetRegistryVersion, InternalError> {
        let current = Self::current()?;
        FleetRegistryOps::version(
            &current.authority,
            &current.component_topology,
            &current.registry,
        )
    }

    fn current() -> Result<FleetCoordinatorRegistryRecord, InternalError> {
        FleetCoordinatorRegistryStore::export()
            .current
            .ok_or_else(|| {
                InternalError::unavailable("Fleet Coordinator genesis is not initialized")
            })
            .and_then(Self::validate_current)
    }

    fn validate_current(
        current: FleetCoordinatorRegistryRecord,
    ) -> Result<FleetCoordinatorRegistryRecord, InternalError> {
        if current.authority != current.registry.authority {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "stored Fleet Coordinator authority does not match its Fleet Registry",
            ));
        }
        if current.configured_app != current.authority.binding.fleet.app {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "stored Fleet Coordinator App does not match its authority",
            ));
        }
        FleetRegistryOps::validate(
            &current.authority,
            &current.component_topology,
            &current.registry,
        )?;
        Ok(current)
    }
}
