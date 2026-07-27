//! Module: ops::fleet_coordinator
//!
//! Responsibility: validate, compile, commit, and read Fleet Coordinator Registry state.
//! Does not own: endpoint authorization, multi-step lifecycle orchestration, or root effects.
//! Boundary: workflow supplies protected init facts and receives canonical Registry projections.

use crate::{
    dto::fleet_coordinator::FleetCoordinatorInitArgs,
    storage::stable::fleet_coordinator::{
        FleetCoordinatorCommitError, FleetCoordinatorCommitOutcome, FleetCoordinatorRegistryRecord,
        FleetCoordinatorRegistryStore, FleetSubnetRootJoinReceiptRecord,
    },
};
use candid::Principal;
use canic_core::{
    control_plane_support::{
        error::{InternalError, InternalErrorOrigin},
        ops::fleet_registry::FleetRegistryOps,
    },
    dto::fleet_registry::{
        FleetRegistry, FleetRegistryManifest, FleetRegistryVersion, FleetSubnetRootEntry,
        FleetSubnetRootJoinRequest, FleetSubnetRootJoinResponse, FleetSubnetRootStatus,
    },
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
            root_join_receipts: Vec::new(),
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

    pub(crate) fn join_root(
        request: FleetSubnetRootJoinRequest,
    ) -> Result<FleetSubnetRootJoinResponse, InternalError> {
        let current = Self::current()?;
        if let Some(receipt) = current.root_join_receipts.iter().find(|receipt| {
            receipt.entry.placement_subnet == request.entry.placement_subnet
                || receipt.entry.fleet_subnet_root == request.entry.fleet_subnet_root
        }) {
            if receipt.entry == request.entry {
                return Ok(FleetSubnetRootJoinResponse {
                    entry: receipt.entry.clone(),
                    version: receipt.version.clone(),
                });
            }
            return Err(InternalError::conflict(
                "Fleet Subnet Root join identity already has different protected authority",
            ));
        }

        let current_version = FleetRegistryOps::version(
            &current.authority,
            &current.component_topology,
            &current.registry,
        )?;
        if request.expected_registry != current_version {
            return Err(InternalError::conflict(
                "Fleet Subnet Root join expected Registry version is stale",
            ));
        }
        let next_registry = FleetRegistryOps::compile_joining(
            &current.authority,
            &current.component_topology,
            &current.registry,
            request.entry.clone(),
        )?;
        if next_registry == current.registry {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "Fleet Registry contains a root without its durable join receipt",
            ));
        }
        let version = FleetRegistryOps::version(
            &current.authority,
            &current.component_topology,
            &next_registry,
        )?;
        let mut next = current.clone();
        next.registry = next_registry;
        next.root_join_receipts
            .push(FleetSubnetRootJoinReceiptRecord {
                entry: request.entry.clone(),
                version: version.clone(),
            });
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(FleetSubnetRootJoinResponse {
            entry: request.entry,
            version,
        })
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
        validate_root_join_receipts(&current)?;
        Ok(current)
    }

    fn commit_transition(
        current: &FleetCoordinatorRegistryRecord,
        next: FleetCoordinatorRegistryRecord,
    ) -> Result<FleetCoordinatorCommitOutcome, InternalError> {
        FleetCoordinatorRegistryStore::commit_transition(current, next).map_err(|error| match error
        {
            FleetCoordinatorCommitError::ConflictingState => InternalError::conflict(
                "Fleet Coordinator Registry changed before the requested transition committed",
            ),
            FleetCoordinatorCommitError::Uninitialized => {
                InternalError::unavailable("Fleet Coordinator genesis is not initialized")
            }
        })
    }
}

fn validate_root_join_receipts(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    if current.root_join_receipts.len() != current.registry.fleet_subnet_roots.len() {
        return Err(receipt_invariant(
            "Fleet Registry root rows and durable join receipts differ",
        ));
    }

    let mut historical_registry = FleetRegistryOps::compile_genesis(
        &current.configured_app,
        current.authority.clone(),
        &current.component_topology,
    )
    .map_err(|_| receipt_invariant("Fleet Registry join receipt genesis is not canonical"))?;
    for receipt in &current.root_join_receipts {
        if receipt.entry.status != FleetSubnetRootStatus::Joining {
            return Err(receipt_invariant(
                "Fleet Registry join receipt does not retain its original Joining row",
            ));
        }
        historical_registry = FleetRegistryOps::compile_joining(
            &current.authority,
            &current.component_topology,
            &historical_registry,
            receipt.entry.clone(),
        )
        .map_err(|_| receipt_invariant("Fleet Registry join receipt history is not canonical"))?;
        let historical_version = FleetRegistryOps::version(
            &current.authority,
            &current.component_topology,
            &historical_registry,
        )
        .map_err(|_| receipt_invariant("Fleet Registry join receipt version cannot be derived"))?;
        if receipt.version != historical_version {
            return Err(receipt_invariant(
                "Fleet Registry join receipt version differs from its historical snapshot",
            ));
        }

        let matching = current
            .registry
            .fleet_subnet_roots
            .iter()
            .filter(|entry| {
                entry.placement_subnet == receipt.entry.placement_subnet
                    || entry.fleet_subnet_root == receipt.entry.fleet_subnet_root
            })
            .collect::<Vec<_>>();
        if matching.len() != 1 || !same_root_authority(matching[0], &receipt.entry) {
            return Err(receipt_invariant(
                "Fleet Registry join receipt differs from the current root authority",
            ));
        }
    }
    Ok(())
}

fn same_root_authority(left: &FleetSubnetRootEntry, right: &FleetSubnetRootEntry) -> bool {
    left.placement_subnet == right.placement_subnet
        && left.fleet_subnet_root == right.fleet_subnet_root
        && left.component_admissions == right.component_admissions
        && left.component_topology_digest == right.component_topology_digest
        && left.active_release_set == right.active_release_set
        && left.limits == right.limits
}

fn receipt_invariant(message: &'static str) -> InternalError {
    InternalError::invariant(InternalErrorOrigin::Storage, message)
}
