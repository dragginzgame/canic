//! Module: ops::fleet_coordinator
//!
//! Responsibility: validate, compile, commit, and read Fleet Coordinator Registry state.
//! Does not own: endpoint authorization, multi-step lifecycle orchestration, or root effects.
//! Boundary: workflow supplies protected init facts and receives canonical Registry projections.

mod deployment_ledger;
mod root_deletion;
#[expect(
    dead_code,
    reason = "B3 grant ops are staged until the accepted workflow slice wires them"
)]
mod root_funding;

use root_deletion::validate_root_deletion_history;

use crate::{
    dto::fleet_coordinator::{
        CoordinatorOperationStatusResponse, CoordinatorRootRemovalOperationStatus,
        FleetCoordinatorInitArgs,
    },
    storage::stable::fleet_coordinator::{
        FleetComponentDirectoryConfirmationIntentRecord, FleetComponentDirectoryConfirmationRecord,
        FleetComponentGroupDeploymentRecord, FleetComponentProvisioningRecord,
        FleetComponentProvisioningRootAcceptanceIntentRecord,
        FleetComponentProvisioningRootAcceptanceRecord,
        FleetComponentProvisioningRootProvisionIntentRecord,
        FleetComponentProvisioningRootProvisionRecord, FleetComponentProvisioningStateRecord,
        FleetComponentRuntimeActivationIntentRecord, FleetComponentRuntimeActivationRecord,
        FleetComponentScaleOutReceiptRecord, FleetCoordinatorCommitError,
        FleetCoordinatorCommitOutcome, FleetCoordinatorRegistryRecord,
        FleetCoordinatorRegistryStore, FleetRegistryActivationReceiptRecord,
        FleetServicePublicationReceiptRecord, FleetSubnetRootDrainingPublicationReceiptRecord,
        FleetSubnetRootDrainingReservationRecord, FleetSubnetRootJoinReceiptRecord,
        FleetSubnetRootRemovalPublicationReceiptRecord,
    },
    view::fleet_coordinator::{
        FleetComponentDirectoryConfirmationCallView,
        FleetComponentDirectoryConfirmationDisposition,
        FleetComponentProvisioningRootAcceptanceCallView,
        FleetComponentProvisioningRootAcceptanceDisposition,
        FleetComponentProvisioningRootProvisionCallView,
        FleetComponentProvisioningRootProvisionDisposition,
        FleetComponentRuntimeActivationCallView, FleetComponentRuntimeActivationDisposition,
    },
};
use std::collections::BTreeSet;

use candid::Principal;
#[cfg(test)]
use canic_core::control_plane_support::config::ConfigModel;
use canic_core::{
    control_plane_support::{
        error::InternalError,
        ops::{
            component_provisioning_plan::{
                ComponentProvisioningPlanOps, MAX_FLEET_COMPONENT_PROVISIONING_PLAN_BATCHES,
                MAX_FLEET_COMPONENT_PROVISIONING_PLAN_CONFIRMATION_ROOTS,
                MAX_FLEET_COMPONENT_PROVISIONING_PLAN_ENTRIES,
                MAX_FLEET_COMPONENT_PROVISIONING_PLAN_PLACEMENTS,
            },
            component_provisioning_receipt::{
                RootComponentProvisioningAcceptanceReceiptAuthority,
                RootComponentProvisioningPublishedReceiptAuthority,
                RootComponentProvisioningReceiptOps,
                RootComponentProvisioningRuntimesActiveReceiptAuthority,
            },
            fleet_registry::FleetRegistryOps,
            fleet_service_binding::FleetServiceBindingOps,
            root_draining_reservation::FleetSubnetRootDrainingReservationOps,
        },
    },
    dto::{
        component_provisioning::{
            FleetComponentActivationRootProgress, FleetComponentProvisioningAdvanceRequest,
            FleetComponentProvisioningOperation, FleetComponentProvisioningPhase,
            FleetComponentProvisioningPlan, FleetComponentProvisioningPrepareRequest,
            FleetComponentProvisioningRootProgress, FleetComponentProvisioningStatusRequest,
            FleetComponentProvisioningStatusResponse, FleetComponentPublicationRootProgress,
            FleetComponentSynchronizationRootProgress, FleetSubnetRootProvisioningBatch,
            RootComponentActivationRequest, RootComponentDirectorySynchronizationRequest,
            RootComponentDirectorySynchronizationResponse,
            RootComponentProvisioningAcceptanceRequest, RootComponentProvisioningAdvanceRequest,
            RootComponentProvisioningPhase, RootComponentProvisioningStatusResponse,
            RootComponentPublicationRequest,
        },
        fleet_registry::{
            FleetRegistry, FleetRegistryActivationRequest, FleetRegistryActivationResponse,
            FleetRegistryManifest, FleetRegistryVersion, FleetSubnetRootDrainingPublicationRequest,
            FleetSubnetRootDrainingPublicationResponse, FleetSubnetRootDrainingReservationRequest,
            FleetSubnetRootDrainingReservationResponse,
            FleetSubnetRootDrainingReservationStatusRequest, FleetSubnetRootEntry,
            FleetSubnetRootJoinRequest, FleetSubnetRootJoinResponse,
            FleetSubnetRootRemovalPublicationRequest, FleetSubnetRootRemovalPublicationResponse,
            FleetSubnetRootSnapshotAcknowledgement, FleetSubnetRootSnapshotAcknowledgementRequest,
            FleetSubnetRootStatus,
        },
    },
    ids::{
        ComponentDeploymentConfigurationDigest, ComponentGroupDeploymentId,
        ComponentTopologyDigest, FleetRegistryAuthority, FleetSubnetRootReleaseSet,
        MAX_FLEET_ROOT_FUNDING_SLOTS, SubnetId,
    },
    shared_support::fleet_funding_policy::{
        validate_coordinator_root_funding_policy, validate_fleet_root_funding_capacity,
    },
};
use sha2::{Digest, Sha256};

const COMPONENT_SCALE_OUT_RECEIPT_HASH_DOMAIN: &[u8] =
    b"canic/fleet-component-scale-out-terminal-receipt/v1";

///
/// FleetCoordinatorOps
///
/// Single-step Coordinator state and canonical Registry operations.
///

pub struct FleetCoordinatorOps;

impl FleetCoordinatorOps {
    /// Authorize a controller or exact Root participating in one durable operation.
    pub(crate) fn authorize_operation_caller(
        operation_id: [u8; 32],
        caller: candid::Principal,
        caller_is_controller: bool,
    ) -> Result<(), InternalError> {
        if caller_is_controller {
            return Ok(());
        }
        let current = Self::current()?;
        let active_provisioning = [
            current.component_provisioning.as_ref(),
            current.component_scale_out.as_ref(),
        ]
        .into_iter()
        .flatten()
        .find(|record| record.operation_id == operation_id);
        let active_participant = active_provisioning.is_some_and(|record| {
            record
                .plan
                .batches
                .iter()
                .any(|batch| batch.root.fleet_subnet_root == caller)
                || record.plan.directory_confirmation_roots.contains(&caller)
        });
        let retired_participant = current
            .component_scale_out_receipts
            .iter()
            .find(|receipt| receipt.operation_id == operation_id)
            .is_some_and(|receipt| {
                receipt
                    .placements
                    .iter()
                    .any(|placement| placement.fleet_subnet_root == caller)
            });
        let removal_participant = current.root_draining_reservations.iter().any(|record| {
            record.response.request.operation_id == operation_id
                && record.response.request.expected_root.fleet_subnet_root == caller
        });
        if active_participant || retired_participant || removal_participant {
            Ok(())
        } else {
            Err(InternalError::forbidden())
        }
    }

    /// Resolve one Coordinator-owned durable operation without caller-supplied secondary keys.
    pub(crate) fn operation_status(
        operation_id: [u8; 32],
    ) -> Result<CoordinatorOperationStatusResponse, InternalError> {
        if operation_id == [0; 32] {
            return Err(InternalError::invalid_input());
        }
        let current = Self::current()?;

        let mut active_provisioning_matches = [
            current.component_provisioning.as_ref(),
            current.component_scale_out.as_ref(),
        ]
        .into_iter()
        .flatten()
        .filter(|record| record.operation_id == operation_id);
        let active_provisioning = active_provisioning_matches.next();
        if active_provisioning_matches.next().is_some() {
            return Err(receipt_invariant(
                "Coordinator operation ID identifies multiple active provisioning records",
            ));
        }
        let retired_provisioning = component_scale_out_receipt_for_operation(
            &current.component_scale_out_receipts,
            operation_id,
        )?;
        if active_provisioning.is_some() && retired_provisioning.is_some() {
            return Err(receipt_invariant(
                "Coordinator operation ID identifies active and retired provisioning records",
            ));
        }
        let provisioning = if let Some(record) = active_provisioning {
            Some(component_provisioning_status_response(record)?)
        } else if let Some(receipt) = retired_provisioning {
            Some(component_scale_out_receipt_response(receipt)?)
        } else {
            None
        };

        let root_removal = coordinator_root_removal_operation_status(&current, operation_id)?;
        if provisioning.is_some() && root_removal.is_some() {
            return Err(receipt_invariant(
                "Coordinator operation ID crosses provisioning and root-removal domains",
            ));
        }

        if let Some(status) = provisioning {
            return Ok(CoordinatorOperationStatusResponse::ComponentProvisioning(
                status,
            ));
        }
        if let Some(status) = root_removal {
            return Ok(CoordinatorOperationStatusResponse::RootRemoval(status));
        }
        Err(InternalError::unavailable())
    }

    pub(crate) fn compile_genesis(
        args: FleetCoordinatorInitArgs,
        coordinator_canister: Principal,
    ) -> Result<FleetCoordinatorRegistryRecord, InternalError> {
        if args.authority.binding.coordinator != coordinator_canister {
            return Err(InternalError::invalid_input());
        }
        args.component_deployment_configuration
            .digest()
            .map_err(|_error| InternalError::invalid_input())?;
        if let Some(policy) = args.root_funding.as_ref() {
            validate_coordinator_root_funding_policy(policy)
                .map_err(|_error| InternalError::invalid_input())?;
        }
        let component_topology = &args.component_deployment_configuration.component_topology;
        let registry = FleetRegistryOps::compile_genesis(
            &args.configured_app,
            args.authority.clone(),
            component_topology,
        )?;
        Ok(FleetCoordinatorRegistryRecord {
            configured_app: args.configured_app,
            authority: args.authority,
            component_deployment_configuration: args.component_deployment_configuration,
            root_funding: args.root_funding,
            registry,
            root_join_receipts: Vec::new(),
            root_snapshot_acknowledgements: Vec::new(),
            registry_activation_receipt: None,
            component_provisioning: None,
            component_group_deployments: Vec::new(),
            component_scale_out_receipts: Vec::new(),
            component_scale_out: None,
            service_publication_receipts: Vec::new(),
            root_draining_reservations: Vec::new(),
            root_draining_publication_receipts: Vec::new(),
            root_removal_publication_receipts: Vec::new(),
            root_deletion_readiness_intents: Vec::new(),
            root_deletion_readiness_receipts: Vec::new(),
            root_deletion_execution_intents: Vec::new(),
            root_deletion_receipts: Vec::new(),
        })
    }

    pub(crate) fn commit_genesis(
        record: FleetCoordinatorRegistryRecord,
    ) -> Result<FleetCoordinatorCommitOutcome, InternalError> {
        FleetCoordinatorRegistryStore::commit_genesis(record).map_err(|_| InternalError::conflict())
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
            return Err(InternalError::conflict());
        }
        if current.registry_activation_receipt.is_some() {
            return Err(InternalError::conflict());
        }

        let current_version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
        )?;
        if request.expected_registry != current_version {
            return Err(InternalError::conflict());
        }
        let next_registry = FleetRegistryOps::compile_joining(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
            request.entry.clone(),
        )?;
        if next_registry == current.registry {
            return Err(InternalError::invariant());
        }
        let version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &next_registry,
        )?;
        let mut next = current.clone();
        next.registry = next_registry;
        next.root_join_receipts
            .push(FleetSubnetRootJoinReceiptRecord {
                entry: request.entry.clone(),
                version: version.clone(),
            });
        next.root_snapshot_acknowledgements.clear();
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
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
        )
    }

    pub(crate) fn registry_for_caller(
        caller: Principal,
        caller_is_controller: bool,
    ) -> Result<FleetRegistry, InternalError> {
        let current = Self::current()?;
        if !caller_is_controller {
            require_snapshot_root(&current, caller)?;
        }
        Ok(current.registry)
    }

    /// Authorize a controller or exact snapshot Root without returning Registry state.
    pub(crate) fn authorize_registry_caller(
        caller: Principal,
        caller_is_controller: bool,
    ) -> Result<(), InternalError> {
        if caller_is_controller {
            return Ok(());
        }
        require_snapshot_root(&Self::current()?, caller).map(|_| ())
    }

    /// Authorize an exact joining Root before acknowledgement workflow dispatch.
    pub(crate) fn authorize_root_snapshot_caller(caller: Principal) -> Result<(), InternalError> {
        require_joining_root(&Self::current()?, caller).map(|_| ())
    }

    pub(crate) fn acknowledge_root_snapshot(
        caller: Principal,
        request: FleetSubnetRootSnapshotAcknowledgementRequest,
    ) -> Result<FleetSubnetRootSnapshotAcknowledgement, InternalError> {
        let current = Self::current()?;
        require_joining_root(&current, caller)?;
        require_all_roots_joining(&current)?;
        let current_version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
        )?;
        if request.version != current_version {
            return Err(InternalError::conflict());
        }
        let acknowledgement = FleetSubnetRootSnapshotAcknowledgement {
            fleet_subnet_root: caller,
            version: current_version,
        };
        if let Some(existing) = current
            .root_snapshot_acknowledgements
            .iter()
            .find(|existing| existing.fleet_subnet_root == caller)
        {
            if existing == &acknowledgement {
                return Ok(existing.clone());
            }
            return Err(InternalError::conflict());
        }

        let mut next = current.clone();
        next.root_snapshot_acknowledgements
            .push(acknowledgement.clone());
        next.root_snapshot_acknowledgements.sort_by(|left, right| {
            left.fleet_subnet_root
                .as_slice()
                .cmp(right.fleet_subnet_root.as_slice())
        });
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(acknowledgement)
    }

    pub(crate) fn activate_registry(
        request: FleetRegistryActivationRequest,
    ) -> Result<FleetRegistryActivationResponse, InternalError> {
        let current = Self::current()?;
        if let Some(receipt) = &current.registry_activation_receipt {
            if receipt.request == request {
                return Ok(receipt.response.clone());
            }
            return Err(InternalError::conflict());
        }
        require_all_roots_joining(&current)?;
        let current_version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
        )?;
        if request.expected_registry != current_version {
            return Err(InternalError::conflict());
        }
        require_complete_snapshot_acknowledgements(&current, &current_version)?;

        let next_registry = FleetRegistryOps::compile_active(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
        )?;
        let version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &next_registry,
        )?;
        let response = FleetRegistryActivationResponse {
            previous_version: current_version,
            version,
        };
        let mut next = current.clone();
        next.registry = next_registry;
        next.root_snapshot_acknowledgements.clear();
        next.registry_activation_receipt = Some(FleetRegistryActivationReceiptRecord {
            request,
            response: response.clone(),
        });
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(response)
    }

    pub(crate) fn prepare_component_provisioning(
        request: FleetComponentProvisioningPrepareRequest,
        planned_at_ns: u64,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        let current = Self::current()?;
        if request.operation_id == [0; 32] {
            return Err(InternalError::invalid_input());
        }
        if planned_at_ns == 0 {
            return Err(InternalError::invalid_input());
        }
        match request.plan.operation {
            FleetComponentProvisioningOperation::FreshInstall => {
                Self::prepare_fresh_component_provisioning(current, request, planned_at_ns)
            }
            FleetComponentProvisioningOperation::ScaleOut { .. } => {
                Self::prepare_component_scale_out(current, request, planned_at_ns)
            }
        }
    }

    fn prepare_fresh_component_provisioning(
        current: FleetCoordinatorRegistryRecord,
        request: FleetComponentProvisioningPrepareRequest,
        planned_at_ns: u64,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        if let Some(existing) = &current.component_provisioning {
            if existing.operation_id == request.operation_id && existing.plan == request.plan {
                return component_provisioning_status_response(existing);
            }
            return Err(InternalError::conflict());
        }
        require_component_plan_roots_unreserved(&current, &request.plan)?;
        if !current.service_publication_receipts.is_empty() {
            return Err(InternalError::conflict());
        }
        let source_registry = initial_active_registry(&current)?;
        if current.registry != source_registry {
            return Err(InternalError::conflict());
        }
        ComponentProvisioningPlanOps::validate_compiled(
            &current.component_deployment_configuration,
            &source_registry,
            &request.plan,
        )?;
        let plan_hash = ComponentProvisioningPlanOps::hash_compiled(
            &current.component_deployment_configuration,
            &source_registry,
            &request.plan,
        )?;
        let record = FleetComponentProvisioningRecord {
            operation_id: request.operation_id,
            plan_hash,
            plan: request.plan,
            state: FleetComponentProvisioningStateRecord::Planned { planned_at_ns },
        };
        let mut next = current.clone();
        next.component_provisioning = Some(record.clone());
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        component_provisioning_status_response(&record)
    }

    fn prepare_component_scale_out(
        current: FleetCoordinatorRegistryRecord,
        request: FleetComponentProvisioningPrepareRequest,
        planned_at_ns: u64,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        if let Some(receipt) = component_scale_out_receipt_for_operation(
            &current.component_scale_out_receipts,
            request.operation_id,
        )? {
            let retry_hash = ComponentProvisioningPlanOps::hash_for_exact_retry(&request.plan)?;
            if receipt.plan_hash == retry_hash {
                return component_scale_out_receipt_response(receipt);
            }
            return Err(InternalError::conflict());
        }
        let terminal_receipt = match &current.component_scale_out {
            Some(existing) if existing.operation_id == request.operation_id => {
                if existing.plan == request.plan {
                    return component_provisioning_status_response(existing);
                }
                return Err(InternalError::conflict());
            }
            Some(existing)
                if matches!(
                    existing.state,
                    FleetComponentProvisioningStateRecord::RuntimesActivated { .. }
                ) =>
            {
                Some(component_scale_out_terminal_receipt(
                    existing,
                    &current.component_group_deployments,
                )?)
            }
            Some(_) => {
                return Err(InternalError::conflict());
            }
            None => None,
        };
        let fresh = current
            .component_provisioning
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        if fresh.operation_id == request.operation_id {
            return Err(InternalError::conflict());
        }
        if !matches!(
            fresh.state,
            FleetComponentProvisioningStateRecord::RuntimesActivated { .. }
        ) {
            return Err(InternalError::unavailable());
        }
        require_component_plan_roots_unreserved(&current, &request.plan)?;
        let plan_hash = deployment_ledger::scale_out_plan_hash(
            &current.component_deployment_configuration,
            &current.registry,
            fresh,
            &current.component_group_deployments,
            &request.plan,
        )?;
        let reserved_deployments = deployment_ledger::reserve_scale_out(
            &current.component_group_deployments,
            &request.plan,
        )?;
        let record = FleetComponentProvisioningRecord {
            operation_id: request.operation_id,
            plan_hash,
            plan: request.plan,
            state: FleetComponentProvisioningStateRecord::Planned { planned_at_ns },
        };
        let mut next = current.clone();
        if let Some(receipt) = terminal_receipt {
            next.component_scale_out_receipts.push(receipt);
        }
        next.component_group_deployments = reserved_deployments;
        next.component_scale_out = Some(record.clone());
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        component_provisioning_status_response(&record)
    }

    pub(crate) fn component_provisioning_status(
        request: FleetComponentProvisioningStatusRequest,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        let current = Self::current()?;
        if let Some(record) = active_provisioning_record_for_status(&current, &request)? {
            return component_provisioning_status_response(record);
        }
        let receipt = component_scale_out_receipt_for_operation(
            &current.component_scale_out_receipts,
            request.operation_id,
        )?
        .ok_or_else(InternalError::unavailable)?;
        if receipt.plan_hash != request.plan_hash {
            return Err(InternalError::conflict());
        }
        component_scale_out_receipt_response(receipt)
    }

    pub(crate) fn advance_component_provisioning_root_acceptance(
        request: &FleetComponentProvisioningAdvanceRequest,
        started_at_ns: u64,
    ) -> Result<FleetComponentProvisioningRootAcceptanceDisposition, InternalError> {
        let current = Self::current()?;
        let record = require_component_provisioning_operation_record(&current, request)?;
        let progress = component_provisioning_root_acceptance_progress(record)?;
        match classify_root_acceptance_advance(request, &progress)? {
            RootAcceptanceAdvance::Current => {
                return component_provisioning_status_response(record)
                    .map(FleetComponentProvisioningRootAcceptanceDisposition::Current);
            }
            RootAcceptanceAdvance::Reconcile => {
                let call = root_acceptance_call(record, progress.accepted_root_count)?;
                return Ok(FleetComponentProvisioningRootAcceptanceDisposition::Reconcile(call));
            }
            RootAcceptanceAdvance::Begin => {}
        }
        if started_at_ns == 0 {
            return Err(InternalError::invalid_input());
        }
        if progress.accepted_root_count == progress.root_batch_count {
            let mut next = current.clone();
            let next_record =
                component_provisioning_operation_record_mut(&mut next, request.operation_id)?;
            next_record.state = FleetComponentProvisioningStateRecord::RootsAccepted {
                planned_at_ns: progress.planned_at_ns,
                acceptances: progress.acceptances,
                roots_accepted_at_ns: started_at_ns,
            };
            let next = Self::validate_current(next)?;
            let response = component_provisioning_status_response(
                component_provisioning_operation_record(&next, request.operation_id)?,
            )?;
            Self::commit_transition(&current, next)?;
            return Ok(FleetComponentProvisioningRootAcceptanceDisposition::Current(response));
        }
        let call = root_acceptance_call(record, progress.accepted_root_count)?;
        let intent = FleetComponentProvisioningRootAcceptanceIntentRecord {
            root_index: progress.accepted_root_count,
            fleet_subnet_root: call.fleet_subnet_root,
            started_at_ns,
        };
        let mut next = current.clone();
        component_provisioning_operation_record_mut(&mut next, request.operation_id)?.state =
            FleetComponentProvisioningStateRecord::AcceptingRoots {
                planned_at_ns: progress.planned_at_ns,
                acceptances: progress.acceptances,
                in_flight: Some(intent),
            };
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(FleetComponentProvisioningRootAcceptanceDisposition::Invoke(
            call,
        ))
    }

    pub(crate) fn record_component_provisioning_root_acceptance(
        request: &FleetComponentProvisioningAdvanceRequest,
        response: RootComponentProvisioningStatusResponse,
        recorded_at_ns: u64,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        let current = Self::current()?;
        let record = require_component_provisioning_operation_record(&current, request)?;
        let mut progress = component_provisioning_root_acceptance_progress(record)?;
        if progress.accepted_root_count > request.expected_accepted_root_count {
            return replay_recorded_root_acceptance(record, request, &response, &progress);
        }
        if progress.accepted_root_count != request.expected_accepted_root_count {
            return Err(InternalError::conflict());
        }
        let intent = progress.in_flight.ok_or_else(InternalError::conflict)?;
        let batch = root_batch(record, intent.root_index)?;
        validate_root_acceptance_response(record, batch, &response)?;
        validate_root_acceptance_observation(intent.started_at_ns, &response, recorded_at_ns)?;
        progress
            .acceptances
            .push(FleetComponentProvisioningRootAcceptanceRecord {
                started_at_ns: intent.started_at_ns,
                response,
                recorded_at_ns,
            });
        let accepted_root_count = u32::try_from(progress.acceptances.len())
            .map_err(|_| InternalError::resource_exhausted())?;
        let mut next = current.clone();
        let next_record =
            component_provisioning_operation_record_mut(&mut next, request.operation_id)?;
        next_record.state = if accepted_root_count == progress.root_batch_count {
            FleetComponentProvisioningStateRecord::RootsAccepted {
                planned_at_ns: progress.planned_at_ns,
                acceptances: progress.acceptances,
                roots_accepted_at_ns: recorded_at_ns,
            }
        } else {
            FleetComponentProvisioningStateRecord::AcceptingRoots {
                planned_at_ns: progress.planned_at_ns,
                acceptances: progress.acceptances,
                in_flight: None,
            }
        };
        let next = Self::validate_current(next)?;
        let result = component_provisioning_status_response(
            component_provisioning_operation_record(&next, request.operation_id)?,
        )?;
        Self::commit_transition(&current, next)?;
        Ok(result)
    }

    pub(crate) fn advance_component_provisioning_root(
        request: &FleetComponentProvisioningAdvanceRequest,
        started_at_ns: u64,
    ) -> Result<FleetComponentProvisioningRootProvisionDisposition, InternalError> {
        let current = Self::current()?;
        let record = require_component_provisioning_operation_record(&current, request)?;
        let progress = component_provisioning_root_provision_progress(record)?;
        match classify_root_provision_advance(request, &progress)? {
            RootProvisionAdvance::Current => {
                return component_provisioning_status_response(record)
                    .map(Box::new)
                    .map(FleetComponentProvisioningRootProvisionDisposition::Current);
            }
            RootProvisionAdvance::Publish => {
                return Ok(FleetComponentProvisioningRootProvisionDisposition::Publish);
            }
            RootProvisionAdvance::Reconcile => {
                let intent = progress.in_flight.as_ref().ok_or_else(|| {
                    receipt_invariant("root provisioning reconciliation intent disappeared")
                })?;
                return Ok(
                    FleetComponentProvisioningRootProvisionDisposition::Reconcile(
                        root_provision_call_from_intent(intent),
                    ),
                );
            }
            RootProvisionAdvance::Begin => {}
        }
        if started_at_ns == 0 {
            return Err(InternalError::invalid_input());
        }
        let roots_accepted_at_ns = progress
            .roots_accepted_at_ns
            .ok_or_else(InternalError::conflict)?;
        let previous_observed_at_ns = root_provision_previous_observed_at(&progress)?;
        if started_at_ns < previous_observed_at_ns {
            return Err(InternalError::invalid_input());
        }
        let response = progress
            .current_response
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        let call = root_provision_call(record, progress.provisioned_root_count, response)?;
        let intent = FleetComponentProvisioningRootProvisionIntentRecord {
            root_index: progress.provisioned_root_count,
            fleet_subnet_root: call.fleet_subnet_root,
            request: call.request,
            started_at_ns,
        };
        let acceptance = component_provisioning_root_acceptance_progress(record)?;
        let mut next = current.clone();
        component_provisioning_operation_record_mut(&mut next, request.operation_id)?.state =
            FleetComponentProvisioningStateRecord::ProvisioningRoots {
                planned_at_ns: acceptance.planned_at_ns,
                acceptances: acceptance.acceptances,
                roots_accepted_at_ns,
                provisions: progress.provisions,
                current: progress.current.map(Box::new),
                in_flight: Some(intent.clone()),
            };
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(FleetComponentProvisioningRootProvisionDisposition::Invoke(
            root_provision_call_from_intent(&intent),
        ))
    }

    pub(crate) fn record_component_provisioning_root(
        request: &FleetComponentProvisioningAdvanceRequest,
        response: RootComponentProvisioningStatusResponse,
        recorded_at_ns: u64,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        let current = Self::current()?;
        let record = require_component_provisioning_operation_record(&current, request)?;
        let mut progress = component_provisioning_root_provision_progress(record)?;
        if let Some(replayed) =
            replay_recorded_root_provision(record, request, &response, &progress)?
        {
            return Ok(replayed);
        }
        if classify_root_provision_advance(request, &progress)? != RootProvisionAdvance::Reconcile {
            return Err(InternalError::conflict());
        }
        let intent = progress
            .in_flight
            .take()
            .ok_or_else(|| receipt_invariant("root provisioning response intent disappeared"))?;
        if recorded_at_ns < intent.started_at_ns {
            return Err(InternalError::invalid_input());
        }
        let previous = progress.current_response.as_ref().ok_or_else(|| {
            receipt_invariant("root provisioning response has no durable predecessor")
        })?;
        let acceptance = component_provisioning_root_acceptance(record, intent.root_index)?;
        validate_root_provision_response(RootProvisionResponseValidation {
            configuration: &current.component_deployment_configuration,
            record,
            root_index: intent.root_index,
            acceptance: &acceptance,
            previous,
            response: &response,
            started_at_ns: intent.started_at_ns,
            recorded_at_ns,
        })?;
        let observed = FleetComponentProvisioningRootProvisionRecord {
            started_at_ns: intent.started_at_ns,
            response,
            recorded_at_ns,
        };
        let root_is_provisioned =
            observed.response.phase == RootComponentProvisioningPhase::Provisioned;
        if root_is_provisioned {
            progress.provisions.push(observed);
            progress.current = None;
        } else {
            progress.current = Some(observed);
        }
        let acceptance_progress = component_provisioning_root_acceptance_progress(record)?;
        let roots_accepted_at_ns = progress.roots_accepted_at_ns.ok_or_else(|| {
            receipt_invariant("root provisioning state lost its RootsAccepted time")
        })?;
        let provisioned_root_count = u32::try_from(progress.provisions.len())
            .map_err(|_| receipt_invariant("provisioned root count does not fit u32"))?;
        let mut next = current.clone();
        let next_record =
            component_provisioning_operation_record_mut(&mut next, request.operation_id)?;
        if provisioned_root_count == acceptance_progress.root_batch_count {
            next_record.state = FleetComponentProvisioningStateRecord::ComponentsProvisioned {
                planned_at_ns: acceptance_progress.planned_at_ns,
                acceptances: acceptance_progress.acceptances,
                roots_accepted_at_ns,
                provisions: progress.provisions,
                components_provisioned_at_ns: recorded_at_ns,
            };
        } else {
            next_record.state = FleetComponentProvisioningStateRecord::ProvisioningRoots {
                planned_at_ns: acceptance_progress.planned_at_ns,
                acceptances: acceptance_progress.acceptances,
                roots_accepted_at_ns,
                provisions: progress.provisions,
                current: progress.current.map(Box::new),
                in_flight: None,
            };
        }
        let next = Self::validate_current(next)?;
        let result = component_provisioning_status_response(
            component_provisioning_operation_record(&next, request.operation_id)?,
        )?;
        Self::commit_transition(&current, next)?;
        Ok(result)
    }

    #[cfg(test)]
    pub(crate) fn prepare_component_provisioning_for_test(
        config: &ConfigModel,
        request: FleetComponentProvisioningPrepareRequest,
        planned_at_ns: u64,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        require_test_component_deployment_configuration(config)?;
        Self::prepare_component_provisioning(request, planned_at_ns)
    }

    #[cfg(test)]
    pub(crate) fn component_provisioning_status_for_test(
        config: &ConfigModel,
        request: FleetComponentProvisioningStatusRequest,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        require_test_component_deployment_configuration(config)?;
        Self::component_provisioning_status(request)
    }

    #[cfg(test)]
    pub(crate) fn advance_component_provisioning_root_acceptance_for_test(
        config: &ConfigModel,
        request: &FleetComponentProvisioningAdvanceRequest,
        started_at_ns: u64,
    ) -> Result<FleetComponentProvisioningRootAcceptanceDisposition, InternalError> {
        require_test_component_deployment_configuration(config)?;
        Self::advance_component_provisioning_root_acceptance(request, started_at_ns)
    }

    #[cfg(test)]
    pub(crate) fn record_component_provisioning_root_acceptance_for_test(
        config: &ConfigModel,
        request: &FleetComponentProvisioningAdvanceRequest,
        response: RootComponentProvisioningStatusResponse,
        recorded_at_ns: u64,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        require_test_component_deployment_configuration(config)?;
        Self::record_component_provisioning_root_acceptance(request, response, recorded_at_ns)
    }

    #[cfg(test)]
    pub(crate) fn advance_component_provisioning_root_for_test(
        config: &ConfigModel,
        request: &FleetComponentProvisioningAdvanceRequest,
        started_at_ns: u64,
    ) -> Result<FleetComponentProvisioningRootProvisionDisposition, InternalError> {
        require_test_component_deployment_configuration(config)?;
        Self::advance_component_provisioning_root(request, started_at_ns)
    }

    #[cfg(test)]
    pub(crate) fn record_component_provisioning_root_for_test(
        config: &ConfigModel,
        request: &FleetComponentProvisioningAdvanceRequest,
        response: RootComponentProvisioningStatusResponse,
        recorded_at_ns: u64,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        require_test_component_deployment_configuration(config)?;
        Self::record_component_provisioning_root(request, response, recorded_at_ns)
    }

    #[cfg(test)]
    pub(crate) fn require_root_lifecycle_open_for_test(
        config: &ConfigModel,
        fleet_subnet_root: Principal,
    ) -> Result<(), InternalError> {
        require_test_component_deployment_configuration(config)?;
        let current = Self::current()?;
        require_grouped_root_lifecycle_open(&current, fleet_subnet_root)
    }

    pub(crate) fn publish_component_provisioning_services(
        request: &FleetComponentProvisioningAdvanceRequest,
        published_at_ns: u64,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        let current = Self::current()?;
        let record = require_component_provisioning_operation_record(&current, request)?;
        let progress = component_provisioning_root_provision_progress(record)?;
        match classify_root_provision_advance(request, &progress)? {
            RootProvisionAdvance::Current => {
                return component_provisioning_status_response(record);
            }
            RootProvisionAdvance::Publish => {}
            RootProvisionAdvance::Begin | RootProvisionAdvance::Reconcile => {
                return Err(InternalError::conflict());
            }
        }
        if published_at_ns == 0 {
            return Err(InternalError::invalid_input());
        }
        let provisioned = components_provisioned_state(record)?;
        if published_at_ns < provisioned.components_provisioned_at_ns {
            return Err(InternalError::invalid_input());
        }
        let publication = compile_service_publication(&current, record, &provisioned)?;
        let mut next = current.clone();
        next.registry = publication.registry;
        next.service_publication_receipts
            .push(publication.receipt.clone());
        component_provisioning_operation_record_mut(&mut next, request.operation_id)?.state =
            FleetComponentProvisioningStateRecord::ServiceTopologyPublished {
                planned_at_ns: provisioned.planned_at_ns,
                acceptances: provisioned.acceptances,
                roots_accepted_at_ns: provisioned.roots_accepted_at_ns,
                provisions: provisioned.provisions,
                components_provisioned_at_ns: provisioned.components_provisioned_at_ns,
                published_fleet_registry: publication.receipt.version,
                service_topology_published_at_ns: published_at_ns,
            };
        let next = Self::validate_current(next)?;
        let result = component_provisioning_status_response(
            component_provisioning_operation_record(&next, request.operation_id)?,
        )?;
        Self::commit_transition(&current, next)?;
        Ok(result)
    }

    pub(crate) fn advance_component_directory_confirmation(
        request: &FleetComponentProvisioningAdvanceRequest,
        started_at_ns: u64,
    ) -> Result<FleetComponentDirectoryConfirmationDisposition, InternalError> {
        let current = Self::current()?;
        let record = require_component_provisioning_operation_record(&current, request)?;
        let progress = component_directory_confirmation_progress(record)?;
        if matches!(
            record.plan.operation,
            FleetComponentProvisioningOperation::ScaleOut { .. }
        ) {
            return advance_scale_out_directory_confirmation(
                &current,
                record,
                request,
                progress,
                started_at_ns,
            );
        }
        match classify_directory_confirmation_advance(request, &progress)? {
            DirectoryConfirmationAdvance::Current => {
                return component_provisioning_status_response(record)
                    .map(Box::new)
                    .map(FleetComponentDirectoryConfirmationDisposition::Current);
            }
            DirectoryConfirmationAdvance::Reconcile => {
                let intent = progress.in_flight.as_ref().ok_or_else(|| {
                    receipt_invariant("Directory confirmation intent disappeared")
                })?;
                return Ok(FleetComponentDirectoryConfirmationDisposition::Reconcile(
                    directory_confirmation_call_from_intent(intent),
                ));
            }
            DirectoryConfirmationAdvance::Begin => {}
        }
        if started_at_ns == 0 || started_at_ns < progress.service_topology_published_at_ns {
            return Err(InternalError::invalid_input());
        }
        let root_index = progress.confirmed_root_count;
        let previous = progress
            .current
            .as_ref()
            .map(fresh_confirmation_response)
            .transpose()?
            .cloned()
            .map_or_else(
                || root_provisioned_response(&progress, root_index).cloned(),
                Ok,
            )?;
        let root = confirmation_root(record, root_index)?;
        if previous.fleet_subnet_root != root {
            return Err(receipt_invariant(
                "Directory confirmation cursor differs from canonical root order",
            ));
        }
        let call = FleetComponentDirectoryConfirmationCallView::FreshPublication {
            fleet_subnet_root: root,
            request: RootComponentPublicationRequest {
                operation_id: record.operation_id,
                plan_hash: record.plan_hash,
                published_fleet_registry: progress.published_fleet_registry.clone(),
                expected_published_component_count: previous.published_component_count,
            },
        };
        let intent = FleetComponentDirectoryConfirmationIntentRecord::FreshPublication {
            root_index,
            fleet_subnet_root: root,
            request: confirmation_call_publication_request(&call)?.clone(),
            started_at_ns,
        };
        let mut next = current.clone();
        component_provisioning_record_mut(&mut next)?.state =
            FleetComponentProvisioningStateRecord::ConfirmingDirectories {
                planned_at_ns: progress.planned_at_ns,
                acceptances: progress.acceptances,
                roots_accepted_at_ns: progress.roots_accepted_at_ns,
                provisions: progress.provisions,
                components_provisioned_at_ns: progress.components_provisioned_at_ns,
                published_fleet_registry: progress.published_fleet_registry,
                service_topology_published_at_ns: progress.service_topology_published_at_ns,
                confirmations: progress.confirmations,
                current: progress.current.map(Box::new),
                in_flight: Some(Box::new(intent)),
            };
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(FleetComponentDirectoryConfirmationDisposition::Invoke(call))
    }

    pub(crate) fn record_component_directory_confirmation(
        request: &FleetComponentProvisioningAdvanceRequest,
        response: RootComponentProvisioningStatusResponse,
        recorded_at_ns: u64,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        let current = Self::current()?;
        let record = require_component_provisioning_operation_record(&current, request)?;
        if matches!(
            record.plan.operation,
            FleetComponentProvisioningOperation::ScaleOut { .. }
        ) {
            return Err(InternalError::conflict());
        }
        let mut progress = component_directory_confirmation_progress(record)?;
        if classify_directory_confirmation_advance(request, &progress)?
            != DirectoryConfirmationAdvance::Reconcile
        {
            return Err(InternalError::conflict());
        }
        let intent = progress
            .in_flight
            .take()
            .ok_or_else(|| receipt_invariant("Directory confirmation intent disappeared"))?;
        let (intent_root_index, intent_root, intent_request, intent_started_at_ns) =
            fresh_confirmation_intent(&intent)?;
        if recorded_at_ns < intent_started_at_ns {
            return Err(InternalError::invalid_input());
        }
        let previous = progress
            .current
            .as_ref()
            .map(fresh_confirmation_response)
            .transpose()?
            .map_or_else(
                || root_provisioned_response(&progress, intent_root_index),
                Ok,
            )?;
        let fleet_directory_content_hash = expected_fleet_directory_content_hash(
            &current,
            &progress.published_fleet_registry,
            intent_root,
        )?;
        validate_directory_confirmation_response(
            RootDirectoryConfirmationValidationContext::new(
                record,
                &progress.published_fleet_registry,
                intent_root,
                fleet_directory_content_hash,
            ),
            previous,
            &response,
            recorded_at_ns,
            true,
        )?;
        if intent_request.operation_id != request.operation_id {
            return Err(InternalError::conflict());
        }
        let observed = FleetComponentDirectoryConfirmationRecord::FreshPublication {
            started_at_ns: intent_started_at_ns,
            response: Box::new(response),
            recorded_at_ns,
        };
        if fresh_confirmation_response(&observed)?.phase
            == RootComponentProvisioningPhase::Published
        {
            progress.confirmations.push(observed);
            progress.current = None;
        } else {
            progress.current = Some(observed);
        }
        commit_directory_confirmation_progress(&current, request, progress, recorded_at_ns)
    }

    pub(crate) fn record_component_scale_out_directory_synchronization(
        request: &FleetComponentProvisioningAdvanceRequest,
        response: RootComponentDirectorySynchronizationResponse,
        recorded_at_ns: u64,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        let current = Self::current()?;
        let record = require_component_provisioning_operation_record(&current, request)?;
        require_scale_out_operation(record)?;
        let mut progress = component_directory_confirmation_progress(record)?;
        if classify_directory_confirmation_advance(request, &progress)?
            != DirectoryConfirmationAdvance::Reconcile
        {
            return Err(InternalError::conflict());
        }
        let intent = progress
            .in_flight
            .take()
            .ok_or_else(|| receipt_invariant("Directory synchronization intent disappeared"))?;
        let (root_index, root, sync_request, started_at_ns) =
            scale_out_synchronization_intent(&intent)?;
        validate_scale_out_synchronization_response(
            &ScaleOutSynchronizationValidationContext {
                coordinator: &current,
                operation: record,
                progress: &progress,
                root_index,
                root,
                request: sync_request,
                started_at_ns,
                recorded_at_ns,
            },
            &response,
        )?;
        let observed = FleetComponentDirectoryConfirmationRecord::ScaleOut {
            started_at_ns,
            synchronization: Box::new(response),
            publication: None,
            recorded_at_ns,
        };
        if scale_out_confirmation_is_terminal(record, root, &observed)? {
            progress.confirmations.push(observed);
            progress.current = None;
        } else {
            progress.current = Some(observed);
        }
        commit_directory_confirmation_progress(&current, request, progress, recorded_at_ns)
    }

    pub(crate) fn record_component_scale_out_directory_publication(
        request: &FleetComponentProvisioningAdvanceRequest,
        response: RootComponentProvisioningStatusResponse,
        recorded_at_ns: u64,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        let current = Self::current()?;
        let record = require_component_provisioning_operation_record(&current, request)?;
        require_scale_out_operation(record)?;
        let mut progress = component_directory_confirmation_progress(record)?;
        if classify_directory_confirmation_advance(request, &progress)?
            != DirectoryConfirmationAdvance::Reconcile
        {
            return Err(InternalError::conflict());
        }
        let intent = progress
            .in_flight
            .take()
            .ok_or_else(|| receipt_invariant("Directory publication intent disappeared"))?;
        let (root_index, root, publication_request, started_at_ns) =
            scale_out_publication_intent(&intent)?;
        let current_confirmation = progress.current.as_ref().ok_or_else(|| {
            receipt_invariant("scale-out publication lacks terminal synchronization evidence")
        })?;
        let (synchronization, previous_publication) =
            scale_out_confirmation_progress(current_confirmation)?;
        if !synchronization.complete {
            return Err(receipt_invariant(
                "scale-out publication preceded terminal Directory synchronization",
            ));
        }
        let previous = previous_publication.map_or_else(
            || selected_root_provisioned_response(record, &progress, root),
            Ok,
        )?;
        let intent_is_exact = [
            root_index == progress.confirmed_root_count,
            confirmation_root(record, root_index)? == root,
            publication_request.operation_id == record.operation_id,
            publication_request.plan_hash == record.plan_hash,
            publication_request.published_fleet_registry == progress.published_fleet_registry,
            publication_request.expected_published_component_count
                == previous.published_component_count,
            started_at_ns >= confirmation_recorded_at_ns(current_confirmation),
            recorded_at_ns >= started_at_ns,
        ]
        .into_iter()
        .all(|matches| matches);
        if !intent_is_exact {
            return Err(InternalError::conflict());
        }
        let fleet_directory_content_hash = expected_fleet_directory_content_hash(
            &current,
            &progress.published_fleet_registry,
            root,
        )?;
        validate_directory_confirmation_response(
            RootDirectoryConfirmationValidationContext::new(
                record,
                &progress.published_fleet_registry,
                root,
                fleet_directory_content_hash,
            ),
            previous,
            &response,
            recorded_at_ns,
            true,
        )?;
        let observed = FleetComponentDirectoryConfirmationRecord::ScaleOut {
            started_at_ns: confirmation_started_at_ns(current_confirmation),
            synchronization: Box::new(synchronization.clone()),
            publication: Some(Box::new(response)),
            recorded_at_ns,
        };
        if confirmation_publication_response(&observed)
            .is_some_and(|response| response.phase == RootComponentProvisioningPhase::Published)
        {
            progress.confirmations.push(observed);
            progress.current = None;
        } else {
            progress.current = Some(observed);
        }
        commit_directory_confirmation_progress(&current, request, progress, recorded_at_ns)
    }

    pub(crate) fn advance_component_runtime_activation(
        request: &FleetComponentProvisioningAdvanceRequest,
        started_at_ns: u64,
    ) -> Result<FleetComponentRuntimeActivationDisposition, InternalError> {
        let current = Self::current()?;
        let record = require_component_provisioning_operation_record(&current, request)?;
        let progress = component_runtime_activation_progress(record)?;
        match classify_runtime_activation_advance(request, &progress)? {
            RuntimeActivationAdvance::Current => {
                return component_provisioning_status_response(record)
                    .map(Box::new)
                    .map(FleetComponentRuntimeActivationDisposition::Current);
            }
            RuntimeActivationAdvance::Reconcile => {
                let intent = progress
                    .in_flight
                    .as_ref()
                    .ok_or_else(|| receipt_invariant("runtime activation intent disappeared"))?;
                return Ok(FleetComponentRuntimeActivationDisposition::Reconcile(
                    runtime_activation_call_from_intent(intent),
                ));
            }
            RuntimeActivationAdvance::Begin => {}
        }
        if started_at_ns == 0 || started_at_ns < progress.directories_confirmed_at_ns {
            return Err(InternalError::invalid_input());
        }
        let root_index = progress.activated_root_count;
        let publication = root_publication_response(record, &progress, root_index)?;
        let current_progress = progress.current.map_or_else(
            || root_activation_progress(publication),
            |record| record.progress,
        );
        let root = activation_root(record, root_index)?;
        if current_progress.fleet_subnet_root != root {
            return Err(receipt_invariant(
                "runtime activation cursor differs from canonical root order",
            ));
        }
        let call = FleetComponentRuntimeActivationCallView {
            fleet_subnet_root: root,
            request: RootComponentActivationRequest {
                operation_id: record.operation_id,
                plan_hash: record.plan_hash,
                expected_activated_component_count: current_progress.activated_component_count,
                expected_root_runtime_active: current_progress.root_runtime_active,
            },
        };
        let intent = FleetComponentRuntimeActivationIntentRecord {
            root_index,
            fleet_subnet_root: root,
            request: call.request,
            started_at_ns,
        };
        let mut next = current.clone();
        component_provisioning_operation_record_mut(&mut next, request.operation_id)?.state =
            FleetComponentProvisioningStateRecord::ActivatingRuntimes {
                planned_at_ns: progress.planned_at_ns,
                acceptances: progress.acceptances,
                roots_accepted_at_ns: progress.roots_accepted_at_ns,
                provisions: progress.provisions,
                components_provisioned_at_ns: progress.components_provisioned_at_ns,
                published_fleet_registry: progress.published_fleet_registry,
                service_topology_published_at_ns: progress.service_topology_published_at_ns,
                confirmations: progress.confirmations,
                directories_confirmed_at_ns: progress.directories_confirmed_at_ns,
                activations: progress.activations,
                current: progress.current.map(Box::new),
                in_flight: Some(intent),
            };
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(FleetComponentRuntimeActivationDisposition::Invoke(call))
    }

    pub(crate) fn record_component_runtime_activation(
        request: &FleetComponentProvisioningAdvanceRequest,
        response: &RootComponentProvisioningStatusResponse,
        recorded_at_ns: u64,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        let current = Self::current()?;
        let record = require_component_provisioning_operation_record(&current, request)?;
        let mut progress = component_runtime_activation_progress(record)?;
        if classify_runtime_activation_advance(request, &progress)?
            != RuntimeActivationAdvance::Reconcile
        {
            return Err(InternalError::conflict());
        }
        let intent = progress
            .in_flight
            .take()
            .ok_or_else(|| receipt_invariant("runtime activation intent disappeared"))?;
        if recorded_at_ns < intent.started_at_ns {
            return Err(InternalError::invalid_input());
        }
        let publication = root_publication_response(record, &progress, intent.root_index)?;
        let previous_record = progress.current;
        let previous = previous_record.map_or_else(
            || root_activation_progress(publication),
            |record| record.progress,
        );
        validate_runtime_activation_response(
            record,
            intent.root_index,
            publication,
            previous,
            previous_record.and_then(|record| record.activation_started_at_ns),
            response,
            recorded_at_ns,
        )?;
        commit_runtime_activation_response(
            &current,
            request,
            progress,
            intent,
            response,
            recorded_at_ns,
        )
    }

    pub(crate) fn prepare_root_draining_reservation(
        request: FleetSubnetRootDrainingReservationRequest,
        prepared_at_ns: u64,
    ) -> Result<FleetSubnetRootDrainingReservationResponse, InternalError> {
        if request.operation_id == [0; 32] {
            return Err(InternalError::invalid_input());
        }
        if prepared_at_ns == 0 {
            return Err(InternalError::invalid_input());
        }
        let current = Self::current()?;
        if let Some(record) = current
            .root_draining_reservations
            .iter()
            .find(|record| draining_reservation_identity_matches(&record.response, &request))
        {
            if record.response.request == request {
                return Ok(record.response.clone());
            }
            return Err(InternalError::conflict());
        }
        if current.registry_activation_receipt.is_none() {
            return Err(InternalError::conflict());
        }
        let current_version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
        )?;
        validate_root_draining_reservation_request(&current, &current_version, &request)?;
        require_grouped_root_lifecycle_open(&current, request.expected_root.fleet_subnet_root)?;

        let mut response = FleetSubnetRootDrainingReservationResponse {
            request,
            coordinator: current.authority.binding.coordinator,
            prepared_at_ns,
            reservation_hash: [0; 32],
        };
        response.reservation_hash = FleetSubnetRootDrainingReservationOps::content_hash(&response)?;
        let mut next = current.clone();
        next.root_draining_reservations
            .push(FleetSubnetRootDrainingReservationRecord {
                response: response.clone(),
            });
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(response)
    }

    pub(crate) fn root_draining_reservation_status(
        request: FleetSubnetRootDrainingReservationStatusRequest,
    ) -> Result<FleetSubnetRootDrainingReservationResponse, InternalError> {
        let current = Self::current()?;
        let record = current
            .root_draining_reservations
            .iter()
            .find(|record| draining_reservation_status_matches(&record.response, &request))
            .ok_or_else(InternalError::unavailable)?;
        let response = &record.response;
        if response.request.operation_id != request.operation_id
            || response.request.expected_root.fleet_subnet_root != request.fleet_subnet_root
        {
            return Err(InternalError::conflict());
        }
        Ok(response.clone())
    }

    pub(crate) fn publish_root_draining(
        request: FleetSubnetRootDrainingPublicationRequest,
    ) -> Result<FleetSubnetRootDrainingPublicationResponse, InternalError> {
        let current = Self::current()?;
        require_grouped_root_lifecycle_open(&current, request.root_draining.fleet_subnet_root)?;
        if let Some(receipt) = current
            .root_draining_publication_receipts
            .iter()
            .find(|receipt| draining_publication_identity_matches(receipt, &request))
        {
            if receipt.request == request {
                return Ok(receipt.response.clone());
            }
            return Err(InternalError::conflict());
        }
        if current.registry_activation_receipt.is_none() {
            return Err(InternalError::conflict());
        }
        let previous_version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
        )?;
        if request.expected_registry != previous_version {
            return Err(InternalError::conflict());
        }
        let reservation = draining_reservation_for_publication(&current, &request)?;
        validate_draining_publication_request(
            &current.registry,
            &previous_version,
            &request,
            reservation,
        )
        .map_err(|_| InternalError::invalid_input())?;

        let next_registry = FleetRegistryOps::compile_draining(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
            request.root_draining.fleet_subnet_root,
        )?;
        let version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &next_registry,
        )?;
        let response = FleetSubnetRootDrainingPublicationResponse {
            root_draining: request.root_draining.clone(),
            previous_version,
            version,
        };
        let mut next = current.clone();
        next.registry = next_registry;
        next.root_draining_publication_receipts.push(
            FleetSubnetRootDrainingPublicationReceiptRecord {
                request,
                response: response.clone(),
            },
        );
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(response)
    }

    pub(crate) fn publish_root_removed(
        caller: Principal,
        request: FleetSubnetRootRemovalPublicationRequest,
    ) -> Result<FleetSubnetRootRemovalPublicationResponse, InternalError> {
        if caller != request.final_inventory.fleet_subnet_root {
            return Err(InternalError::forbidden());
        }
        let current = Self::current()?;
        require_grouped_root_lifecycle_open(&current, request.final_inventory.fleet_subnet_root)?;
        if let Some(receipt) = current
            .root_removal_publication_receipts
            .iter()
            .find(|receipt| removal_publication_identity_matches(receipt, &request))
        {
            if receipt.request == request {
                return Ok(receipt.response.clone());
            }
            return Err(InternalError::conflict());
        }
        let previous_version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
        )?;
        if request.expected_registry != previous_version {
            return Err(InternalError::conflict());
        }
        let history = canonical_registry_lifecycle_history(&current)?;
        validate_removal_publication_request(
            &current.registry,
            &previous_version,
            &current.root_draining_publication_receipts,
            &history,
            &request,
        )
        .map_err(|_| InternalError::invalid_input())?;

        let next_registry = FleetRegistryOps::compile_removed(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
            request.final_inventory.fleet_subnet_root,
        )?;
        let version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &next_registry,
        )?;
        let response = FleetSubnetRootRemovalPublicationResponse {
            final_inventory: request.final_inventory.clone(),
            previous_version,
            version,
        };
        let mut next = current.clone();
        next.registry = next_registry;
        next.root_removal_publication_receipts.push(
            FleetSubnetRootRemovalPublicationReceiptRecord {
                request,
                response: response.clone(),
            },
        );
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(response)
    }

    pub(crate) fn root_snapshot_acknowledgements()
    -> Result<Vec<FleetSubnetRootSnapshotAcknowledgement>, InternalError> {
        Ok(Self::current()?.root_snapshot_acknowledgements)
    }

    pub(crate) fn version() -> Result<FleetRegistryVersion, InternalError> {
        let current = Self::current()?;
        FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
        )
    }

    fn current() -> Result<FleetCoordinatorRegistryRecord, InternalError> {
        let current = FleetCoordinatorRegistryStore::export()
            .current
            .ok_or_else(InternalError::unavailable)?;
        Self::validate_current(current)
    }

    fn validate_current(
        current: FleetCoordinatorRegistryRecord,
    ) -> Result<FleetCoordinatorRegistryRecord, InternalError> {
        let current = Self::validate_current_registry(current)?;
        validate_component_provisioning_record(&current)?;
        validate_component_scale_out_receipts(&current)?;
        validate_component_scale_out_progress(&current)?;
        validate_service_publication_receipt_owners(&current)?;
        let deployment_registry = current
            .component_scale_out
            .as_ref()
            .map(|record| component_operation_source_registry(&current, record))
            .transpose()?
            .unwrap_or_else(|| current.registry.clone());
        deployment_ledger::validate(
            &current.component_deployment_configuration,
            &deployment_registry,
            current.component_provisioning.as_ref(),
            &current.component_scale_out_receipts,
            current.component_scale_out.as_ref(),
            &current.component_group_deployments,
        )?;
        Ok(current)
    }

    fn validate_current_registry(
        current: FleetCoordinatorRegistryRecord,
    ) -> Result<FleetCoordinatorRegistryRecord, InternalError> {
        if current.authority != current.registry.authority {
            return Err(InternalError::invariant());
        }
        if current.configured_app != current.authority.binding.fleet.app {
            return Err(InternalError::invariant());
        }
        current
            .component_deployment_configuration
            .digest()
            .map_err(|_| {
                receipt_invariant("stored Component deployment configuration is invalid")
            })?;
        FleetRegistryOps::validate(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
        )?;
        if current.registry.fleet_subnet_roots.len() > MAX_FLEET_ROOT_FUNDING_SLOTS {
            return Err(InternalError::invariant());
        }
        match current.root_funding.as_ref() {
            Some(policy) => {
                validate_coordinator_root_funding_policy(policy)
                    .map_err(|_error| InternalError::invariant())?;
                validate_fleet_root_funding_capacity(
                    policy,
                    current
                        .registry
                        .fleet_subnet_roots
                        .iter()
                        .map(|root| &root.funding),
                )
                .map_err(|_error| InternalError::invariant())?;
            }
            None if !current.registry.fleet_subnet_roots.is_empty() => {
                return Err(InternalError::invariant());
            }
            None => {}
        }
        validate_root_join_receipts(&current)?;
        validate_root_snapshot_acknowledgements(&current)?;
        validate_registry_lifecycle_history(&current)?;
        validate_root_draining_reservations(&current)?;
        validate_root_deletion_history(&current)?;
        Ok(current)
    }

    fn commit_transition(
        current: &FleetCoordinatorRegistryRecord,
        next: FleetCoordinatorRegistryRecord,
    ) -> Result<FleetCoordinatorCommitOutcome, InternalError> {
        FleetCoordinatorRegistryStore::commit_transition(current, next).map_err(|error| match error
        {
            FleetCoordinatorCommitError::ConflictingState => InternalError::conflict(),
            FleetCoordinatorCommitError::Uninitialized => InternalError::unavailable(),
        })
    }
}

#[cfg(test)]
fn require_test_component_deployment_configuration(
    config: &ConfigModel,
) -> Result<(), InternalError> {
    let expected = config
        .compile_component_deployment_configuration()
        .map_err(|_error| InternalError::invalid_input())?;
    let current = FleetCoordinatorRegistryStore::export()
        .current
        .ok_or_else(InternalError::unavailable)?;
    if current.component_deployment_configuration != expected {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn validate_component_provisioning_record(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    let Some(record) = &current.component_provisioning else {
        if !current.service_publication_receipts.is_empty() {
            return Err(receipt_invariant(
                "Fleet-service publication receipt lacks its provisioning operation",
            ));
        }
        return Ok(());
    };
    if record.operation_id == [0; 32] {
        return Err(receipt_invariant(
            "Fleet Component provisioning operation ID is zero",
        ));
    }
    if record.plan_hash == [0; 32] {
        return Err(receipt_invariant(
            "Fleet Component provisioning plan hash is zero",
        ));
    }
    if record.plan.operation != FleetComponentProvisioningOperation::FreshInstall {
        return Err(receipt_invariant(
            "Fleet Component provisioning record contains an unavailable operation kind",
        ));
    }
    let source_registry = component_operation_source_registry(current, record)?;
    ComponentProvisioningPlanOps::validate_compiled(
        &current.component_deployment_configuration,
        &source_registry,
        &record.plan,
    )
    .map_err(|_| {
        receipt_invariant(
            "Fleet Component provisioning plan differs from canonical configuration or Registry authority",
        )
    })?;
    let plan_hash = ComponentProvisioningPlanOps::hash_compiled(
        &current.component_deployment_configuration,
        &source_registry,
        &record.plan,
    )
    .map_err(|_| receipt_invariant("Fleet Component provisioning plan hash cannot be rederived"))?;
    if record.plan_hash != plan_hash {
        return Err(receipt_invariant(
            "Fleet Component provisioning plan hash differs from canonical bytes",
        ));
    }
    validate_component_provisioning_root_acceptance_state(record)?;
    validate_component_provisioning_root_provision_state(
        &current.component_deployment_configuration,
        &source_registry,
        record,
    )?;
    validate_service_publication_authority(current, record)?;
    validate_component_directory_confirmation_state(current, record)?;
    validate_component_runtime_activation_state(record)?;
    component_provisioning_plan_counts(&record.plan)?;
    Ok(())
}

fn validate_component_scale_out_progress(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    let Some(record) = &current.component_scale_out else {
        return Ok(());
    };
    if !matches!(
        record.plan.operation,
        FleetComponentProvisioningOperation::ScaleOut { .. }
    ) {
        return Err(receipt_invariant(
            "Fleet Component scale-out record contains a different operation kind",
        ));
    }
    if !matches!(
        record.state,
        FleetComponentProvisioningStateRecord::Planned { .. }
            | FleetComponentProvisioningStateRecord::AcceptingRoots { .. }
            | FleetComponentProvisioningStateRecord::RootsAccepted { .. }
            | FleetComponentProvisioningStateRecord::ProvisioningRoots { .. }
            | FleetComponentProvisioningStateRecord::ComponentsProvisioned { .. }
            | FleetComponentProvisioningStateRecord::ServiceTopologyPublished { .. }
            | FleetComponentProvisioningStateRecord::ConfirmingDirectories { .. }
            | FleetComponentProvisioningStateRecord::DirectoriesConfirmed { .. }
            | FleetComponentProvisioningStateRecord::ActivatingRuntimes { .. }
            | FleetComponentProvisioningStateRecord::RuntimesActivated { .. }
    ) {
        return Err(receipt_invariant(
            "Fleet Component scale-out has an invalid runtime-activation state",
        ));
    }
    let source_registry = component_operation_source_registry(current, record)?;
    validate_component_provisioning_root_acceptance_state(record)?;
    validate_component_provisioning_root_provision_state(
        &current.component_deployment_configuration,
        &source_registry,
        record,
    )?;
    validate_service_publication_authority(current, record)?;
    validate_scale_out_service_publication_fence(record)?;
    validate_component_directory_confirmation_state(current, record)?;
    validate_component_runtime_activation_state(record)?;
    component_provisioning_plan_counts(&record.plan)?;
    Ok(())
}

fn validate_component_scale_out_receipts(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    if current.component_scale_out_receipts.len() > MAX_FLEET_COMPONENT_PROVISIONING_PLAN_PLACEMENTS
    {
        return Err(receipt_invariant(
            "retired scale-out receipt count exceeds the placement bound",
        ));
    }
    let fresh_operation = current
        .component_provisioning
        .as_ref()
        .map(|record| record.operation_id);
    let active_operation = current
        .component_scale_out
        .as_ref()
        .map(|record| record.operation_id);
    let configuration_digest = current
        .component_deployment_configuration
        .digest()
        .map_err(|_| receipt_invariant("deployment configuration digest cannot be rederived"))?;
    let mut operation_ids = BTreeSet::new();
    let mut previous_completed_at_ns = 0_u64;
    for receipt in &current.component_scale_out_receipts {
        validate_retired_scale_out_identity(
            receipt,
            configuration_digest,
            fresh_operation,
            active_operation,
            operation_ids.insert(receipt.operation_id),
        )?;
        validate_retired_scale_out_content_hash(receipt)?;
        let authority = retired_scale_out_authority(receipt)?;
        validate_retired_scale_out_counts(receipt, authority.placement_count)?;
        validate_retired_scale_out_times(receipt, previous_completed_at_ns)?;
        validate_retired_scale_out_registry(receipt, &current.authority)?;
        validate_retired_scale_out_placements(receipt, &authority)?;
        validate_retired_scale_out_publication(current, receipt)?;
        component_scale_out_receipt_response(receipt)?;
        previous_completed_at_ns = receipt.runtimes_activated_at_ns;
    }
    if let Some(active) = &current.component_scale_out {
        let active = component_provisioning_status_response(active)?;
        if active.planned_at_ns < previous_completed_at_ns {
            return Err(receipt_invariant(
                "active scale-out journal predates retired terminal history",
            ));
        }
    }
    Ok(())
}

struct RetiredScaleOutAuthority<'a> {
    deployment: &'a ComponentGroupDeploymentId,
    previous_placements: u32,
    placement_count: usize,
}

fn retired_scale_out_authority(
    receipt: &FleetComponentScaleOutReceiptRecord,
) -> Result<RetiredScaleOutAuthority<'_>, InternalError> {
    let FleetComponentProvisioningOperation::ScaleOut {
        deployment,
        previous_placements,
        requested_placements,
    } = &receipt.operation
    else {
        return Err(receipt_invariant(
            "retired Component operation is not scale-out",
        ));
    };
    let placement_count = requested_placements
        .checked_sub(*previous_placements)
        .filter(|count| *count > 0)
        .ok_or_else(|| receipt_invariant("retired scale-out count is not monotonic"))?;
    Ok(RetiredScaleOutAuthority {
        deployment,
        previous_placements: *previous_placements,
        placement_count: usize::try_from(placement_count)
            .map_err(|_| receipt_invariant("retired scale-out count does not fit usize"))?,
    })
}

fn validate_retired_scale_out_identity(
    receipt: &FleetComponentScaleOutReceiptRecord,
    configuration_digest: ComponentDeploymentConfigurationDigest,
    fresh_operation: Option<[u8; 32]>,
    active_operation: Option<[u8; 32]>,
    operation_is_unique: bool,
) -> Result<(), InternalError> {
    let identity_facts = [
        receipt.operation_id != [0; 32],
        receipt.plan_hash != [0; 32],
        fresh_operation != Some(receipt.operation_id),
        active_operation != Some(receipt.operation_id),
        operation_is_unique,
        receipt.configuration_digest == configuration_digest,
    ];
    if identity_facts.into_iter().all(|fact| fact) {
        Ok(())
    } else {
        Err(receipt_invariant(
            "retired scale-out receipt has invalid or reused operation authority",
        ))
    }
}

fn validate_retired_scale_out_content_hash(
    receipt: &FleetComponentScaleOutReceiptRecord,
) -> Result<(), InternalError> {
    if receipt.receipt_content_hash == [0; 32]
        || receipt.receipt_content_hash != component_scale_out_receipt_content_hash(receipt)?
    {
        return Err(receipt_invariant(
            "retired scale-out receipt content hash is invalid",
        ));
    }
    Ok(())
}

fn validate_retired_scale_out_counts(
    receipt: &FleetComponentScaleOutReceiptRecord,
    placement_count: usize,
) -> Result<(), InternalError> {
    let root_batch_count = usize::try_from(receipt.root_batch_count)
        .map_err(|_| receipt_invariant("retired root count does not fit usize"))?;
    let confirmation_root_count = usize::try_from(receipt.directory_confirmation_root_count)
        .map_err(|_| receipt_invariant("retired confirmation count does not fit usize"))?;
    let component_count = usize::try_from(receipt.component_count)
        .map_err(|_| receipt_invariant("retired Component count does not fit usize"))?;
    let count_facts = [
        receipt.placements.len() == placement_count,
        root_batch_count > 0,
        root_batch_count <= MAX_FLEET_COMPONENT_PROVISIONING_PLAN_BATCHES,
        confirmation_root_count >= root_batch_count,
        confirmation_root_count <= MAX_FLEET_COMPONENT_PROVISIONING_PLAN_CONFIRMATION_ROOTS,
        component_count >= placement_count,
        component_count <= MAX_FLEET_COMPONENT_PROVISIONING_PLAN_ENTRIES,
    ];
    if count_facts.into_iter().all(|fact| fact) {
        Ok(())
    } else {
        Err(receipt_invariant(
            "retired scale-out receipt has invalid bounded counts",
        ))
    }
}

fn validate_retired_scale_out_times(
    receipt: &FleetComponentScaleOutReceiptRecord,
    previous_completed_at_ns: u64,
) -> Result<(), InternalError> {
    let times = [
        receipt.planned_at_ns,
        receipt.roots_accepted_at_ns,
        receipt.components_provisioned_at_ns,
        receipt.service_topology_published_at_ns,
        receipt.directories_confirmed_at_ns,
        receipt.runtimes_activated_at_ns,
    ];
    let time_facts = [
        receipt.planned_at_ns >= previous_completed_at_ns,
        times[0] > 0,
        times.windows(2).all(|pair| pair[0] <= pair[1]),
    ];
    if time_facts.into_iter().all(|fact| fact) {
        Ok(())
    } else {
        Err(receipt_invariant(
            "retired scale-out receipt has invalid terminal ordering",
        ))
    }
}

fn validate_retired_scale_out_registry(
    receipt: &FleetComponentScaleOutReceiptRecord,
    authority: &FleetRegistryAuthority,
) -> Result<(), InternalError> {
    let registry_facts = [
        &receipt.fleet_registry.authority == authority,
        &receipt.published_fleet_registry.authority == authority,
        receipt.published_fleet_registry.revision >= receipt.fleet_registry.revision,
    ];
    if registry_facts.into_iter().all(|fact| fact) {
        Ok(())
    } else {
        Err(receipt_invariant(
            "retired scale-out receipt has invalid Fleet Registry authority",
        ))
    }
}

fn validate_retired_scale_out_placements(
    receipt: &FleetComponentScaleOutReceiptRecord,
    authority: &RetiredScaleOutAuthority<'_>,
) -> Result<(), InternalError> {
    let mut selected_root_receipts = BTreeSet::new();
    for (offset, placement) in receipt.placements.iter().enumerate() {
        let offset = u32::try_from(offset)
            .map_err(|_| receipt_invariant("retired placement offset does not fit u32"))?;
        let ordinal = authority
            .previous_placements
            .checked_add(offset)
            .ok_or_else(|| receipt_invariant("retired placement ordinal overflowed"))?;
        let placement_facts = [
            &placement.placement.deployment == authority.deployment,
            placement.placement.ordinal == ordinal,
            placement.operation_id == receipt.operation_id,
            placement.plan_hash == receipt.plan_hash,
            placement.root_receipt_content_hash != [0; 32],
        ];
        if !placement_facts.into_iter().all(|fact| fact) {
            return Err(receipt_invariant(
                "retired scale-out placement authority is invalid",
            ));
        }
        selected_root_receipts.insert((
            placement.fleet_subnet_root,
            placement.root_receipt_content_hash,
        ));
    }
    if selected_root_receipts.len()
        == usize::try_from(receipt.root_batch_count)
            .map_err(|_| receipt_invariant("retired root count does not fit usize"))?
    {
        Ok(())
    } else {
        Err(receipt_invariant(
            "retired scale-out receipt lacks exact selected-root evidence",
        ))
    }
}

fn validate_retired_scale_out_publication(
    current: &FleetCoordinatorRegistryRecord,
    receipt: &FleetComponentScaleOutReceiptRecord,
) -> Result<(), InternalError> {
    let publication = service_publication_receipt_for_operation(current, receipt.operation_id)?
        .ok_or_else(|| receipt_invariant("retired scale-out lacks publication authority"))?;
    let actual = (
        publication.operation_id,
        publication.plan_hash,
        publication.configuration_digest,
        &publication.previous_version,
        &publication.version,
    );
    let expected = (
        receipt.operation_id,
        receipt.plan_hash,
        receipt.configuration_digest,
        &receipt.fleet_registry,
        &receipt.published_fleet_registry,
    );
    if actual == expected {
        Ok(())
    } else {
        Err(receipt_invariant(
            "retired scale-out publication authority is invalid",
        ))
    }
}

fn validate_service_publication_receipt_owners(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    let mut operation_ids = current
        .component_scale_out_receipts
        .iter()
        .map(|receipt| receipt.operation_id)
        .collect::<BTreeSet<_>>();
    operation_ids.extend(
        [
            current.component_provisioning.as_ref(),
            current.component_scale_out.as_ref(),
        ]
        .into_iter()
        .flatten()
        .map(|record| record.operation_id),
    );
    for receipt in &current.service_publication_receipts {
        if !operation_ids.contains(&receipt.operation_id) {
            return Err(receipt_invariant(
                "Fleet-service publication receipt lacks its provisioning operation",
            ));
        }
    }
    Ok(())
}

fn validate_scale_out_service_publication_fence(
    record: &FleetComponentProvisioningRecord,
) -> Result<(), InternalError> {
    let progress = component_provisioning_root_provision_progress(record)?;
    let current_root_crossed_terminal_fence = progress
        .current_response
        .as_ref()
        .is_some_and(|response| response.phase != RootComponentProvisioningPhase::Accepted);
    if current_root_crossed_terminal_fence {
        return Err(receipt_invariant(
            "Fleet Component scale-out current root crossed its terminal provisioning fence",
        ));
    }
    Ok(())
}

fn validate_component_directory_confirmation_state(
    coordinator: &FleetCoordinatorRegistryRecord,
    record: &FleetComponentProvisioningRecord,
) -> Result<(), InternalError> {
    let progress = match &record.state {
        FleetComponentProvisioningStateRecord::ServiceTopologyPublished { .. }
        | FleetComponentProvisioningStateRecord::ConfirmingDirectories { .. }
        | FleetComponentProvisioningStateRecord::DirectoriesConfirmed { .. }
        | FleetComponentProvisioningStateRecord::ActivatingRuntimes { .. }
        | FleetComponentProvisioningStateRecord::RuntimesActivated { .. } => {
            component_directory_confirmation_progress(record)?
        }
        _ => return Ok(()),
    };
    let selected_root_count = u32::try_from(record.plan.batches.len())
        .map_err(|_| receipt_invariant("root batch count does not fit u32"))?;
    let scale_out = matches!(
        record.plan.operation,
        FleetComponentProvisioningOperation::ScaleOut { .. }
    );
    let root_count_is_valid = if scale_out {
        selected_root_count <= progress.confirmation_root_count
            && record.plan.batches.iter().all(|batch| {
                record
                    .plan
                    .directory_confirmation_roots
                    .contains(&batch.root.fleet_subnet_root)
            })
    } else {
        selected_root_count == progress.confirmation_root_count
    };
    if !root_count_is_valid || progress.confirmed_root_count > progress.confirmation_root_count {
        return Err(receipt_invariant(
            "Directory confirmation roots differ from the protected barrier",
        ));
    }
    let mut previous_recorded_at_ns = validate_completed_directory_confirmations(
        coordinator,
        record,
        &progress,
        progress.service_topology_published_at_ns,
    )?;
    previous_recorded_at_ns = validate_current_directory_confirmation(
        coordinator,
        record,
        &progress,
        previous_recorded_at_ns,
    )?;
    validate_directory_confirmation_intent(record, &progress, previous_recorded_at_ns)?;
    validate_terminal_directory_confirmation(record, &progress, previous_recorded_at_ns)
}

fn validate_component_runtime_activation_state(
    record: &FleetComponentProvisioningRecord,
) -> Result<(), InternalError> {
    let progress = match &record.state {
        FleetComponentProvisioningStateRecord::ActivatingRuntimes { .. }
        | FleetComponentProvisioningStateRecord::RuntimesActivated { .. } => {
            component_runtime_activation_progress(record)?
        }
        _ => return Ok(()),
    };
    if progress.activation_root_count
        != u32::try_from(record.plan.batches.len())
            .map_err(|_| receipt_invariant("runtime activation root count does not fit u32"))?
        || progress.activated_root_count > progress.activation_root_count
    {
        return Err(receipt_invariant(
            "runtime activation roots differ from selected root batches",
        ));
    }
    let mut previous_recorded_at_ns = progress.directories_confirmed_at_ns;
    for (index, activation) in progress.activations.iter().enumerate() {
        let root_index = u32::try_from(index)
            .map_err(|_| receipt_invariant("runtime activation root index does not fit u32"))?;
        validate_stored_runtime_activation(record, &progress, root_index, activation, true)?;
        if activation.started_at_ns < previous_recorded_at_ns
            || activation.recorded_at_ns < activation.started_at_ns
        {
            return Err(receipt_invariant(
                "runtime activation observation time evidence is invalid",
            ));
        }
        previous_recorded_at_ns = activation.recorded_at_ns;
    }
    if let Some(current) = &progress.current {
        validate_stored_runtime_activation(
            record,
            &progress,
            progress.activated_root_count,
            current,
            false,
        )?;
        if current.started_at_ns < previous_recorded_at_ns
            || current.recorded_at_ns < current.started_at_ns
        {
            return Err(receipt_invariant(
                "current runtime activation observation time evidence is invalid",
            ));
        }
        previous_recorded_at_ns = current.recorded_at_ns;
    }
    validate_runtime_activation_intent(record, &progress, previous_recorded_at_ns)?;
    validate_terminal_runtime_activation_state(&progress, previous_recorded_at_ns)
}

fn validate_stored_runtime_activation(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentRuntimeActivationProgress,
    root_index: u32,
    activation: &FleetComponentRuntimeActivationRecord,
    terminal: bool,
) -> Result<(), InternalError> {
    let publication = root_publication_response(record, progress, root_index)?;
    let expected_progress = FleetComponentActivationRootProgress {
        fleet_subnet_root: publication.fleet_subnet_root,
        component_count: publication.component_count,
        activated_component_count: if terminal {
            publication.component_count
        } else {
            activation.progress.activated_component_count
        },
        root_runtime_active: terminal,
    };
    let activation_started_at_ns = activation
        .activation_started_at_ns
        .ok_or_else(|| receipt_invariant("stored runtime activation lacks its root start time"))?;
    let published_at_ns = publication
        .published_at_ns
        .ok_or_else(|| receipt_invariant("stored root publication lacks completion time"))?;
    let component_cursor_is_bounded = terminal
        || (activation.progress.activated_component_count > 0
            && activation.progress.activated_component_count <= publication.component_count);
    if activation.progress != expected_progress
        || !component_cursor_is_bounded
        || activation_started_at_ns < published_at_ns
        || activation.recorded_at_ns < activation_started_at_ns
    {
        return Err(receipt_invariant(
            "stored runtime activation progress or time authority is invalid",
        ));
    }
    if terminal {
        validate_stored_terminal_runtime_activation(
            record,
            root_index,
            publication,
            activation,
            activation_started_at_ns,
        )
    } else if activation.activation.is_some()
        || activation.runtimes_activated_at_ns.is_some()
        || activation.receipt_content_hash != publication.receipt_content_hash
    {
        Err(receipt_invariant(
            "stored in-progress runtime activation changed publication authority",
        ))
    } else {
        Ok(())
    }
}

fn validate_stored_terminal_runtime_activation(
    record: &FleetComponentProvisioningRecord,
    root_index: u32,
    publication: &RootComponentProvisioningStatusResponse,
    stored: &FleetComponentRuntimeActivationRecord,
    activation_started_at_ns: u64,
) -> Result<(), InternalError> {
    let activation = stored
        .activation
        .ok_or_else(|| receipt_invariant("stored terminal runtime activation lacks evidence"))?;
    let runtimes_activated_at_ns = stored.runtimes_activated_at_ns.ok_or_else(|| {
        receipt_invariant("stored terminal runtime activation lacks completion time")
    })?;
    let identity_is_exact = activation.component_count == publication.component_count
        && activation.fleet_activation_operation_id != [0; 32]
        && activation.initial_inventory_hash != [0; 32];
    let timing_is_exact = terminal_root_activation_timing_is_valid(
        &record.plan.operation,
        activation.root_activated_at_ns,
        publication.accepted_at_ns,
        activation_started_at_ns,
        runtimes_activated_at_ns,
    );
    let observation_is_exact = stored.recorded_at_ns >= runtimes_activated_at_ns;
    let evidence_is_exact = identity_is_exact && timing_is_exact && observation_is_exact;
    if !evidence_is_exact {
        return Err(receipt_invariant(
            "stored terminal runtime activation evidence is invalid",
        ));
    }
    let batch = root_batch(record, root_index)?;
    let expected = RootComponentProvisioningReceiptOps::runtimes_active_content_hash(
        RootComponentProvisioningRuntimesActiveReceiptAuthority {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            configuration_digest: record.plan.configuration_digest,
            root: &batch.root,
            published_receipt_content_hash: publication.receipt_content_hash,
            activation,
            activation_started_at_ns,
            runtimes_activated_at_ns,
        },
    )?;
    if stored.receipt_content_hash != expected {
        return Err(receipt_invariant(
            "stored terminal runtime activation receipt hash is invalid",
        ));
    }
    Ok(())
}

fn validate_runtime_activation_intent(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentRuntimeActivationProgress,
    previous_recorded_at_ns: u64,
) -> Result<(), InternalError> {
    let Some(intent) = &progress.in_flight else {
        return Ok(());
    };
    let root = activation_root(record, progress.activated_root_count)?;
    let current = progress.current.map_or_else(
        || {
            root_publication_response(record, progress, progress.activated_root_count)
                .map(root_activation_progress)
        },
        |current| Ok(current.progress),
    )?;
    let intent_is_exact = [
        intent.root_index == progress.activated_root_count,
        intent.fleet_subnet_root == root,
        intent.request.operation_id == record.operation_id,
        intent.request.plan_hash == record.plan_hash,
        intent.request.expected_activated_component_count == current.activated_component_count,
        intent.request.expected_root_runtime_active == current.root_runtime_active,
        intent.started_at_ns >= previous_recorded_at_ns,
    ]
    .into_iter()
    .all(|matches| matches);
    if !intent_is_exact {
        return Err(receipt_invariant(
            "runtime activation pre-call intent is invalid",
        ));
    }
    Ok(())
}

fn validate_terminal_runtime_activation_state(
    progress: &FleetComponentRuntimeActivationProgress,
    previous_recorded_at_ns: u64,
) -> Result<(), InternalError> {
    if progress.complete {
        let completed_at_ns = progress.runtimes_activated_at_ns.ok_or_else(|| {
            receipt_invariant("terminal runtime activation lacks completion time")
        })?;
        if progress.activated_root_count != progress.activation_root_count
            || progress.current.is_some()
            || progress.in_flight.is_some()
            || completed_at_ns < previous_recorded_at_ns
        {
            return Err(receipt_invariant(
                "terminal runtime activation evidence is incomplete",
            ));
        }
    } else if progress.activated_root_count >= progress.activation_root_count {
        return Err(receipt_invariant(
            "runtime activation remained nonterminal after every selected root",
        ));
    }
    Ok(())
}

fn validate_completed_directory_confirmations(
    coordinator: &FleetCoordinatorRegistryRecord,
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentDirectoryConfirmationProgress,
    mut previous_recorded_at_ns: u64,
) -> Result<u64, InternalError> {
    for (index, confirmation) in progress.confirmations.iter().enumerate() {
        let root_index = u32::try_from(index)
            .map_err(|_| receipt_invariant("Directory confirmation index does not fit u32"))?;
        let root = confirmation_root(record, root_index)?;
        if matches!(
            record.plan.operation,
            FleetComponentProvisioningOperation::ScaleOut { .. }
        ) {
            validate_stored_scale_out_confirmation(
                coordinator,
                record,
                progress,
                root,
                confirmation,
                true,
            )?;
        } else {
            let response = fresh_confirmation_response(confirmation)?;
            let previous = root_provisioned_response(progress, root_index)?;
            let fleet_directory_content_hash = expected_fleet_directory_content_hash(
                coordinator,
                &progress.published_fleet_registry,
                root,
            )?;
            validate_directory_confirmation_response(
                RootDirectoryConfirmationValidationContext::new(
                    record,
                    &progress.published_fleet_registry,
                    root,
                    fleet_directory_content_hash,
                ),
                previous,
                response,
                confirmation_recorded_at_ns(confirmation),
                false,
            )
            .map_err(|_| receipt_invariant("stored Directory confirmation receipt is invalid"))?;
            if response.phase != RootComponentProvisioningPhase::Published {
                return Err(receipt_invariant(
                    "stored fresh Directory confirmation is not terminal",
                ));
            }
        }
        let started_at_ns = confirmation_started_at_ns(confirmation);
        let recorded_at_ns = confirmation_recorded_at_ns(confirmation);
        if started_at_ns < previous_recorded_at_ns || recorded_at_ns < started_at_ns {
            return Err(receipt_invariant(
                "stored Directory confirmation time or terminal phase is invalid",
            ));
        }
        previous_recorded_at_ns = recorded_at_ns;
    }
    Ok(previous_recorded_at_ns)
}

fn validate_current_directory_confirmation(
    coordinator: &FleetCoordinatorRegistryRecord,
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentDirectoryConfirmationProgress,
    mut previous_recorded_at_ns: u64,
) -> Result<u64, InternalError> {
    if let Some(current) = &progress.current {
        let root = confirmation_root(record, progress.confirmed_root_count)?;
        if matches!(
            record.plan.operation,
            FleetComponentProvisioningOperation::ScaleOut { .. }
        ) {
            validate_stored_scale_out_confirmation(
                coordinator,
                record,
                progress,
                root,
                current,
                false,
            )?;
        } else {
            let response = fresh_confirmation_response(current)?;
            let previous = root_provisioned_response(progress, progress.confirmed_root_count)?;
            let fleet_directory_content_hash = expected_fleet_directory_content_hash(
                coordinator,
                &progress.published_fleet_registry,
                root,
            )?;
            validate_directory_confirmation_response(
                RootDirectoryConfirmationValidationContext::new(
                    record,
                    &progress.published_fleet_registry,
                    root,
                    fleet_directory_content_hash,
                ),
                previous,
                response,
                confirmation_recorded_at_ns(current),
                false,
            )
            .map_err(|_| {
                receipt_invariant("stored in-progress Directory confirmation is invalid")
            })?;
            if response.phase != RootComponentProvisioningPhase::Provisioned {
                return Err(receipt_invariant(
                    "stored fresh Directory confirmation crossed its terminal boundary",
                ));
            }
        }
        let started_at_ns = confirmation_started_at_ns(current);
        let recorded_at_ns = confirmation_recorded_at_ns(current);
        if started_at_ns < previous_recorded_at_ns || recorded_at_ns < started_at_ns {
            return Err(receipt_invariant(
                "in-progress Directory confirmation time or phase is invalid",
            ));
        }
        previous_recorded_at_ns = recorded_at_ns;
    }
    Ok(previous_recorded_at_ns)
}

fn validate_stored_scale_out_confirmation(
    coordinator: &FleetCoordinatorRegistryRecord,
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentDirectoryConfirmationProgress,
    root: Principal,
    confirmation: &FleetComponentDirectoryConfirmationRecord,
    terminal: bool,
) -> Result<(), InternalError> {
    let (synchronization, publication) = scale_out_confirmation_progress(confirmation)?;
    let expected_directory_hash = expected_fleet_directory_content_hash(
        coordinator,
        &progress.published_fleet_registry,
        root,
    )?;
    let synchronization_authority_is_exact = [
        synchronization.operation_id == record.operation_id,
        synchronization.plan_hash == record.plan_hash,
        synchronization.source_fleet_registry == record.plan.fleet_registry,
        synchronization.published_fleet_registry == progress.published_fleet_registry,
        synchronization.fleet_subnet_root == root,
        synchronization.fleet_directory_content_hash == expected_directory_hash,
        synchronization.synchronized_component_count <= synchronization.affected_component_count,
    ]
    .into_iter()
    .all(|matches| matches);
    if !synchronization_authority_is_exact {
        return Err(receipt_invariant(
            "stored scale-out Directory synchronization authority is invalid",
        ));
    }
    let synchronization_evidence_is_exact = if synchronization.complete {
        [
            synchronization.synchronized_component_count
                == synchronization.affected_component_count,
            synchronization.synchronized_at_ns.is_some_and(|time| {
                time >= confirmation_started_at_ns(confirmation)
                    && confirmation_recorded_at_ns(confirmation) >= time
            }),
            synchronization.receipt_content_hash
                == RootComponentProvisioningReceiptOps::directory_synchronization_content_hash(
                    synchronization,
                )?,
        ]
        .into_iter()
        .all(|matches| matches)
    } else {
        [
            synchronization.synchronized_component_count < synchronization.affected_component_count,
            synchronization.synchronized_at_ns.is_none(),
            synchronization.receipt_content_hash == [0; 32],
        ]
        .into_iter()
        .all(|matches| matches)
    };
    if !synchronization_evidence_is_exact {
        return Err(receipt_invariant(
            "stored scale-out Directory synchronization evidence is invalid",
        ));
    }
    let selected_batch = record
        .plan
        .batches
        .iter()
        .find(|batch| batch.root.fleet_subnet_root == root);
    match (selected_batch, publication, terminal) {
        (None, None, true) if synchronization.complete => Ok(()),
        (None, None, false) if !synchronization.complete => Ok(()),
        (Some(_), None, false) => Ok(()),
        (Some(_), Some(response), expected_terminal) => {
            if !synchronization.complete {
                return Err(receipt_invariant(
                    "stored scale-out publication preceded Directory synchronization",
                ));
            }
            let previous = selected_root_provisioned_response(record, progress, root)?;
            validate_directory_confirmation_response(
                RootDirectoryConfirmationValidationContext::new(
                    record,
                    &progress.published_fleet_registry,
                    root,
                    expected_directory_hash,
                ),
                previous,
                response,
                confirmation_recorded_at_ns(confirmation),
                false,
            )
            .map_err(|_| receipt_invariant("stored scale-out Directory publication is invalid"))?;
            let is_terminal = response.phase == RootComponentProvisioningPhase::Published;
            if is_terminal != expected_terminal {
                return Err(receipt_invariant(
                    "stored scale-out Directory publication phase is invalid",
                ));
            }
            Ok(())
        }
        _ => Err(receipt_invariant(
            "stored scale-out Directory confirmation has invalid root evidence",
        )),
    }
}

fn validate_directory_confirmation_intent(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentDirectoryConfirmationProgress,
    previous_recorded_at_ns: u64,
) -> Result<(), InternalError> {
    let Some(intent) = &progress.in_flight else {
        return Ok(());
    };
    let root = confirmation_root(record, progress.confirmed_root_count)?;
    let intent_is_exact = match intent {
        FleetComponentDirectoryConfirmationIntentRecord::FreshPublication {
            root_index,
            fleet_subnet_root,
            request,
            started_at_ns,
        } => {
            let previous = progress
                .current
                .as_ref()
                .map(fresh_confirmation_response)
                .transpose()?
                .map_or_else(
                    || root_provisioned_response(progress, progress.confirmed_root_count),
                    Ok,
                )?;
            [
                *root_index == progress.confirmed_root_count,
                *fleet_subnet_root == root,
                request.operation_id == record.operation_id,
                request.plan_hash == record.plan_hash,
                request.published_fleet_registry == progress.published_fleet_registry,
                request.expected_published_component_count == previous.published_component_count,
                *started_at_ns >= previous_recorded_at_ns,
            ]
            .into_iter()
            .all(|matches| matches)
        }
        FleetComponentDirectoryConfirmationIntentRecord::ScaleOutSynchronization {
            root_index,
            fleet_subnet_root,
            request,
            started_at_ns,
        } => {
            let expected_count = progress
                .current
                .as_ref()
                .map(scale_out_confirmation_progress)
                .transpose()?
                .map_or(0, |(synchronization, _)| {
                    synchronization.synchronized_component_count
                });
            [
                *root_index == progress.confirmed_root_count,
                *fleet_subnet_root == root,
                request.operation_id == record.operation_id,
                request.plan_hash == record.plan_hash,
                request.source_fleet_registry == record.plan.fleet_registry,
                request.published_fleet_registry == progress.published_fleet_registry,
                request.expected_synchronized_component_count == expected_count,
                *started_at_ns >= previous_recorded_at_ns,
            ]
            .into_iter()
            .all(|matches| matches)
        }
        FleetComponentDirectoryConfirmationIntentRecord::ScaleOutPublication {
            root_index,
            fleet_subnet_root,
            request,
            started_at_ns,
        } => {
            let current = progress.current.as_ref().ok_or_else(|| {
                receipt_invariant("scale-out publication intent lacks synchronization evidence")
            })?;
            let (synchronization, publication) = scale_out_confirmation_progress(current)?;
            let previous = publication.map_or_else(
                || selected_root_provisioned_response(record, progress, root),
                Ok,
            )?;
            [
                synchronization.complete,
                *root_index == progress.confirmed_root_count,
                *fleet_subnet_root == root,
                request.operation_id == record.operation_id,
                request.plan_hash == record.plan_hash,
                request.published_fleet_registry == progress.published_fleet_registry,
                request.expected_published_component_count == previous.published_component_count,
                *started_at_ns >= previous_recorded_at_ns,
            ]
            .into_iter()
            .all(|matches| matches)
        }
    };
    if !intent_is_exact {
        return Err(receipt_invariant(
            "Directory confirmation pre-call intent is invalid",
        ));
    }
    Ok(())
}

fn validate_terminal_directory_confirmation(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentDirectoryConfirmationProgress,
    previous_recorded_at_ns: u64,
) -> Result<(), InternalError> {
    if progress.complete {
        let directories_confirmed_at_ns = match &record.state {
            FleetComponentProvisioningStateRecord::DirectoriesConfirmed {
                directories_confirmed_at_ns,
                ..
            }
            | FleetComponentProvisioningStateRecord::ActivatingRuntimes {
                directories_confirmed_at_ns,
                ..
            }
            | FleetComponentProvisioningStateRecord::RuntimesActivated {
                directories_confirmed_at_ns,
                ..
            } => *directories_confirmed_at_ns,
            _ => unreachable!("complete Directory progress has terminal state"),
        };
        if progress.confirmed_root_count != progress.confirmation_root_count
            || directories_confirmed_at_ns < previous_recorded_at_ns
        {
            return Err(receipt_invariant(
                "terminal Directory confirmation evidence is incomplete",
            ));
        }
    }
    Ok(())
}

fn component_provisioning_status_response(
    record: &FleetComponentProvisioningRecord,
) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
    let counts = component_provisioning_plan_counts(&record.plan)?;
    let acceptance = component_provisioning_root_acceptance_progress(record)?;
    let provisioning = component_provisioning_root_provision_progress(record)?;
    let directory = if provisioning.published_fleet_registry.is_some() {
        Some(component_directory_confirmation_progress(record)?)
    } else {
        None
    };
    let activation = match &record.state {
        FleetComponentProvisioningStateRecord::ActivatingRuntimes { .. }
        | FleetComponentProvisioningStateRecord::RuntimesActivated { .. } => {
            Some(component_runtime_activation_progress(record)?)
        }
        _ => None,
    };
    let current_synchronization = match (
        &record.plan.operation,
        directory
            .as_ref()
            .and_then(|progress| progress.current.as_ref()),
    ) {
        (FleetComponentProvisioningOperation::ScaleOut { .. }, Some(current)) => {
            confirmation_synchronization_progress(current)
        }
        _ => None,
    };
    Ok(FleetComponentProvisioningStatusResponse {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        fleet_registry: record.plan.fleet_registry.clone(),
        configuration_digest: record.plan.configuration_digest,
        operation: record.plan.operation.clone(),
        phase: acceptance.phase,
        directory_confirmation_root_count: counts.directory_confirmation_roots,
        root_batch_count: counts.root_batches,
        accepted_root_count: acceptance.accepted_root_count,
        acceptance_in_flight_root: acceptance.in_flight.map(|intent| intent.fleet_subnet_root),
        provisioned_root_count: provisioning.provisioned_root_count,
        current_root: provisioning
            .current_response
            .as_ref()
            .map(root_provisioning_progress),
        provisioning_in_flight_root: provisioning
            .in_flight
            .as_ref()
            .map(|intent| intent.fleet_subnet_root),
        directory_confirmed_root_count: directory
            .as_ref()
            .map_or(0, |progress| progress.confirmed_root_count),
        current_synchronization,
        current_publication: directory
            .as_ref()
            .and_then(|progress| progress.current.as_ref())
            .and_then(confirmation_publication_response)
            .map(root_publication_progress),
        publication_in_flight_root: directory
            .as_ref()
            .and_then(|progress| progress.in_flight.as_ref())
            .map(confirmation_intent_root),
        runtime_activated_root_count: activation
            .as_ref()
            .map_or(0, |progress| progress.activated_root_count),
        current_activation: activation
            .as_ref()
            .and_then(|progress| progress.current.map(|record| record.progress)),
        activation_in_flight_root: activation
            .as_ref()
            .and_then(|progress| progress.in_flight)
            .map(|intent| intent.fleet_subnet_root),
        group_placement_count: counts.group_placements,
        component_count: counts.components,
        planned_at_ns: acceptance.planned_at_ns,
        roots_accepted_at_ns: acceptance.roots_accepted_at_ns,
        components_provisioned_at_ns: provisioning.components_provisioned_at_ns,
        published_fleet_registry: provisioning.published_fleet_registry,
        service_topology_published_at_ns: provisioning.service_topology_published_at_ns,
        directories_confirmed_at_ns: match &record.state {
            FleetComponentProvisioningStateRecord::DirectoriesConfirmed {
                directories_confirmed_at_ns,
                ..
            }
            | FleetComponentProvisioningStateRecord::ActivatingRuntimes {
                directories_confirmed_at_ns,
                ..
            }
            | FleetComponentProvisioningStateRecord::RuntimesActivated {
                directories_confirmed_at_ns,
                ..
            } => Some(*directories_confirmed_at_ns),
            _ => None,
        },
        runtimes_activated_at_ns: activation
            .as_ref()
            .and_then(|progress| progress.runtimes_activated_at_ns),
    })
}

fn component_scale_out_terminal_receipt(
    record: &FleetComponentProvisioningRecord,
    deployments: &[FleetComponentGroupDeploymentRecord],
) -> Result<FleetComponentScaleOutReceiptRecord, InternalError> {
    let FleetComponentProvisioningStateRecord::RuntimesActivated {
        planned_at_ns,
        roots_accepted_at_ns,
        components_provisioned_at_ns,
        published_fleet_registry,
        service_topology_published_at_ns,
        directories_confirmed_at_ns,
        runtimes_activated_at_ns,
        ..
    } = &record.state
    else {
        return Err(receipt_invariant(
            "only terminal scale-out authority may be retired",
        ));
    };
    if !matches!(
        record.plan.operation,
        FleetComponentProvisioningOperation::ScaleOut { .. }
    ) {
        return Err(receipt_invariant(
            "retired Component operation is not scale-out",
        ));
    }
    let counts = component_provisioning_plan_counts(&record.plan)?;
    let mut placements = deployments
        .iter()
        .flat_map(|deployment| &deployment.placements)
        .filter(|placement| placement.operation_id == record.operation_id)
        .cloned()
        .collect::<Vec<_>>();
    placements.sort_unstable_by(|left, right| left.placement.cmp(&right.placement));
    let mut receipt = FleetComponentScaleOutReceiptRecord {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        fleet_registry: record.plan.fleet_registry.clone(),
        configuration_digest: record.plan.configuration_digest,
        operation: record.plan.operation.clone(),
        directory_confirmation_root_count: counts.directory_confirmation_roots,
        root_batch_count: counts.root_batches,
        component_count: counts.components,
        planned_at_ns: *planned_at_ns,
        roots_accepted_at_ns: *roots_accepted_at_ns,
        components_provisioned_at_ns: *components_provisioned_at_ns,
        published_fleet_registry: published_fleet_registry.clone(),
        service_topology_published_at_ns: *service_topology_published_at_ns,
        directories_confirmed_at_ns: *directories_confirmed_at_ns,
        runtimes_activated_at_ns: *runtimes_activated_at_ns,
        placements,
        receipt_content_hash: [0; 32],
    };
    receipt.receipt_content_hash = component_scale_out_receipt_content_hash(&receipt)?;
    Ok(receipt)
}

fn component_scale_out_receipt_content_hash(
    receipt: &FleetComponentScaleOutReceiptRecord,
) -> Result<[u8; 32], InternalError> {
    let mut authority = receipt.clone();
    authority.receipt_content_hash = [0; 32];
    let payload = candid::encode_one(authority).map_err(|_error| InternalError::invariant())?;
    let mut hasher = Sha256::new();
    hasher.update(COMPONENT_SCALE_OUT_RECEIPT_HASH_DOMAIN);
    hasher.update(payload);
    Ok(hasher.finalize().into())
}

fn component_scale_out_receipt_response(
    receipt: &FleetComponentScaleOutReceiptRecord,
) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
    let FleetComponentProvisioningOperation::ScaleOut {
        previous_placements,
        requested_placements,
        ..
    } = receipt.operation
    else {
        return Err(receipt_invariant(
            "retired Component operation is not scale-out",
        ));
    };
    let group_placement_count = requested_placements
        .checked_sub(previous_placements)
        .filter(|count| *count > 0)
        .ok_or_else(|| receipt_invariant("retired scale-out count is not monotonic"))?;
    Ok(FleetComponentProvisioningStatusResponse {
        operation_id: receipt.operation_id,
        plan_hash: receipt.plan_hash,
        fleet_registry: receipt.fleet_registry.clone(),
        configuration_digest: receipt.configuration_digest,
        operation: receipt.operation.clone(),
        phase: FleetComponentProvisioningPhase::RuntimesActivated,
        directory_confirmation_root_count: receipt.directory_confirmation_root_count,
        root_batch_count: receipt.root_batch_count,
        accepted_root_count: receipt.root_batch_count,
        acceptance_in_flight_root: None,
        provisioned_root_count: receipt.root_batch_count,
        current_root: None,
        provisioning_in_flight_root: None,
        directory_confirmed_root_count: receipt.directory_confirmation_root_count,
        current_synchronization: None,
        current_publication: None,
        publication_in_flight_root: None,
        runtime_activated_root_count: receipt.root_batch_count,
        current_activation: None,
        activation_in_flight_root: None,
        group_placement_count,
        component_count: receipt.component_count,
        planned_at_ns: receipt.planned_at_ns,
        roots_accepted_at_ns: Some(receipt.roots_accepted_at_ns),
        components_provisioned_at_ns: Some(receipt.components_provisioned_at_ns),
        published_fleet_registry: Some(receipt.published_fleet_registry.clone()),
        service_topology_published_at_ns: Some(receipt.service_topology_published_at_ns),
        directories_confirmed_at_ns: Some(receipt.directories_confirmed_at_ns),
        runtimes_activated_at_ns: Some(receipt.runtimes_activated_at_ns),
    })
}

fn component_scale_out_receipt_for_operation(
    receipts: &[FleetComponentScaleOutReceiptRecord],
    operation_id: [u8; 32],
) -> Result<Option<&FleetComponentScaleOutReceiptRecord>, InternalError> {
    let mut matches = receipts
        .iter()
        .filter(|receipt| receipt.operation_id == operation_id);
    let receipt = matches.next();
    if matches.next().is_some() {
        return Err(receipt_invariant(
            "retired scale-out operation has duplicate receipts",
        ));
    }
    Ok(receipt)
}

fn coordinator_root_removal_operation_status(
    current: &FleetCoordinatorRegistryRecord,
    operation_id: [u8; 32],
) -> Result<Option<CoordinatorRootRemovalOperationStatus>, InternalError> {
    let reservation = unique_operation_match(
        current
            .root_draining_reservations
            .iter()
            .filter(|record| record.response.request.operation_id == operation_id),
    )?;
    let readiness_intent = unique_operation_match(
        current
            .root_deletion_readiness_intents
            .iter()
            .filter(|response| response.request.operation_id == operation_id),
    )?;
    let draining = unique_operation_match(
        current
            .root_draining_publication_receipts
            .iter()
            .filter(|receipt| receipt.request.root_draining.operation_id == operation_id),
    )?;
    let removal = unique_operation_match(
        current
            .root_removal_publication_receipts
            .iter()
            .filter(|receipt| receipt.request.final_inventory.operation_id == operation_id),
    )?;
    let readiness = unique_operation_match(
        current
            .root_deletion_readiness_receipts
            .iter()
            .filter(|response| response.request.operation_id == operation_id),
    )?;
    let execution = unique_operation_match(
        current
            .root_deletion_execution_intents
            .iter()
            .filter(|response| response.request.operation_id == operation_id),
    )?;
    let completion = unique_operation_match(
        current
            .root_deletion_receipts
            .iter()
            .filter(|response| response.operation_id == operation_id),
    )?;

    let any_phase = reservation.is_some()
        || draining.is_some()
        || removal.is_some()
        || readiness_intent.is_some()
        || readiness.is_some()
        || execution.is_some()
        || completion.is_some();
    if !any_phase {
        return Ok(None);
    }
    let reservation = reservation.ok_or_else(|| {
        receipt_invariant("Coordinator root-removal operation has no reservation authority")
    })?;
    Ok(Some(CoordinatorRootRemovalOperationStatus {
        operation_id,
        reservation: reservation.response.clone(),
        draining: draining.map(|receipt| receipt.response.clone()),
        removal: removal.map(|receipt| receipt.response.clone()),
        readiness_intent: readiness_intent.cloned(),
        readiness: readiness.cloned(),
        execution: execution.cloned(),
        completion: completion.cloned(),
    }))
}

fn unique_operation_match<'a, T: 'a>(
    mut matches: impl Iterator<Item = &'a T>,
) -> Result<Option<&'a T>, InternalError> {
    let first = matches.next();
    if matches.next().is_some() {
        return Err(receipt_invariant(
            "Coordinator operation ID identifies duplicate durable records",
        ));
    }
    Ok(first)
}

#[derive(Clone)]
struct FleetComponentProvisioningRootAcceptanceProgress {
    planned_at_ns: u64,
    phase: FleetComponentProvisioningPhase,
    acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
    accepted_root_count: u32,
    root_batch_count: u32,
    in_flight: Option<FleetComponentProvisioningRootAcceptanceIntentRecord>,
    roots_accepted_at_ns: Option<u64>,
}

#[derive(Clone)]
struct FleetComponentProvisioningRootProvisionProgress {
    provisions: Vec<FleetComponentProvisioningRootProvisionRecord>,
    provisioned_root_count: u32,
    current: Option<FleetComponentProvisioningRootProvisionRecord>,
    current_response: Option<RootComponentProvisioningStatusResponse>,
    in_flight: Option<FleetComponentProvisioningRootProvisionIntentRecord>,
    roots_accepted_at_ns: Option<u64>,
    components_provisioned_at_ns: Option<u64>,
    published_fleet_registry: Option<FleetRegistryVersion>,
    service_topology_published_at_ns: Option<u64>,
}

#[derive(Clone)]
struct FleetComponentDirectoryConfirmationProgress {
    planned_at_ns: u64,
    acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
    roots_accepted_at_ns: u64,
    provisions: Vec<FleetComponentProvisioningRootProvisionRecord>,
    components_provisioned_at_ns: u64,
    published_fleet_registry: FleetRegistryVersion,
    service_topology_published_at_ns: u64,
    confirmations: Vec<FleetComponentDirectoryConfirmationRecord>,
    confirmed_root_count: u32,
    confirmation_root_count: u32,
    current: Option<FleetComponentDirectoryConfirmationRecord>,
    in_flight: Option<FleetComponentDirectoryConfirmationIntentRecord>,
    complete: bool,
}

#[derive(Clone)]
struct FleetComponentRuntimeActivationProgress {
    planned_at_ns: u64,
    acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
    roots_accepted_at_ns: u64,
    provisions: Vec<FleetComponentProvisioningRootProvisionRecord>,
    components_provisioned_at_ns: u64,
    published_fleet_registry: FleetRegistryVersion,
    service_topology_published_at_ns: u64,
    confirmations: Vec<FleetComponentDirectoryConfirmationRecord>,
    directories_confirmed_at_ns: u64,
    activations: Vec<FleetComponentRuntimeActivationRecord>,
    activated_root_count: u32,
    activation_root_count: u32,
    current: Option<FleetComponentRuntimeActivationRecord>,
    in_flight: Option<FleetComponentRuntimeActivationIntentRecord>,
    runtimes_activated_at_ns: Option<u64>,
    complete: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RootAcceptanceAdvance {
    Begin,
    Reconcile,
    Current,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RootProvisionAdvance {
    Begin,
    Reconcile,
    Current,
    Publish,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum DirectoryConfirmationAdvance {
    Begin,
    Reconcile,
    Current,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RuntimeActivationAdvance {
    Begin,
    Reconcile,
    Current,
}

#[derive(Clone, Copy)]
struct FleetComponentProvisioningPlanCounts {
    directory_confirmation_roots: u32,
    root_batches: u32,
    group_placements: u32,
    components: u32,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct FleetComponentProvisioningAuthority {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    configuration_digest: ComponentDeploymentConfigurationDigest,
}

const fn component_provisioning_authority(
    record: &FleetComponentProvisioningRecord,
) -> FleetComponentProvisioningAuthority {
    FleetComponentProvisioningAuthority {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        configuration_digest: record.plan.configuration_digest,
    }
}

const fn service_publication_authority(
    receipt: &FleetServicePublicationReceiptRecord,
) -> FleetComponentProvisioningAuthority {
    FleetComponentProvisioningAuthority {
        operation_id: receipt.operation_id,
        plan_hash: receipt.plan_hash,
        configuration_digest: receipt.configuration_digest,
    }
}

fn validate_service_publication_authority(
    current: &FleetCoordinatorRegistryRecord,
    record: &FleetComponentProvisioningRecord,
) -> Result<(), InternalError> {
    let Some((publication, receipt)) = paired_service_publication_evidence(current, record)? else {
        return Ok(());
    };
    if component_provisioning_authority(record) != service_publication_authority(receipt) {
        return Err(receipt_invariant(
            "Fleet-service publication receipt differs from its provisioning plan",
        ));
    }
    if publication.published_at_ns < publication.components_provisioned_at_ns {
        return Err(receipt_invariant(
            "Fleet-service publication time precedes complete root provisioning",
        ));
    }
    let source_registry = component_operation_source_registry(current, record)?;
    let root_receipts = publication
        .provisions
        .iter()
        .map(|provision| provision.response.clone())
        .collect::<Vec<_>>();
    let receipt_hashes = root_receipts
        .iter()
        .map(|root_receipt| root_receipt.receipt_content_hash)
        .collect::<Vec<_>>();
    let services = compile_component_operation_services(
        &current.component_deployment_configuration,
        &source_registry,
        record,
        &root_receipts,
    )
    .map_err(|_| {
        receipt_invariant("published root provisioning receipts do not compile canonical services")
    })?;
    let receipt_is_exact = [
        receipt.previous_version == record.plan.fleet_registry,
        receipt.version == *publication.published_registry,
        receipt.root_receipt_content_hashes == receipt_hashes,
        receipt.services == services,
    ]
    .into_iter()
    .all(|fact| fact);
    if !receipt_is_exact {
        return Err(receipt_invariant(
            "Fleet-service publication receipt differs from its exact terminal evidence",
        ));
    }
    Ok(())
}

struct FleetServicePublicationState<'a> {
    provisions: &'a [FleetComponentProvisioningRootProvisionRecord],
    components_provisioned_at_ns: u64,
    published_registry: &'a FleetRegistryVersion,
    published_at_ns: u64,
}

fn service_publication_state(
    record: &FleetComponentProvisioningRecord,
) -> Option<FleetServicePublicationState<'_>> {
    match &record.state {
        FleetComponentProvisioningStateRecord::ServiceTopologyPublished {
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::ConfirmingDirectories {
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::DirectoriesConfirmed {
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::ActivatingRuntimes {
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::RuntimesActivated {
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        } => Some(FleetServicePublicationState {
            provisions,
            components_provisioned_at_ns: *components_provisioned_at_ns,
            published_registry: published_fleet_registry,
            published_at_ns: *service_topology_published_at_ns,
        }),
        FleetComponentProvisioningStateRecord::Planned { .. }
        | FleetComponentProvisioningStateRecord::AcceptingRoots { .. }
        | FleetComponentProvisioningStateRecord::RootsAccepted { .. }
        | FleetComponentProvisioningStateRecord::ProvisioningRoots { .. }
        | FleetComponentProvisioningStateRecord::ComponentsProvisioned { .. } => None,
    }
}

fn paired_service_publication_evidence<'a>(
    current: &'a FleetCoordinatorRegistryRecord,
    record: &'a FleetComponentProvisioningRecord,
) -> Result<
    Option<(
        FleetServicePublicationState<'a>,
        &'a FleetServicePublicationReceiptRecord,
    )>,
    InternalError,
> {
    let receipt = service_publication_receipt_for_operation(current, record.operation_id)?;
    match (service_publication_state(record), receipt) {
        (Some(publication), Some(receipt)) => Ok(Some((publication, receipt))),
        (None, None) => Ok(None),
        (Some(_), None) => Err(receipt_invariant(
            "Fleet-service publication state lacks its atomic receipt",
        )),
        (None, Some(_)) => Err(receipt_invariant(
            "Fleet-service publication receipt lacks its atomic state",
        )),
    }
}

fn service_publication_receipt_for_operation(
    current: &FleetCoordinatorRegistryRecord,
    operation_id: [u8; 32],
) -> Result<Option<&FleetServicePublicationReceiptRecord>, InternalError> {
    let mut matches = current
        .service_publication_receipts
        .iter()
        .filter(|receipt| receipt.operation_id == operation_id);
    let receipt = matches.next();
    if matches.next().is_some() {
        return Err(receipt_invariant(
            "Fleet-service publication operation has duplicate receipts",
        ));
    }
    Ok(receipt)
}

fn require_component_provisioning_operation_record<'a>(
    current: &'a FleetCoordinatorRegistryRecord,
    request: &FleetComponentProvisioningAdvanceRequest,
) -> Result<&'a FleetComponentProvisioningRecord, InternalError> {
    active_provisioning_record_for_status(
        current,
        &FleetComponentProvisioningStatusRequest {
            operation_id: request.operation_id,
            plan_hash: request.plan_hash,
        },
    )?
    .ok_or_else(|| receipt_invariant("active Fleet Component operation record disappeared"))
}

fn active_provisioning_record_for_status<'a>(
    current: &'a FleetCoordinatorRegistryRecord,
    request: &FleetComponentProvisioningStatusRequest,
) -> Result<Option<&'a FleetComponentProvisioningRecord>, InternalError> {
    let records = [
        current.component_provisioning.as_ref(),
        current.component_scale_out.as_ref(),
    ];
    if let Some(record) = records
        .into_iter()
        .flatten()
        .find(|record| record.operation_id == request.operation_id)
    {
        if record.plan_hash == request.plan_hash {
            return Ok(Some(record));
        }
        return Err(InternalError::conflict());
    }
    Ok(None)
}

fn component_provisioning_record_mut(
    current: &mut FleetCoordinatorRegistryRecord,
) -> Result<&mut FleetComponentProvisioningRecord, InternalError> {
    current
        .component_provisioning
        .as_mut()
        .ok_or_else(|| receipt_invariant("Fleet Component provisioning record disappeared"))
}

fn component_provisioning_operation_record(
    current: &FleetCoordinatorRegistryRecord,
    operation_id: [u8; 32],
) -> Result<&FleetComponentProvisioningRecord, InternalError> {
    [
        current.component_provisioning.as_ref(),
        current.component_scale_out.as_ref(),
    ]
    .into_iter()
    .flatten()
    .find(|record| record.operation_id == operation_id)
    .ok_or_else(|| receipt_invariant("Fleet Component operation record disappeared"))
}

fn component_provisioning_operation_record_mut(
    current: &mut FleetCoordinatorRegistryRecord,
    operation_id: [u8; 32],
) -> Result<&mut FleetComponentProvisioningRecord, InternalError> {
    if current
        .component_provisioning
        .as_ref()
        .is_some_and(|record| record.operation_id == operation_id)
    {
        return current
            .component_provisioning
            .as_mut()
            .ok_or_else(|| receipt_invariant("Fleet Component operation record disappeared"));
    }
    current
        .component_scale_out
        .as_mut()
        .filter(|record| record.operation_id == operation_id)
        .ok_or_else(|| receipt_invariant("Fleet Component operation record disappeared"))
}

#[derive(Clone)]
struct ComponentsProvisionedState {
    planned_at_ns: u64,
    acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
    roots_accepted_at_ns: u64,
    provisions: Vec<FleetComponentProvisioningRootProvisionRecord>,
    components_provisioned_at_ns: u64,
}

struct ServicePublication {
    registry: FleetRegistry,
    receipt: FleetServicePublicationReceiptRecord,
}

fn components_provisioned_state(
    record: &FleetComponentProvisioningRecord,
) -> Result<ComponentsProvisionedState, InternalError> {
    let FleetComponentProvisioningStateRecord::ComponentsProvisioned {
        planned_at_ns,
        acceptances,
        roots_accepted_at_ns,
        provisions,
        components_provisioned_at_ns,
    } = &record.state
    else {
        return Err(receipt_invariant(
            "Fleet-service publication disposition lacks ComponentsProvisioned state",
        ));
    };
    Ok(ComponentsProvisionedState {
        planned_at_ns: *planned_at_ns,
        acceptances: acceptances.clone(),
        roots_accepted_at_ns: *roots_accepted_at_ns,
        provisions: provisions.clone(),
        components_provisioned_at_ns: *components_provisioned_at_ns,
    })
}

fn compile_service_publication(
    current: &FleetCoordinatorRegistryRecord,
    record: &FleetComponentProvisioningRecord,
    provisioned: &ComponentsProvisionedState,
) -> Result<ServicePublication, InternalError> {
    let source_registry = component_operation_source_registry(current, record)?;
    if current.registry != source_registry {
        return Err(InternalError::conflict());
    }
    if service_publication_receipt_for_operation(current, record.operation_id)?.is_some() {
        return Err(receipt_invariant(
            "ComponentsProvisioned state already contains Fleet-service publication evidence",
        ));
    }
    let root_receipts = provisioned
        .provisions
        .iter()
        .map(|provision| provision.response.clone())
        .collect::<Vec<_>>();
    let services = compile_component_operation_services(
        &current.component_deployment_configuration,
        &source_registry,
        record,
        &root_receipts,
    )?;
    let topology = &current
        .component_deployment_configuration
        .component_topology;
    let previous_version =
        FleetRegistryOps::version(&current.authority, topology, &current.registry)?;
    if previous_version != record.plan.fleet_registry {
        return Err(InternalError::conflict());
    }
    let registry = if services == current.registry.services {
        current.registry.clone()
    } else {
        match record.plan.operation {
            FleetComponentProvisioningOperation::FreshInstall => {
                FleetRegistryOps::compile_initial_services(
                    &current.authority,
                    topology,
                    &current.registry,
                    services.clone(),
                )?
            }
            FleetComponentProvisioningOperation::ScaleOut { .. } => {
                FleetRegistryOps::compile_service_additions(
                    &current.authority,
                    topology,
                    &current.registry,
                    services.clone(),
                )?
            }
        }
    };
    let version = FleetRegistryOps::version(&current.authority, topology, &registry)?;
    let root_receipt_content_hashes = root_receipts
        .iter()
        .map(|receipt| receipt.receipt_content_hash)
        .collect();
    Ok(ServicePublication {
        registry,
        receipt: FleetServicePublicationReceiptRecord {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            configuration_digest: record.plan.configuration_digest,
            root_receipt_content_hashes,
            services,
            previous_version,
            version,
        },
    })
}

fn compile_component_operation_services(
    configuration: &canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
    source_registry: &FleetRegistry,
    record: &FleetComponentProvisioningRecord,
    root_receipts: &[RootComponentProvisioningStatusResponse],
) -> Result<Vec<canic_core::dto::fleet_registry::FleetServiceBinding>, InternalError> {
    match record.plan.operation {
        FleetComponentProvisioningOperation::FreshInstall => {
            FleetServiceBindingOps::compile_initial_compiled(
                configuration,
                source_registry,
                &record.plan,
                record.operation_id,
                root_receipts,
            )
        }
        FleetComponentProvisioningOperation::ScaleOut { .. } => {
            FleetServiceBindingOps::compile_scale_out_compiled(
                configuration,
                source_registry,
                &record.plan,
                record.operation_id,
                record.plan_hash,
                root_receipts,
            )
        }
    }
}

fn component_provisioning_root_acceptance_progress(
    record: &FleetComponentProvisioningRecord,
) -> Result<FleetComponentProvisioningRootAcceptanceProgress, InternalError> {
    let root_batch_count = u32::try_from(record.plan.batches.len())
        .map_err(|_| receipt_invariant("root batch count does not fit u32"))?;
    match &record.state {
        FleetComponentProvisioningStateRecord::Planned { planned_at_ns } => {
            planned_root_acceptance_progress(*planned_at_ns, root_batch_count)
        }
        FleetComponentProvisioningStateRecord::AcceptingRoots {
            planned_at_ns,
            acceptances,
            in_flight,
        } => root_acceptance_progress_from_parts(
            *planned_at_ns,
            FleetComponentProvisioningPhase::AcceptingRoots,
            acceptances,
            *in_flight,
            None,
            root_batch_count,
        ),
        state => {
            let authority = post_acceptance_authority(state);
            root_acceptance_progress_from_parts(
                authority.planned_at_ns,
                authority.phase,
                authority.acceptances,
                None,
                Some(authority.roots_accepted_at_ns),
                root_batch_count,
            )
        }
    }
}

struct PostAcceptanceAuthority<'a> {
    planned_at_ns: u64,
    phase: FleetComponentProvisioningPhase,
    acceptances: &'a [FleetComponentProvisioningRootAcceptanceRecord],
    roots_accepted_at_ns: u64,
}

fn post_acceptance_authority(
    state: &FleetComponentProvisioningStateRecord,
) -> PostAcceptanceAuthority<'_> {
    let (planned_at_ns, acceptances, roots_accepted_at_ns) = match state {
        FleetComponentProvisioningStateRecord::RootsAccepted {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
        }
        | FleetComponentProvisioningStateRecord::ProvisioningRoots {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::ComponentsProvisioned {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::ServiceTopologyPublished {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::ConfirmingDirectories {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::DirectoriesConfirmed {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::ActivatingRuntimes {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::RuntimesActivated {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            ..
        } => (
            *planned_at_ns,
            acceptances.as_slice(),
            *roots_accepted_at_ns,
        ),
        _ => unreachable!("pre-acceptance states are handled by the caller"),
    };
    let phase = match state {
        FleetComponentProvisioningStateRecord::RootsAccepted { .. } => {
            FleetComponentProvisioningPhase::RootsAccepted
        }
        FleetComponentProvisioningStateRecord::ProvisioningRoots { .. } => {
            FleetComponentProvisioningPhase::ProvisioningRoots
        }
        FleetComponentProvisioningStateRecord::ComponentsProvisioned { .. } => {
            FleetComponentProvisioningPhase::ComponentsProvisioned
        }
        FleetComponentProvisioningStateRecord::ServiceTopologyPublished { .. } => {
            FleetComponentProvisioningPhase::ServiceTopologyPublished
        }
        FleetComponentProvisioningStateRecord::ConfirmingDirectories { .. } => {
            FleetComponentProvisioningPhase::ConfirmingDirectories
        }
        FleetComponentProvisioningStateRecord::DirectoriesConfirmed { .. } => {
            FleetComponentProvisioningPhase::DirectoriesConfirmed
        }
        FleetComponentProvisioningStateRecord::ActivatingRuntimes { .. } => {
            FleetComponentProvisioningPhase::ActivatingRuntimes
        }
        FleetComponentProvisioningStateRecord::RuntimesActivated { .. } => {
            FleetComponentProvisioningPhase::RuntimesActivated
        }
        _ => unreachable!("pre-acceptance states are handled by the caller"),
    };
    PostAcceptanceAuthority {
        planned_at_ns,
        phase,
        acceptances,
        roots_accepted_at_ns,
    }
}

fn root_acceptance_progress_from_parts(
    planned_at_ns: u64,
    phase: FleetComponentProvisioningPhase,
    acceptances: &[FleetComponentProvisioningRootAcceptanceRecord],
    in_flight: Option<FleetComponentProvisioningRootAcceptanceIntentRecord>,
    roots_accepted_at_ns: Option<u64>,
    root_batch_count: u32,
) -> Result<FleetComponentProvisioningRootAcceptanceProgress, InternalError> {
    let accepted_root_count = u32::try_from(acceptances.len())
        .map_err(|_| receipt_invariant("accepted root count does not fit u32"))?;
    Ok(FleetComponentProvisioningRootAcceptanceProgress {
        planned_at_ns,
        phase,
        acceptances: acceptances.to_vec(),
        accepted_root_count,
        root_batch_count,
        in_flight,
        roots_accepted_at_ns,
    })
}

fn planned_root_acceptance_progress(
    planned_at_ns: u64,
    root_batch_count: u32,
) -> Result<FleetComponentProvisioningRootAcceptanceProgress, InternalError> {
    root_acceptance_progress_from_parts(
        planned_at_ns,
        FleetComponentProvisioningPhase::Planned,
        &[],
        None,
        None,
        root_batch_count,
    )
}

fn component_provisioning_root_provision_progress(
    record: &FleetComponentProvisioningRecord,
) -> Result<FleetComponentProvisioningRootProvisionProgress, InternalError> {
    match &record.state {
        FleetComponentProvisioningStateRecord::Planned { .. }
        | FleetComponentProvisioningStateRecord::AcceptingRoots { .. } => {
            Ok(empty_root_provision_progress())
        }
        FleetComponentProvisioningStateRecord::RootsAccepted {
            acceptances,
            roots_accepted_at_ns,
            ..
        } => Ok(accepted_root_provision_progress(
            acceptances,
            *roots_accepted_at_ns,
        )),
        FleetComponentProvisioningStateRecord::ProvisioningRoots {
            acceptances,
            roots_accepted_at_ns,
            provisions,
            current,
            in_flight,
            ..
        } => active_root_provision_progress(
            acceptances,
            *roots_accepted_at_ns,
            provisions,
            current.as_deref(),
            in_flight.as_ref(),
        ),
        FleetComponentProvisioningStateRecord::ComponentsProvisioned {
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            ..
        } => terminal_root_provision_progress(
            provisions,
            *roots_accepted_at_ns,
            *components_provisioned_at_ns,
            None,
        ),
        FleetComponentProvisioningStateRecord::ServiceTopologyPublished {
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::ConfirmingDirectories {
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::DirectoriesConfirmed {
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::ActivatingRuntimes {
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::RuntimesActivated {
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            ..
        } => terminal_root_provision_progress(
            provisions,
            *roots_accepted_at_ns,
            *components_provisioned_at_ns,
            Some((published_fleet_registry, *service_topology_published_at_ns)),
        ),
    }
}

const fn empty_root_provision_progress() -> FleetComponentProvisioningRootProvisionProgress {
    FleetComponentProvisioningRootProvisionProgress {
        provisions: Vec::new(),
        provisioned_root_count: 0,
        current: None,
        current_response: None,
        in_flight: None,
        roots_accepted_at_ns: None,
        components_provisioned_at_ns: None,
        published_fleet_registry: None,
        service_topology_published_at_ns: None,
    }
}

fn accepted_root_provision_progress(
    acceptances: &[FleetComponentProvisioningRootAcceptanceRecord],
    roots_accepted_at_ns: u64,
) -> FleetComponentProvisioningRootProvisionProgress {
    FleetComponentProvisioningRootProvisionProgress {
        current_response: acceptances.first().map(|record| record.response.clone()),
        roots_accepted_at_ns: Some(roots_accepted_at_ns),
        ..empty_root_provision_progress()
    }
}

fn active_root_provision_progress(
    acceptances: &[FleetComponentProvisioningRootAcceptanceRecord],
    roots_accepted_at_ns: u64,
    provisions: &[FleetComponentProvisioningRootProvisionRecord],
    current: Option<&FleetComponentProvisioningRootProvisionRecord>,
    in_flight: Option<&FleetComponentProvisioningRootProvisionIntentRecord>,
) -> Result<FleetComponentProvisioningRootProvisionProgress, InternalError> {
    let provisioned_root_count = u32::try_from(provisions.len())
        .map_err(|_| receipt_invariant("provisioned root count does not fit u32"))?;
    let current_response = current.map_or_else(
        || {
            acceptances
                .get(provisions.len())
                .map(|record| record.response.clone())
        },
        |record| Some(record.response.clone()),
    );
    Ok(FleetComponentProvisioningRootProvisionProgress {
        provisions: provisions.to_vec(),
        provisioned_root_count,
        current: current.cloned(),
        current_response,
        in_flight: in_flight.cloned(),
        roots_accepted_at_ns: Some(roots_accepted_at_ns),
        components_provisioned_at_ns: None,
        published_fleet_registry: None,
        service_topology_published_at_ns: None,
    })
}

fn terminal_root_provision_progress(
    provisions: &[FleetComponentProvisioningRootProvisionRecord],
    roots_accepted_at_ns: u64,
    components_provisioned_at_ns: u64,
    publication: Option<(&FleetRegistryVersion, u64)>,
) -> Result<FleetComponentProvisioningRootProvisionProgress, InternalError> {
    let provisioned_root_count = u32::try_from(provisions.len())
        .map_err(|_| receipt_invariant("provisioned root count does not fit u32"))?;
    Ok(FleetComponentProvisioningRootProvisionProgress {
        provisions: provisions.to_vec(),
        provisioned_root_count,
        current: None,
        current_response: None,
        in_flight: None,
        roots_accepted_at_ns: Some(roots_accepted_at_ns),
        components_provisioned_at_ns: Some(components_provisioned_at_ns),
        published_fleet_registry: publication.map(|(version, _)| version.clone()),
        service_topology_published_at_ns: publication.map(|(_, published_at_ns)| published_at_ns),
    })
}

fn component_directory_confirmation_progress(
    record: &FleetComponentProvisioningRecord,
) -> Result<FleetComponentDirectoryConfirmationProgress, InternalError> {
    let confirmation_root_count = u32::try_from(record.plan.directory_confirmation_roots.len())
        .map_err(|_| receipt_invariant("Directory confirmation root count does not fit u32"))?;
    let progress = match &record.state {
        FleetComponentProvisioningStateRecord::ServiceTopologyPublished {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
        } => FleetComponentDirectoryConfirmationProgress {
            planned_at_ns: *planned_at_ns,
            acceptances: acceptances.clone(),
            roots_accepted_at_ns: *roots_accepted_at_ns,
            provisions: provisions.clone(),
            components_provisioned_at_ns: *components_provisioned_at_ns,
            published_fleet_registry: published_fleet_registry.clone(),
            service_topology_published_at_ns: *service_topology_published_at_ns,
            confirmations: vec![],
            confirmed_root_count: 0,
            confirmation_root_count,
            current: None,
            in_flight: None,
            complete: false,
        },
        FleetComponentProvisioningStateRecord::ConfirmingDirectories {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            confirmations,
            current,
            in_flight,
        } => FleetComponentDirectoryConfirmationProgress {
            planned_at_ns: *planned_at_ns,
            acceptances: acceptances.clone(),
            roots_accepted_at_ns: *roots_accepted_at_ns,
            provisions: provisions.clone(),
            components_provisioned_at_ns: *components_provisioned_at_ns,
            published_fleet_registry: published_fleet_registry.clone(),
            service_topology_published_at_ns: *service_topology_published_at_ns,
            confirmations: confirmations.clone(),
            confirmed_root_count: u32::try_from(confirmations.len())
                .map_err(|_| receipt_invariant("Directory confirmation count does not fit u32"))?,
            confirmation_root_count,
            current: current.as_deref().cloned(),
            in_flight: in_flight.as_deref().cloned(),
            complete: false,
        },
        FleetComponentProvisioningStateRecord::DirectoriesConfirmed {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            confirmations,
            ..
        } => FleetComponentDirectoryConfirmationProgress {
            planned_at_ns: *planned_at_ns,
            acceptances: acceptances.clone(),
            roots_accepted_at_ns: *roots_accepted_at_ns,
            provisions: provisions.clone(),
            components_provisioned_at_ns: *components_provisioned_at_ns,
            published_fleet_registry: published_fleet_registry.clone(),
            service_topology_published_at_ns: *service_topology_published_at_ns,
            confirmations: confirmations.clone(),
            confirmed_root_count: u32::try_from(confirmations.len())
                .map_err(|_| receipt_invariant("Directory confirmation count does not fit u32"))?,
            confirmation_root_count,
            current: None,
            in_flight: None,
            complete: true,
        },
        state @ (FleetComponentProvisioningStateRecord::ActivatingRuntimes { .. }
        | FleetComponentProvisioningStateRecord::RuntimesActivated { .. }) => {
            terminal_downstream_directory_progress(state, confirmation_root_count)?
        }
        _ => {
            return Err(InternalError::conflict());
        }
    };
    Ok(progress)
}

fn terminal_downstream_directory_progress(
    state: &FleetComponentProvisioningStateRecord,
    confirmation_root_count: u32,
) -> Result<FleetComponentDirectoryConfirmationProgress, InternalError> {
    let (
        planned_at_ns,
        acceptances,
        roots_accepted_at_ns,
        provisions,
        components_provisioned_at_ns,
        published_fleet_registry,
        service_topology_published_at_ns,
        confirmations,
    ) = match state {
        FleetComponentProvisioningStateRecord::ActivatingRuntimes {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            confirmations,
            ..
        }
        | FleetComponentProvisioningStateRecord::RuntimesActivated {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            confirmations,
            ..
        } => (
            *planned_at_ns,
            acceptances,
            *roots_accepted_at_ns,
            provisions,
            *components_provisioned_at_ns,
            published_fleet_registry,
            *service_topology_published_at_ns,
            confirmations,
        ),
        _ => unreachable!("only downstream Directory states delegate here"),
    };
    Ok(FleetComponentDirectoryConfirmationProgress {
        planned_at_ns,
        acceptances: acceptances.clone(),
        roots_accepted_at_ns,
        provisions: provisions.clone(),
        components_provisioned_at_ns,
        published_fleet_registry: published_fleet_registry.clone(),
        service_topology_published_at_ns,
        confirmations: confirmations.clone(),
        confirmed_root_count: u32::try_from(confirmations.len())
            .map_err(|_| receipt_invariant("Directory confirmation count does not fit u32"))?,
        confirmation_root_count,
        current: None,
        in_flight: None,
        complete: true,
    })
}

struct RuntimeActivationAuthority {
    planned_at_ns: u64,
    acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
    roots_accepted_at_ns: u64,
    provisions: Vec<FleetComponentProvisioningRootProvisionRecord>,
    components_provisioned_at_ns: u64,
    published_fleet_registry: FleetRegistryVersion,
    service_topology_published_at_ns: u64,
    confirmations: Vec<FleetComponentDirectoryConfirmationRecord>,
    directories_confirmed_at_ns: u64,
}

fn runtime_activation_authority(
    record: &FleetComponentProvisioningRecord,
) -> Result<RuntimeActivationAuthority, InternalError> {
    let (
        planned_at_ns,
        acceptances,
        roots_accepted_at_ns,
        provisions,
        components_provisioned_at_ns,
        published_fleet_registry,
        service_topology_published_at_ns,
        confirmations,
        directories_confirmed_at_ns,
    ) = match &record.state {
        FleetComponentProvisioningStateRecord::DirectoriesConfirmed {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            confirmations,
            directories_confirmed_at_ns,
        }
        | FleetComponentProvisioningStateRecord::ActivatingRuntimes {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            confirmations,
            directories_confirmed_at_ns,
            ..
        }
        | FleetComponentProvisioningStateRecord::RuntimesActivated {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
            provisions,
            components_provisioned_at_ns,
            published_fleet_registry,
            service_topology_published_at_ns,
            confirmations,
            directories_confirmed_at_ns,
            ..
        } => (
            *planned_at_ns,
            acceptances.clone(),
            *roots_accepted_at_ns,
            provisions.clone(),
            *components_provisioned_at_ns,
            published_fleet_registry.clone(),
            *service_topology_published_at_ns,
            confirmations.clone(),
            *directories_confirmed_at_ns,
        ),
        _ => {
            return Err(InternalError::conflict());
        }
    };
    Ok(RuntimeActivationAuthority {
        planned_at_ns,
        acceptances,
        roots_accepted_at_ns,
        provisions,
        components_provisioned_at_ns,
        published_fleet_registry,
        service_topology_published_at_ns,
        confirmations,
        directories_confirmed_at_ns,
    })
}

fn component_runtime_activation_progress(
    record: &FleetComponentProvisioningRecord,
) -> Result<FleetComponentRuntimeActivationProgress, InternalError> {
    let authority = runtime_activation_authority(record)?;
    let activation_root_count = u32::try_from(record.plan.batches.len())
        .map_err(|_| receipt_invariant("runtime activation root count does not fit u32"))?;
    let (activations, current, in_flight, runtimes_activated_at_ns, complete) = match &record.state
    {
        FleetComponentProvisioningStateRecord::DirectoriesConfirmed { .. } => {
            (vec![], None, None, None, false)
        }
        FleetComponentProvisioningStateRecord::ActivatingRuntimes {
            activations,
            current,
            in_flight,
            ..
        } => (
            activations.clone(),
            current.as_deref().copied(),
            *in_flight,
            None,
            false,
        ),
        FleetComponentProvisioningStateRecord::RuntimesActivated {
            activations,
            runtimes_activated_at_ns,
            ..
        } => (
            activations.clone(),
            None,
            None,
            Some(*runtimes_activated_at_ns),
            true,
        ),
        _ => unreachable!("runtime activation authority rejected earlier phases"),
    };
    let activated_root_count = u32::try_from(activations.len())
        .map_err(|_| receipt_invariant("runtime-activated root count does not fit u32"))?;
    Ok(FleetComponentRuntimeActivationProgress {
        planned_at_ns: authority.planned_at_ns,
        acceptances: authority.acceptances,
        roots_accepted_at_ns: authority.roots_accepted_at_ns,
        provisions: authority.provisions,
        components_provisioned_at_ns: authority.components_provisioned_at_ns,
        published_fleet_registry: authority.published_fleet_registry,
        service_topology_published_at_ns: authority.service_topology_published_at_ns,
        confirmations: authority.confirmations,
        directories_confirmed_at_ns: authority.directories_confirmed_at_ns,
        activations,
        activated_root_count,
        activation_root_count,
        current,
        in_flight,
        runtimes_activated_at_ns,
        complete,
    })
}

fn classify_runtime_activation_advance(
    request: &FleetComponentProvisioningAdvanceRequest,
    progress: &FleetComponentRuntimeActivationProgress,
) -> Result<RuntimeActivationAdvance, InternalError> {
    if progress.complete {
        return if runtime_activation_request_is_current(request, progress)
            || terminal_runtime_activation_replay(request, progress)?
        {
            Ok(RuntimeActivationAdvance::Current)
        } else {
            Err(InternalError::conflict())
        };
    }
    if request.expected_runtime_activated_root_count < progress.activated_root_count {
        return if terminal_runtime_activation_replay(request, progress)? {
            Ok(RuntimeActivationAdvance::Current)
        } else {
            Err(InternalError::conflict())
        };
    }
    if request.expected_runtime_activated_root_count != progress.activated_root_count {
        return Err(InternalError::conflict());
    }
    let actual = progress.current.map(|record| record.progress);
    if request.expected_current_activation != actual {
        let replays_last = request
            .expected_current_activation
            .zip(actual)
            .is_some_and(|(expected, actual)| activation_progress_advances(expected, actual));
        let replays_first = request.expected_current_activation.is_none()
            && actual.is_some_and(first_component_activation_progress);
        return if replays_last || replays_first {
            Ok(RuntimeActivationAdvance::Current)
        } else {
            Err(InternalError::conflict())
        };
    }
    if progress.in_flight.is_some() {
        Ok(RuntimeActivationAdvance::Reconcile)
    } else {
        Ok(RuntimeActivationAdvance::Begin)
    }
}

const fn first_component_activation_progress(actual: FleetComponentActivationRootProgress) -> bool {
    actual.component_count > 0
        && actual.activated_component_count == 1
        && !actual.root_runtime_active
}

const fn runtime_activation_request_is_current(
    request: &FleetComponentProvisioningAdvanceRequest,
    progress: &FleetComponentRuntimeActivationProgress,
) -> bool {
    request.expected_runtime_activated_root_count == progress.activated_root_count
        && request.expected_current_activation.is_none()
}

fn terminal_runtime_activation_replay(
    request: &FleetComponentProvisioningAdvanceRequest,
    progress: &FleetComponentRuntimeActivationProgress,
) -> Result<bool, InternalError> {
    if request.expected_runtime_activated_root_count.checked_add(1)
        != Some(progress.activated_root_count)
    {
        return Ok(false);
    }
    let terminal = progress
        .activations
        .last()
        .ok_or_else(|| receipt_invariant("terminal runtime activation lacks a root receipt"))?;
    Ok(request
        .expected_current_activation
        .map_or(terminal.progress.component_count == 0, |expected| {
            activation_progress_advances(expected, terminal.progress)
        }))
}

fn activation_progress_advances(
    expected: FleetComponentActivationRootProgress,
    actual: FleetComponentActivationRootProgress,
) -> bool {
    if expected.fleet_subnet_root != actual.fleet_subnet_root
        || expected.component_count != actual.component_count
    {
        return false;
    }
    let component_advances = !expected.root_runtime_active
        && !actual.root_runtime_active
        && expected.activated_component_count.checked_add(1)
            == Some(actual.activated_component_count);
    let root_advances = !expected.root_runtime_active
        && actual.root_runtime_active
        && expected.activated_component_count == actual.activated_component_count;
    component_advances || root_advances
}

fn classify_directory_confirmation_advance(
    request: &FleetComponentProvisioningAdvanceRequest,
    progress: &FleetComponentDirectoryConfirmationProgress,
) -> Result<DirectoryConfirmationAdvance, InternalError> {
    if progress.complete {
        let current_is_exact = request.expected_directory_confirmed_root_count
            == progress.confirmed_root_count
            && request.expected_current_synchronization.is_none()
            && request.expected_current_publication.is_none();
        let replays_terminal_call = terminal_directory_confirmation_replay(request, progress)?;
        return if current_is_exact || replays_terminal_call {
            Ok(DirectoryConfirmationAdvance::Current)
        } else {
            Err(InternalError::conflict())
        };
    }
    if request.expected_directory_confirmed_root_count < progress.confirmed_root_count {
        return if request
            .expected_directory_confirmed_root_count
            .checked_add(1)
            == Some(progress.confirmed_root_count)
        {
            Ok(DirectoryConfirmationAdvance::Current)
        } else {
            Err(InternalError::conflict())
        };
    }
    if request.expected_directory_confirmed_root_count != progress.confirmed_root_count {
        return Err(InternalError::conflict());
    }
    let actual_synchronization = progress
        .current
        .as_ref()
        .and_then(confirmation_synchronization_progress);
    if request.expected_current_synchronization != actual_synchronization {
        if synchronization_progress_replays(
            request.expected_current_synchronization,
            actual_synchronization,
        ) {
            return Ok(DirectoryConfirmationAdvance::Current);
        }
        return Err(InternalError::conflict());
    }
    let actual_current = progress
        .current
        .as_ref()
        .and_then(confirmation_publication_response)
        .map(root_publication_progress);
    if request.expected_current_publication != actual_current {
        if publication_progress_replays(request.expected_current_publication, actual_current) {
            return Ok(DirectoryConfirmationAdvance::Current);
        }
        return Err(InternalError::conflict());
    }
    if progress.in_flight.is_some() {
        Ok(DirectoryConfirmationAdvance::Reconcile)
    } else {
        Ok(DirectoryConfirmationAdvance::Begin)
    }
}

fn terminal_directory_confirmation_replay(
    request: &FleetComponentProvisioningAdvanceRequest,
    progress: &FleetComponentDirectoryConfirmationProgress,
) -> Result<bool, InternalError> {
    if request
        .expected_directory_confirmed_root_count
        .checked_add(1)
        != Some(progress.confirmed_root_count)
    {
        return Ok(false);
    }
    let terminal = progress
        .confirmations
        .last()
        .ok_or_else(|| receipt_invariant("terminal Directory confirmation lacks a root receipt"))?;
    let terminal_progress =
        confirmation_publication_response(terminal).map(root_publication_progress);
    let terminal_synchronization = confirmation_synchronization_progress(terminal);
    let synchronization_replays = request.expected_current_synchronization
        == terminal_synchronization
        || synchronization_progress_replays(
            request.expected_current_synchronization,
            terminal_synchronization,
        );
    let publication_replays = request.expected_current_publication == terminal_progress
        || publication_progress_replays(request.expected_current_publication, terminal_progress);
    Ok(synchronization_replays && publication_replays)
}

const fn root_synchronization_progress(
    response: &RootComponentDirectorySynchronizationResponse,
) -> FleetComponentSynchronizationRootProgress {
    FleetComponentSynchronizationRootProgress {
        fleet_subnet_root: response.fleet_subnet_root,
        affected_component_count: response.affected_component_count,
        synchronized_component_count: response.synchronized_component_count,
        complete: response.complete,
    }
}

fn confirmation_synchronization_progress(
    confirmation: &FleetComponentDirectoryConfirmationRecord,
) -> Option<FleetComponentSynchronizationRootProgress> {
    match confirmation {
        FleetComponentDirectoryConfirmationRecord::FreshPublication { .. } => None,
        FleetComponentDirectoryConfirmationRecord::ScaleOut {
            synchronization, ..
        } => Some(root_synchronization_progress(synchronization)),
    }
}

fn synchronization_progress_replays(
    expected: Option<FleetComponentSynchronizationRootProgress>,
    actual: Option<FleetComponentSynchronizationRootProgress>,
) -> bool {
    let Some(actual) = actual else {
        return false;
    };
    let Some(expected) = expected else {
        return actual.synchronized_component_count <= 1;
    };
    if expected.fleet_subnet_root != actual.fleet_subnet_root
        || expected.affected_component_count != actual.affected_component_count
    {
        return false;
    }
    let component_advances = !expected.complete
        && expected.synchronized_component_count.checked_add(1)
            == Some(actual.synchronized_component_count);
    let terminal_advances = !expected.complete
        && actual.complete
        && expected.synchronized_component_count == actual.synchronized_component_count;
    component_advances || terminal_advances
}

fn publication_progress_replays(
    expected: Option<FleetComponentPublicationRootProgress>,
    actual: Option<FleetComponentPublicationRootProgress>,
) -> bool {
    let Some(actual) = actual else {
        return false;
    };
    let Some(expected) = expected else {
        return actual.published_component_count == 1;
    };
    expected.fleet_subnet_root == actual.fleet_subnet_root
        && expected.component_count == actual.component_count
        && expected.published_component_count.checked_add(1)
            == Some(actual.published_component_count)
}

const fn root_publication_progress(
    response: &RootComponentProvisioningStatusResponse,
) -> FleetComponentPublicationRootProgress {
    FleetComponentPublicationRootProgress {
        fleet_subnet_root: response.fleet_subnet_root,
        component_count: response.component_count,
        published_component_count: response.published_component_count,
    }
}

const fn root_provisioning_progress(
    response: &RootComponentProvisioningStatusResponse,
) -> FleetComponentProvisioningRootProgress {
    FleetComponentProvisioningRootProgress {
        fleet_subnet_root: response.fleet_subnet_root,
        component_count: response.component_count,
        reserved_component_count: response.reserved_component_count,
        claimed_component_count: response.claimed_component_count,
        installed_component_count: response.installed_component_count,
        registry_committed_component_count: response.registry_committed_component_count,
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct RootProvisioningCounts {
    reserved: u32,
    claimed: u32,
    installed: u32,
    registry_committed: u32,
}

impl RootProvisioningCounts {
    const fn from_response(response: &RootComponentProvisioningStatusResponse) -> Self {
        Self {
            reserved: response.reserved_component_count,
            claimed: response.claimed_component_count,
            installed: response.installed_component_count,
            registry_committed: response.registry_committed_component_count,
        }
    }

    const fn from_progress(progress: FleetComponentProvisioningRootProgress) -> Self {
        Self {
            reserved: progress.reserved_component_count,
            claimed: progress.claimed_component_count,
            installed: progress.installed_component_count,
            registry_committed: progress.registry_committed_component_count,
        }
    }

    const fn is_terminal(self, component_count: u32) -> bool {
        self.reserved == component_count
            && self.claimed == component_count
            && self.installed == component_count
            && self.registry_committed == component_count
    }

    fn is_canonical(self, component_count: u32) -> bool {
        let counts_are_bounded = [
            self.reserved <= component_count,
            self.claimed <= component_count,
            self.installed <= component_count,
            self.registry_committed <= component_count,
        ]
        .into_iter()
        .all(|fact| fact);
        let phases_are_ordered = [
            stage_follows_complete_predecessor(self.claimed, self.reserved, component_count),
            stage_follows_complete_predecessor(self.installed, self.claimed, component_count),
            stage_follows_complete_predecessor(
                self.registry_committed,
                self.installed,
                component_count,
            ),
        ]
        .into_iter()
        .all(|fact| fact);
        counts_are_bounded && phases_are_ordered
    }

    fn advances_one_step_to(self, next: Self, component_count: u32) -> bool {
        let states_are_canonical = [
            self.is_canonical(component_count),
            next.is_canonical(component_count),
        ]
        .into_iter()
        .all(|fact| fact);
        if !states_are_canonical {
            return false;
        }
        let reservation_advances = [
            self.claimed == 0,
            self.installed == 0,
            self.registry_committed == 0,
            next.claimed == 0,
            next.installed == 0,
            next.registry_committed == 0,
            self.reserved.checked_add(1) == Some(next.reserved),
        ]
        .into_iter()
        .all(|fact| fact);
        let claim_advances = [
            self.reserved == next.reserved,
            self.installed == 0,
            self.registry_committed == 0,
            next.installed == 0,
            next.registry_committed == 0,
            self.claimed.checked_add(1) == Some(next.claimed),
        ]
        .into_iter()
        .all(|fact| fact);
        let install_advances = [
            self.reserved == next.reserved,
            self.claimed == next.claimed,
            self.registry_committed == 0,
            next.registry_committed == 0,
            self.installed.checked_add(1) == Some(next.installed),
        ]
        .into_iter()
        .all(|fact| fact);
        let registry_advances = [
            self.reserved == next.reserved,
            self.claimed == next.claimed,
            self.installed == next.installed,
            self.registry_committed.checked_add(1) == Some(next.registry_committed),
        ]
        .into_iter()
        .all(|fact| fact);
        [
            reservation_advances,
            claim_advances,
            install_advances,
            registry_advances,
        ]
        .into_iter()
        .any(|advances| advances)
    }
}

const fn stage_follows_complete_predecessor(
    stage: u32,
    predecessor: u32,
    component_count: u32,
) -> bool {
    stage == 0 || predecessor == component_count
}

fn classify_root_provision_advance(
    request: &FleetComponentProvisioningAdvanceRequest,
    progress: &FleetComponentProvisioningRootProvisionProgress,
) -> Result<RootProvisionAdvance, InternalError> {
    if request.expected_provisioned_root_count == progress.provisioned_root_count {
        let Some(current) = progress.current_response.as_ref() else {
            if request.expected_current_root.is_none()
                && progress.components_provisioned_at_ns.is_some()
            {
                return Ok(if progress.service_topology_published_at_ns.is_some() {
                    RootProvisionAdvance::Current
                } else {
                    RootProvisionAdvance::Publish
                });
            }
            return Err(InternalError::conflict());
        };
        let actual = root_provisioning_progress(current);
        if request.expected_current_root.as_ref() == Some(&actual) {
            return Ok(if progress.in_flight.is_some() {
                RootProvisionAdvance::Reconcile
            } else {
                RootProvisionAdvance::Begin
            });
        }
        if let Some(expected) = request.expected_current_root
            && expected.fleet_subnet_root == actual.fleet_subnet_root
            && expected.component_count == actual.component_count
            && RootProvisioningCounts::from_progress(expected).advances_one_step_to(
                RootProvisioningCounts::from_progress(actual),
                actual.component_count,
            )
        {
            return Ok(RootProvisionAdvance::Current);
        }
        return Err(InternalError::conflict());
    }
    if request.expected_provisioned_root_count.checked_add(1)
        == Some(progress.provisioned_root_count)
    {
        let index = usize::try_from(request.expected_provisioned_root_count)
            .map_err(|_| InternalError::resource_exhausted())?;
        let provision = progress.provisions.get(index).ok_or_else(|| {
            receipt_invariant("terminal root provisioning receipt is absent at its cursor")
        })?;
        if request.expected_current_root.as_ref()
            == Some(&root_provisioning_progress(&provision.response))
        {
            return Ok(RootProvisionAdvance::Current);
        }
    }
    Err(InternalError::conflict())
}

fn root_provision_call(
    record: &FleetComponentProvisioningRecord,
    root_index: u32,
    response: &RootComponentProvisioningStatusResponse,
) -> Result<FleetComponentProvisioningRootProvisionCallView, InternalError> {
    let batch = root_batch(record, root_index)?;
    if response.fleet_subnet_root != batch.root.fleet_subnet_root
        || response.phase != RootComponentProvisioningPhase::Accepted
    {
        return Err(receipt_invariant(
            "current root provisioning cursor differs from its plan batch",
        ));
    }
    Ok(FleetComponentProvisioningRootProvisionCallView {
        fleet_subnet_root: batch.root.fleet_subnet_root,
        request: RootComponentProvisioningAdvanceRequest {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            expected_reserved_component_count: response.reserved_component_count,
            expected_claimed_component_count: response.claimed_component_count,
            expected_installed_component_count: response.installed_component_count,
            expected_registry_committed_component_count: response
                .registry_committed_component_count,
        },
    })
}

const fn root_provision_call_from_intent(
    intent: &FleetComponentProvisioningRootProvisionIntentRecord,
) -> FleetComponentProvisioningRootProvisionCallView {
    FleetComponentProvisioningRootProvisionCallView {
        fleet_subnet_root: intent.fleet_subnet_root,
        request: intent.request,
    }
}

fn advance_scale_out_directory_confirmation(
    current: &FleetCoordinatorRegistryRecord,
    record: &FleetComponentProvisioningRecord,
    request: &FleetComponentProvisioningAdvanceRequest,
    progress: FleetComponentDirectoryConfirmationProgress,
    started_at_ns: u64,
) -> Result<FleetComponentDirectoryConfirmationDisposition, InternalError> {
    match classify_directory_confirmation_advance(request, &progress)? {
        DirectoryConfirmationAdvance::Current => {
            return component_provisioning_status_response(record)
                .map(Box::new)
                .map(FleetComponentDirectoryConfirmationDisposition::Current);
        }
        DirectoryConfirmationAdvance::Reconcile => {
            let intent = progress.in_flight.as_ref().ok_or_else(|| {
                receipt_invariant("scale-out Directory confirmation intent disappeared")
            })?;
            return Ok(FleetComponentDirectoryConfirmationDisposition::Reconcile(
                directory_confirmation_call_from_intent(intent),
            ));
        }
        DirectoryConfirmationAdvance::Begin => {}
    }
    if started_at_ns == 0 || started_at_ns < progress.service_topology_published_at_ns {
        return Err(InternalError::invalid_input());
    }
    let root_index = progress.confirmed_root_count;
    let root = confirmation_root(record, root_index)?;
    let (call, intent) = match progress.current.as_ref() {
        None => {
            scale_out_synchronization_call(record, &progress, root_index, root, 0, started_at_ns)
        }
        Some(current_confirmation) => {
            let (synchronization, publication) =
                scale_out_confirmation_progress(current_confirmation)?;
            if synchronization.complete {
                let previous = publication.map_or_else(
                    || selected_root_provisioned_response(record, &progress, root),
                    Ok,
                )?;
                scale_out_publication_call(
                    record,
                    &progress,
                    root_index,
                    root,
                    previous.published_component_count,
                    started_at_ns,
                )?
            } else {
                scale_out_synchronization_call(
                    record,
                    &progress,
                    root_index,
                    root,
                    synchronization.synchronized_component_count,
                    started_at_ns,
                )
            }
        }
    };
    let mut next = current.clone();
    component_provisioning_operation_record_mut(&mut next, record.operation_id)?.state =
        FleetComponentProvisioningStateRecord::ConfirmingDirectories {
            planned_at_ns: progress.planned_at_ns,
            acceptances: progress.acceptances,
            roots_accepted_at_ns: progress.roots_accepted_at_ns,
            provisions: progress.provisions,
            components_provisioned_at_ns: progress.components_provisioned_at_ns,
            published_fleet_registry: progress.published_fleet_registry,
            service_topology_published_at_ns: progress.service_topology_published_at_ns,
            confirmations: progress.confirmations,
            current: progress.current.map(Box::new),
            in_flight: Some(Box::new(intent)),
        };
    let next = FleetCoordinatorOps::validate_current(next)?;
    FleetCoordinatorOps::commit_transition(current, next)?;
    Ok(FleetComponentDirectoryConfirmationDisposition::Invoke(call))
}

fn scale_out_synchronization_call(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentDirectoryConfirmationProgress,
    root_index: u32,
    root: Principal,
    expected_synchronized_component_count: u32,
    started_at_ns: u64,
) -> (
    FleetComponentDirectoryConfirmationCallView,
    FleetComponentDirectoryConfirmationIntentRecord,
) {
    let request = RootComponentDirectorySynchronizationRequest {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        source_fleet_registry: record.plan.fleet_registry.clone(),
        published_fleet_registry: progress.published_fleet_registry.clone(),
        expected_synchronized_component_count,
    };
    (
        FleetComponentDirectoryConfirmationCallView::ScaleOutSynchronization {
            fleet_subnet_root: root,
            request: request.clone(),
        },
        FleetComponentDirectoryConfirmationIntentRecord::ScaleOutSynchronization {
            root_index,
            fleet_subnet_root: root,
            request,
            started_at_ns,
        },
    )
}

fn scale_out_publication_call(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentDirectoryConfirmationProgress,
    root_index: u32,
    root: Principal,
    expected_published_component_count: u32,
    started_at_ns: u64,
) -> Result<
    (
        FleetComponentDirectoryConfirmationCallView,
        FleetComponentDirectoryConfirmationIntentRecord,
    ),
    InternalError,
> {
    selected_root_batch(record, root)?;
    let request = RootComponentPublicationRequest {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        published_fleet_registry: progress.published_fleet_registry.clone(),
        expected_published_component_count,
    };
    Ok((
        FleetComponentDirectoryConfirmationCallView::ScaleOutPublication {
            fleet_subnet_root: root,
            request: request.clone(),
        },
        FleetComponentDirectoryConfirmationIntentRecord::ScaleOutPublication {
            root_index,
            fleet_subnet_root: root,
            request,
            started_at_ns,
        },
    ))
}

fn confirmation_root(
    record: &FleetComponentProvisioningRecord,
    root_index: u32,
) -> Result<Principal, InternalError> {
    let index = usize::try_from(root_index)
        .map_err(|_| receipt_invariant("Directory confirmation root index exceeds usize"))?;
    let root = *record
        .plan
        .directory_confirmation_roots
        .get(index)
        .ok_or_else(|| receipt_invariant("Directory confirmation root index is out of bounds"))?;
    if matches!(
        record.plan.operation,
        FleetComponentProvisioningOperation::ScaleOut { .. }
    ) {
        return Ok(root);
    }
    let batch =
        record.plan.batches.get(index).ok_or_else(|| {
            receipt_invariant("Directory confirmation root has no selected batch")
        })?;
    if batch.root.fleet_subnet_root != root {
        return Err(receipt_invariant(
            "fresh Directory confirmation roots differ from selected batch order",
        ));
    }
    Ok(root)
}

fn directory_confirmation_call_from_intent(
    intent: &FleetComponentDirectoryConfirmationIntentRecord,
) -> FleetComponentDirectoryConfirmationCallView {
    match intent {
        FleetComponentDirectoryConfirmationIntentRecord::FreshPublication {
            fleet_subnet_root,
            request,
            ..
        } => FleetComponentDirectoryConfirmationCallView::FreshPublication {
            fleet_subnet_root: *fleet_subnet_root,
            request: request.clone(),
        },
        FleetComponentDirectoryConfirmationIntentRecord::ScaleOutSynchronization {
            fleet_subnet_root,
            request,
            ..
        } => FleetComponentDirectoryConfirmationCallView::ScaleOutSynchronization {
            fleet_subnet_root: *fleet_subnet_root,
            request: request.clone(),
        },
        FleetComponentDirectoryConfirmationIntentRecord::ScaleOutPublication {
            fleet_subnet_root,
            request,
            ..
        } => FleetComponentDirectoryConfirmationCallView::ScaleOutPublication {
            fleet_subnet_root: *fleet_subnet_root,
            request: request.clone(),
        },
    }
}

const fn confirmation_intent_root(
    intent: &FleetComponentDirectoryConfirmationIntentRecord,
) -> Principal {
    match intent {
        FleetComponentDirectoryConfirmationIntentRecord::FreshPublication {
            fleet_subnet_root,
            ..
        }
        | FleetComponentDirectoryConfirmationIntentRecord::ScaleOutSynchronization {
            fleet_subnet_root,
            ..
        }
        | FleetComponentDirectoryConfirmationIntentRecord::ScaleOutPublication {
            fleet_subnet_root,
            ..
        } => *fleet_subnet_root,
    }
}

const fn confirmation_call_publication_request(
    call: &FleetComponentDirectoryConfirmationCallView,
) -> Result<&RootComponentPublicationRequest, InternalError> {
    match call {
        FleetComponentDirectoryConfirmationCallView::FreshPublication { request, .. }
        | FleetComponentDirectoryConfirmationCallView::ScaleOutPublication { request, .. } => {
            Ok(request)
        }
        FleetComponentDirectoryConfirmationCallView::ScaleOutSynchronization { .. } => Err(
            receipt_invariant("Directory publication call contains synchronization authority"),
        ),
    }
}

const fn fresh_confirmation_intent(
    intent: &FleetComponentDirectoryConfirmationIntentRecord,
) -> Result<(u32, Principal, &RootComponentPublicationRequest, u64), InternalError> {
    let FleetComponentDirectoryConfirmationIntentRecord::FreshPublication {
        root_index,
        fleet_subnet_root,
        request,
        started_at_ns,
    } = intent
    else {
        return Err(receipt_invariant(
            "fresh Directory confirmation contains scale-out intent",
        ));
    };
    Ok((*root_index, *fleet_subnet_root, request, *started_at_ns))
}

const fn scale_out_synchronization_intent(
    intent: &FleetComponentDirectoryConfirmationIntentRecord,
) -> Result<
    (
        u32,
        Principal,
        &RootComponentDirectorySynchronizationRequest,
        u64,
    ),
    InternalError,
> {
    let FleetComponentDirectoryConfirmationIntentRecord::ScaleOutSynchronization {
        root_index,
        fleet_subnet_root,
        request,
        started_at_ns,
    } = intent
    else {
        return Err(receipt_invariant(
            "scale-out Directory synchronization contains different intent",
        ));
    };
    Ok((*root_index, *fleet_subnet_root, request, *started_at_ns))
}

const fn scale_out_publication_intent(
    intent: &FleetComponentDirectoryConfirmationIntentRecord,
) -> Result<(u32, Principal, &RootComponentPublicationRequest, u64), InternalError> {
    let FleetComponentDirectoryConfirmationIntentRecord::ScaleOutPublication {
        root_index,
        fleet_subnet_root,
        request,
        started_at_ns,
    } = intent
    else {
        return Err(receipt_invariant(
            "scale-out Directory publication contains different intent",
        ));
    };
    Ok((*root_index, *fleet_subnet_root, request, *started_at_ns))
}

fn confirmation_publication_response(
    record: &FleetComponentDirectoryConfirmationRecord,
) -> Option<&RootComponentProvisioningStatusResponse> {
    match record {
        FleetComponentDirectoryConfirmationRecord::FreshPublication { response, .. } => {
            Some(response.as_ref())
        }
        FleetComponentDirectoryConfirmationRecord::ScaleOut { publication, .. } => {
            publication.as_deref()
        }
    }
}

fn fresh_confirmation_response(
    record: &FleetComponentDirectoryConfirmationRecord,
) -> Result<&RootComponentProvisioningStatusResponse, InternalError> {
    let FleetComponentDirectoryConfirmationRecord::FreshPublication { response, .. } = record
    else {
        return Err(receipt_invariant(
            "fresh Directory confirmation contains scale-out evidence",
        ));
    };
    Ok(response.as_ref())
}

const fn confirmation_started_at_ns(record: &FleetComponentDirectoryConfirmationRecord) -> u64 {
    match record {
        FleetComponentDirectoryConfirmationRecord::FreshPublication { started_at_ns, .. }
        | FleetComponentDirectoryConfirmationRecord::ScaleOut { started_at_ns, .. } => {
            *started_at_ns
        }
    }
}

const fn confirmation_recorded_at_ns(record: &FleetComponentDirectoryConfirmationRecord) -> u64 {
    match record {
        FleetComponentDirectoryConfirmationRecord::FreshPublication { recorded_at_ns, .. }
        | FleetComponentDirectoryConfirmationRecord::ScaleOut { recorded_at_ns, .. } => {
            *recorded_at_ns
        }
    }
}

fn scale_out_confirmation_progress(
    record: &FleetComponentDirectoryConfirmationRecord,
) -> Result<
    (
        &RootComponentDirectorySynchronizationResponse,
        Option<&RootComponentProvisioningStatusResponse>,
    ),
    InternalError,
> {
    let FleetComponentDirectoryConfirmationRecord::ScaleOut {
        synchronization,
        publication,
        ..
    } = record
    else {
        return Err(receipt_invariant(
            "scale-out Directory confirmation contains fresh evidence",
        ));
    };
    Ok((synchronization.as_ref(), publication.as_deref()))
}

const fn require_scale_out_operation(
    record: &FleetComponentProvisioningRecord,
) -> Result<(), InternalError> {
    if matches!(
        record.plan.operation,
        FleetComponentProvisioningOperation::ScaleOut { .. }
    ) {
        return Ok(());
    }
    Err(InternalError::conflict())
}

fn selected_root_batch(
    record: &FleetComponentProvisioningRecord,
    root: Principal,
) -> Result<&FleetSubnetRootProvisioningBatch, InternalError> {
    record
        .plan
        .batches
        .iter()
        .find(|batch| batch.root.fleet_subnet_root == root)
        .ok_or_else(|| receipt_invariant("Directory publication root has no selected batch"))
}

fn selected_root_provisioned_response<'a>(
    record: &FleetComponentProvisioningRecord,
    progress: &'a FleetComponentDirectoryConfirmationProgress,
    root: Principal,
) -> Result<&'a RootComponentProvisioningStatusResponse, InternalError> {
    let index = record
        .plan
        .batches
        .iter()
        .position(|batch| batch.root.fleet_subnet_root == root)
        .ok_or_else(|| receipt_invariant("Directory publication root has no selected batch"))?;
    let response = progress
        .provisions
        .get(index)
        .map(|record| &record.response)
        .ok_or_else(|| receipt_invariant("selected Directory root lacks provisioning evidence"))?;
    if response.fleet_subnet_root != root {
        return Err(receipt_invariant(
            "selected Directory root provisioning evidence changed root",
        ));
    }
    Ok(response)
}

fn scale_out_confirmation_is_terminal(
    record: &FleetComponentProvisioningRecord,
    root: Principal,
    confirmation: &FleetComponentDirectoryConfirmationRecord,
) -> Result<bool, InternalError> {
    let (synchronization, publication) = scale_out_confirmation_progress(confirmation)?;
    if !synchronization.complete {
        return Ok(false);
    }
    let selected = record
        .plan
        .batches
        .iter()
        .any(|batch| batch.root.fleet_subnet_root == root);
    Ok(if selected {
        publication
            .is_some_and(|response| response.phase == RootComponentProvisioningPhase::Published)
    } else {
        publication.is_none()
    })
}

struct ScaleOutSynchronizationValidationContext<'a> {
    coordinator: &'a FleetCoordinatorRegistryRecord,
    operation: &'a FleetComponentProvisioningRecord,
    progress: &'a FleetComponentDirectoryConfirmationProgress,
    root_index: u32,
    root: Principal,
    request: &'a RootComponentDirectorySynchronizationRequest,
    started_at_ns: u64,
    recorded_at_ns: u64,
}

fn validate_scale_out_synchronization_response(
    context: &ScaleOutSynchronizationValidationContext<'_>,
    response: &RootComponentDirectorySynchronizationResponse,
) -> Result<(), InternalError> {
    if context.root_index != context.progress.confirmed_root_count
        || confirmation_root(context.operation, context.root_index)? != context.root
    {
        return Err(receipt_invariant(
            "scale-out Directory synchronization cursor changed canonical root",
        ));
    }
    let previous = context
        .progress
        .current
        .as_ref()
        .map(scale_out_confirmation_progress)
        .transpose()?
        .map_or(0, |(response, _)| response.synchronized_component_count);
    let count_advances = response.synchronized_component_count == previous
        || previous.checked_add(1) == Some(response.synchronized_component_count);
    let authority_is_exact = [
        context.request.operation_id == context.operation.operation_id,
        context.request.plan_hash == context.operation.plan_hash,
        context.request.source_fleet_registry == context.operation.plan.fleet_registry,
        context.request.published_fleet_registry == context.progress.published_fleet_registry,
        context.request.expected_synchronized_component_count == previous,
        response.operation_id == context.operation.operation_id,
        response.plan_hash == context.operation.plan_hash,
        response.source_fleet_registry == context.operation.plan.fleet_registry,
        response.published_fleet_registry == context.progress.published_fleet_registry,
        response.fleet_subnet_root == context.root,
        response.synchronized_component_count <= response.affected_component_count,
        count_advances,
        context.recorded_at_ns >= context.started_at_ns,
    ]
    .into_iter()
    .all(|matches| matches);
    if !authority_is_exact {
        return Err(InternalError::conflict());
    }
    if let Some(current) = &context.progress.current {
        let (previous_response, publication) = scale_out_confirmation_progress(current)?;
        let retained_authority_changed = [
            previous_response.affected_component_count != response.affected_component_count,
            previous_response.fleet_directory_content_hash != response.fleet_directory_content_hash,
            publication.is_some(),
        ]
        .into_iter()
        .any(|changed| changed);
        if retained_authority_changed {
            return Err(InternalError::conflict());
        }
    }
    let expected_directory_hash = expected_fleet_directory_content_hash(
        context.coordinator,
        &context.progress.published_fleet_registry,
        context.root,
    )?;
    if response.fleet_directory_content_hash != expected_directory_hash {
        return Err(InternalError::conflict());
    }
    let terminal_evidence_is_exact = if response.complete {
        [
            response.synchronized_component_count == response.affected_component_count,
            response
                .synchronized_at_ns
                .is_some_and(|time| time >= context.started_at_ns),
            response.receipt_content_hash
                == RootComponentProvisioningReceiptOps::directory_synchronization_content_hash(
                    response,
                )?,
        ]
        .into_iter()
        .all(|matches| matches)
    } else {
        [
            response.synchronized_component_count < response.affected_component_count,
            response.synchronized_at_ns.is_none(),
            response.receipt_content_hash == [0; 32],
        ]
        .into_iter()
        .all(|matches| matches)
    };
    if !terminal_evidence_is_exact {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn commit_directory_confirmation_progress(
    current: &FleetCoordinatorRegistryRecord,
    request: &FleetComponentProvisioningAdvanceRequest,
    progress: FleetComponentDirectoryConfirmationProgress,
    recorded_at_ns: u64,
) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
    let confirmed_root_count = u32::try_from(progress.confirmations.len())
        .map_err(|_| receipt_invariant("Directory confirmation count does not fit u32"))?;
    let mut next = current.clone();
    component_provisioning_operation_record_mut(&mut next, request.operation_id)?.state =
        if confirmed_root_count == progress.confirmation_root_count {
            FleetComponentProvisioningStateRecord::DirectoriesConfirmed {
                planned_at_ns: progress.planned_at_ns,
                acceptances: progress.acceptances,
                roots_accepted_at_ns: progress.roots_accepted_at_ns,
                provisions: progress.provisions,
                components_provisioned_at_ns: progress.components_provisioned_at_ns,
                published_fleet_registry: progress.published_fleet_registry,
                service_topology_published_at_ns: progress.service_topology_published_at_ns,
                confirmations: progress.confirmations,
                directories_confirmed_at_ns: recorded_at_ns,
            }
        } else {
            FleetComponentProvisioningStateRecord::ConfirmingDirectories {
                planned_at_ns: progress.planned_at_ns,
                acceptances: progress.acceptances,
                roots_accepted_at_ns: progress.roots_accepted_at_ns,
                provisions: progress.provisions,
                components_provisioned_at_ns: progress.components_provisioned_at_ns,
                published_fleet_registry: progress.published_fleet_registry,
                service_topology_published_at_ns: progress.service_topology_published_at_ns,
                confirmations: progress.confirmations,
                current: progress.current.map(Box::new),
                in_flight: None,
            }
        };
    let next = FleetCoordinatorOps::validate_current(next)?;
    let result = component_provisioning_status_response(component_provisioning_operation_record(
        &next,
        request.operation_id,
    )?)?;
    FleetCoordinatorOps::commit_transition(current, next)?;
    Ok(result)
}

fn commit_runtime_activation_response(
    current: &FleetCoordinatorRegistryRecord,
    request: &FleetComponentProvisioningAdvanceRequest,
    mut progress: FleetComponentRuntimeActivationProgress,
    intent: FleetComponentRuntimeActivationIntentRecord,
    response: &RootComponentProvisioningStatusResponse,
    recorded_at_ns: u64,
) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
    let observed = FleetComponentRuntimeActivationRecord {
        started_at_ns: intent.started_at_ns,
        progress: root_activation_progress(response),
        activation: response.activation,
        activation_started_at_ns: response.activation_started_at_ns,
        runtimes_activated_at_ns: response.runtimes_activated_at_ns,
        receipt_content_hash: response.receipt_content_hash,
        recorded_at_ns,
    };
    if response.phase == RootComponentProvisioningPhase::RuntimesActive {
        progress.activations.push(observed);
        progress.current = None;
    } else {
        progress.current = Some(observed);
    }
    let activated_root_count = u32::try_from(progress.activations.len())
        .map_err(|_| receipt_invariant("runtime-activated root count does not fit u32"))?;
    let runtimes_are_terminal = activated_root_count == progress.activation_root_count;
    let mut next = current.clone();
    component_provisioning_operation_record_mut(&mut next, request.operation_id)?.state =
        runtime_activation_state(progress, runtimes_are_terminal, recorded_at_ns);
    if runtimes_are_terminal {
        next.component_group_deployments =
            terminal_component_deployments(&next, request.operation_id)?;
    }
    let next = FleetCoordinatorOps::validate_current(next)?;
    let result = component_provisioning_status_response(component_provisioning_operation_record(
        &next,
        request.operation_id,
    )?)?;
    FleetCoordinatorOps::commit_transition(current, next)?;
    Ok(result)
}

fn runtime_activation_state(
    progress: FleetComponentRuntimeActivationProgress,
    terminal: bool,
    recorded_at_ns: u64,
) -> FleetComponentProvisioningStateRecord {
    if terminal {
        FleetComponentProvisioningStateRecord::RuntimesActivated {
            planned_at_ns: progress.planned_at_ns,
            acceptances: progress.acceptances,
            roots_accepted_at_ns: progress.roots_accepted_at_ns,
            provisions: progress.provisions,
            components_provisioned_at_ns: progress.components_provisioned_at_ns,
            published_fleet_registry: progress.published_fleet_registry,
            service_topology_published_at_ns: progress.service_topology_published_at_ns,
            confirmations: progress.confirmations,
            directories_confirmed_at_ns: progress.directories_confirmed_at_ns,
            activations: progress.activations,
            runtimes_activated_at_ns: recorded_at_ns,
        }
    } else {
        FleetComponentProvisioningStateRecord::ActivatingRuntimes {
            planned_at_ns: progress.planned_at_ns,
            acceptances: progress.acceptances,
            roots_accepted_at_ns: progress.roots_accepted_at_ns,
            provisions: progress.provisions,
            components_provisioned_at_ns: progress.components_provisioned_at_ns,
            published_fleet_registry: progress.published_fleet_registry,
            service_topology_published_at_ns: progress.service_topology_published_at_ns,
            confirmations: progress.confirmations,
            directories_confirmed_at_ns: progress.directories_confirmed_at_ns,
            activations: progress.activations,
            current: progress.current.map(Box::new),
            in_flight: None,
        }
    }
}

fn terminal_component_deployments(
    current: &FleetCoordinatorRegistryRecord,
    operation_id: [u8; 32],
) -> Result<Vec<FleetComponentGroupDeploymentRecord>, InternalError> {
    let operation = component_provisioning_operation_record(current, operation_id)?;
    match &operation.plan.operation {
        FleetComponentProvisioningOperation::FreshInstall => deployment_ledger::compile_initial(
            &current.component_deployment_configuration,
            operation,
        ),
        FleetComponentProvisioningOperation::ScaleOut { .. } => {
            deployment_ledger::commit_scale_out(&current.component_group_deployments, operation)
        }
    }
}

const fn runtime_activation_call_from_intent(
    intent: &FleetComponentRuntimeActivationIntentRecord,
) -> FleetComponentRuntimeActivationCallView {
    FleetComponentRuntimeActivationCallView {
        fleet_subnet_root: intent.fleet_subnet_root,
        request: intent.request,
    }
}

fn root_provisioned_response(
    progress: &FleetComponentDirectoryConfirmationProgress,
    root_index: u32,
) -> Result<&RootComponentProvisioningStatusResponse, InternalError> {
    let index = usize::try_from(root_index)
        .map_err(|_| receipt_invariant("Directory confirmation root index exceeds usize"))?;
    progress
        .provisions
        .get(index)
        .map(|record| &record.response)
        .ok_or_else(|| receipt_invariant("Directory confirmation lacks root provisioning"))
}

fn root_publication_response<'a>(
    operation: &FleetComponentProvisioningRecord,
    progress: &'a FleetComponentRuntimeActivationProgress,
    root_index: u32,
) -> Result<&'a RootComponentProvisioningStatusResponse, InternalError> {
    let root = activation_root(operation, root_index)?;
    progress
        .confirmations
        .iter()
        .find_map(|confirmation| {
            confirmation_publication_response(confirmation)
                .filter(|response| response.fleet_subnet_root == root)
        })
        .ok_or_else(|| receipt_invariant("runtime activation lacks root publication evidence"))
}

fn activation_root(
    record: &FleetComponentProvisioningRecord,
    root_index: u32,
) -> Result<Principal, InternalError> {
    Ok(root_batch(record, root_index)?.root.fleet_subnet_root)
}

const fn root_activation_progress(
    response: &RootComponentProvisioningStatusResponse,
) -> FleetComponentActivationRootProgress {
    FleetComponentActivationRootProgress {
        fleet_subnet_root: response.fleet_subnet_root,
        component_count: response.component_count,
        activated_component_count: response.activated_component_count,
        root_runtime_active: response.root_runtime_active,
    }
}

fn validate_runtime_activation_response(
    record: &FleetComponentProvisioningRecord,
    root_index: u32,
    publication: &RootComponentProvisioningStatusResponse,
    previous: FleetComponentActivationRootProgress,
    previous_activation_started_at_ns: Option<u64>,
    response: &RootComponentProvisioningStatusResponse,
    recorded_at_ns: u64,
) -> Result<(), InternalError> {
    validate_runtime_activation_authority(publication, response)?;
    let actual = root_activation_progress(response);
    if !activation_progress_advances(previous, actual) {
        return Err(InternalError::conflict());
    }
    let activation_started_at_ns = response
        .activation_started_at_ns
        .ok_or_else(InternalError::conflict)?;
    if previous_activation_started_at_ns
        .is_some_and(|expected| expected != activation_started_at_ns)
    {
        return Err(InternalError::conflict());
    }
    let published_at_ns = response.published_at_ns.ok_or_else(|| {
        receipt_invariant("runtime activation publication lacks its completion time")
    })?;
    if activation_started_at_ns < published_at_ns || recorded_at_ns < activation_started_at_ns {
        return Err(InternalError::conflict());
    }
    match response.phase {
        RootComponentProvisioningPhase::Published => {
            let progress_is_exact = [
                !response.root_runtime_active,
                response.activated_component_count <= response.component_count,
                response.activation.is_none(),
                response.runtimes_activated_at_ns.is_none(),
                response.receipt_content_hash == publication.receipt_content_hash,
            ]
            .into_iter()
            .all(|matches| matches);
            if !progress_is_exact {
                return Err(InternalError::conflict());
            }
        }
        RootComponentProvisioningPhase::RuntimesActive => {
            validate_terminal_runtime_activation(
                record,
                root_index,
                publication,
                response,
                activation_started_at_ns,
                recorded_at_ns,
            )?;
        }
        _ => {
            return Err(InternalError::conflict());
        }
    }
    Ok(())
}

fn validate_runtime_activation_authority(
    publication: &RootComponentProvisioningStatusResponse,
    response: &RootComponentProvisioningStatusResponse,
) -> Result<(), InternalError> {
    let published = publication.phase == RootComponentProvisioningPhase::Published;
    let authority_is_exact = RootRuntimeActivationAuthority::from_response(response)
        == RootRuntimeActivationAuthority::from_response(publication);
    if !published || !authority_is_exact {
        return Err(InternalError::conflict());
    }
    Ok(())
}

#[derive(Eq, PartialEq)]
struct RootRuntimeActivationAuthority<'a> {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    fleet_registry: &'a FleetRegistryVersion,
    configuration_digest: ComponentDeploymentConfigurationDigest,
    fleet_subnet_root: Principal,
    counts: RootRuntimeActivationCounts,
    result: &'a Option<canic_core::dto::component_provisioning::RootComponentProvisioningResult>,
    publication:
        &'a Option<canic_core::dto::component_provisioning::RootComponentPublicationEvidence>,
    accepted_at_ns: u64,
    provisioned_at_ns: Option<u64>,
    published_at_ns: Option<u64>,
}

impl<'a> RootRuntimeActivationAuthority<'a> {
    const fn from_response(response: &'a RootComponentProvisioningStatusResponse) -> Self {
        Self {
            operation_id: response.operation_id,
            plan_hash: response.plan_hash,
            fleet_registry: &response.fleet_registry,
            configuration_digest: response.configuration_digest,
            fleet_subnet_root: response.fleet_subnet_root,
            counts: RootRuntimeActivationCounts::from_response(response),
            result: &response.result,
            publication: &response.publication,
            accepted_at_ns: response.accepted_at_ns,
            provisioned_at_ns: response.provisioned_at_ns,
            published_at_ns: response.published_at_ns,
        }
    }
}

#[derive(Eq, PartialEq)]
struct RootRuntimeActivationCounts {
    placements: u32,
    components: u32,
    reserved: u32,
    claimed: u32,
    installed: u32,
    registry_committed: u32,
    published: u32,
}

impl RootRuntimeActivationCounts {
    const fn from_response(response: &RootComponentProvisioningStatusResponse) -> Self {
        Self {
            placements: response.placement_count,
            components: response.component_count,
            reserved: response.reserved_component_count,
            claimed: response.claimed_component_count,
            installed: response.installed_component_count,
            registry_committed: response.registry_committed_component_count,
            published: response.published_component_count,
        }
    }
}

fn validate_terminal_runtime_activation(
    record: &FleetComponentProvisioningRecord,
    root_index: u32,
    publication: &RootComponentProvisioningStatusResponse,
    response: &RootComponentProvisioningStatusResponse,
    activation_started_at_ns: u64,
    recorded_at_ns: u64,
) -> Result<(), InternalError> {
    let activation = response.activation.ok_or_else(InternalError::conflict)?;
    let runtimes_activated_at_ns = response
        .runtimes_activated_at_ns
        .ok_or_else(InternalError::conflict)?;
    let progress_is_terminal = response.root_runtime_active
        && response.activated_component_count == response.component_count;
    let identity_is_exact = activation.component_count == response.component_count
        && activation.fleet_activation_operation_id != [0; 32]
        && activation.initial_inventory_hash != [0; 32];
    let timing_is_exact = terminal_root_activation_timing_is_valid(
        &record.plan.operation,
        activation.root_activated_at_ns,
        response.accepted_at_ns,
        activation_started_at_ns,
        runtimes_activated_at_ns,
    );
    let observation_is_exact = recorded_at_ns >= runtimes_activated_at_ns;
    let evidence_is_exact =
        progress_is_terminal && identity_is_exact && timing_is_exact && observation_is_exact;
    if !evidence_is_exact {
        return Err(InternalError::conflict());
    }
    let batch = root_batch(record, root_index)?;
    let expected = RootComponentProvisioningReceiptOps::runtimes_active_content_hash(
        RootComponentProvisioningRuntimesActiveReceiptAuthority {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            configuration_digest: record.plan.configuration_digest,
            root: &batch.root,
            published_receipt_content_hash: publication.receipt_content_hash,
            activation,
            activation_started_at_ns,
            runtimes_activated_at_ns,
        },
    )?;
    if response.receipt_content_hash != expected {
        return Err(InternalError::conflict());
    }
    Ok(())
}

const fn terminal_root_activation_timing_is_valid(
    operation: &FleetComponentProvisioningOperation,
    root_activated_at_ns: u64,
    accepted_at_ns: u64,
    activation_started_at_ns: u64,
    runtimes_activated_at_ns: u64,
) -> bool {
    if runtimes_activated_at_ns < activation_started_at_ns {
        return false;
    }
    match operation {
        FleetComponentProvisioningOperation::FreshInstall => {
            root_activated_at_ns == runtimes_activated_at_ns
        }
        FleetComponentProvisioningOperation::ScaleOut { .. } => {
            root_activated_at_ns > 0 && root_activated_at_ns <= accepted_at_ns
        }
    }
}

fn expected_fleet_directory_content_hash(
    current: &FleetCoordinatorRegistryRecord,
    published_registry: &FleetRegistryVersion,
    root: Principal,
) -> Result<[u8; 32], InternalError> {
    let registry = registry_snapshot_at_version(current, published_registry)?;
    let directory = FleetRegistryOps::directory_for_root(
        &registry.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &registry,
        root,
    )?;
    RootComponentProvisioningReceiptOps::fleet_directory_content_hash(&directory)
}

struct RootDirectoryConfirmationValidationContext<'a> {
    operation: &'a FleetComponentProvisioningRecord,
    published_registry: &'a FleetRegistryVersion,
    root: Principal,
    fleet_directory_content_hash: [u8; 32],
}

impl<'a> RootDirectoryConfirmationValidationContext<'a> {
    const fn new(
        operation: &'a FleetComponentProvisioningRecord,
        published_registry: &'a FleetRegistryVersion,
        root: Principal,
        fleet_directory_content_hash: [u8; 32],
    ) -> Self {
        Self {
            operation,
            published_registry,
            root,
            fleet_directory_content_hash,
        }
    }
}

fn validate_directory_confirmation_response(
    context: RootDirectoryConfirmationValidationContext<'_>,
    previous: &RootComponentProvisioningStatusResponse,
    response: &RootComponentProvisioningStatusResponse,
    recorded_at_ns: u64,
    require_bounded_advance: bool,
) -> Result<(), InternalError> {
    let batch = context
        .operation
        .plan
        .batches
        .iter()
        .find(|batch| batch.root.fleet_subnet_root == context.root)
        .ok_or_else(|| receipt_invariant("Directory confirmation root has no planned batch"))?;
    let expected_authority =
        RootDirectoryConfirmationAuthority::expected(context.operation, context.root, previous);
    if RootDirectoryConfirmationAuthority::observed(response) != expected_authority {
        return Err(InternalError::conflict());
    }
    let count_advances = response.published_component_count == previous.published_component_count
        || previous.published_component_count.checked_add(1)
            == Some(response.published_component_count);
    if (require_bounded_advance && !count_advances)
        || response.published_component_count > response.component_count
    {
        return Err(InternalError::conflict());
    }
    let publication = response
        .publication
        .as_ref()
        .ok_or_else(InternalError::conflict)?;
    if &publication.fleet_registry != context.published_registry
        || publication.fleet_directory_content_hash != context.fleet_directory_content_hash
    {
        return Err(InternalError::conflict());
    }
    validate_root_publication_evidence(context.operation, batch, response, publication)?;
    match response.phase {
        RootComponentProvisioningPhase::Provisioned => {
            if response.published_at_ns.is_some()
                || response.receipt_content_hash != previous.receipt_content_hash
            {
                return Err(InternalError::conflict());
            }
        }
        RootComponentProvisioningPhase::Published => {
            let result = response.result.as_ref().ok_or_else(|| {
                receipt_invariant("Published Directory confirmation lacks provisioned result")
            })?;
            let provisioned_at_ns = response.provisioned_at_ns.ok_or_else(|| {
                receipt_invariant("Published Directory confirmation lacks provisioning time")
            })?;
            let published_at_ns = response.published_at_ns.ok_or_else(|| {
                receipt_invariant("Published Directory confirmation lacks publication time")
            })?;
            if response.published_component_count != response.component_count
                || published_at_ns < provisioned_at_ns
                || recorded_at_ns < published_at_ns
            {
                return Err(InternalError::conflict());
            }
            let expected = RootComponentProvisioningReceiptOps::published_content_hash(
                RootComponentProvisioningPublishedReceiptAuthority {
                    operation_id: context.operation.operation_id,
                    plan_hash: context.operation.plan_hash,
                    configuration_digest: context.operation.plan.configuration_digest,
                    root: &batch.root,
                    result,
                    publication,
                    accepted_at_ns: response.accepted_at_ns,
                    provisioned_at_ns,
                    published_at_ns,
                },
            )?;
            if response.receipt_content_hash != expected {
                return Err(InternalError::conflict());
            }
        }
        _ => {
            return Err(InternalError::conflict());
        }
    }
    Ok(())
}

#[derive(Eq, PartialEq)]
struct RootDirectoryConfirmationCounts {
    placements: u32,
    components: u32,
    reserved: u32,
    claimed: u32,
    installed: u32,
    registry_committed: u32,
}

#[derive(Eq, PartialEq)]
struct RootDirectoryConfirmationAuthority<'a> {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    configuration_digest: &'a ComponentDeploymentConfigurationDigest,
    fleet_registry: &'a FleetRegistryVersion,
    fleet_subnet_root: Principal,
    counts: RootDirectoryConfirmationCounts,
    result: &'a Option<canic_core::dto::component_provisioning::RootComponentProvisioningResult>,
    accepted_at_ns: u64,
    provisioned_at_ns: Option<u64>,
}

impl<'a> RootDirectoryConfirmationAuthority<'a> {
    const fn expected(
        record: &'a FleetComponentProvisioningRecord,
        root: Principal,
        previous: &'a RootComponentProvisioningStatusResponse,
    ) -> Self {
        Self {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            configuration_digest: &record.plan.configuration_digest,
            fleet_registry: &record.plan.fleet_registry,
            fleet_subnet_root: root,
            counts: RootDirectoryConfirmationCounts::from_response(previous),
            result: &previous.result,
            accepted_at_ns: previous.accepted_at_ns,
            provisioned_at_ns: previous.provisioned_at_ns,
        }
    }

    const fn observed(response: &'a RootComponentProvisioningStatusResponse) -> Self {
        Self {
            operation_id: response.operation_id,
            plan_hash: response.plan_hash,
            configuration_digest: &response.configuration_digest,
            fleet_registry: &response.fleet_registry,
            fleet_subnet_root: response.fleet_subnet_root,
            counts: RootDirectoryConfirmationCounts::from_response(response),
            result: &response.result,
            accepted_at_ns: response.accepted_at_ns,
            provisioned_at_ns: response.provisioned_at_ns,
        }
    }
}

impl RootDirectoryConfirmationCounts {
    const fn from_response(response: &RootComponentProvisioningStatusResponse) -> Self {
        Self {
            placements: response.placement_count,
            components: response.component_count,
            reserved: response.reserved_component_count,
            claimed: response.claimed_component_count,
            installed: response.installed_component_count,
            registry_committed: response.registry_committed_component_count,
        }
    }
}

fn validate_root_publication_evidence(
    record: &FleetComponentProvisioningRecord,
    batch: &FleetSubnetRootProvisioningBatch,
    response: &RootComponentProvisioningStatusResponse,
    publication: &canic_core::dto::component_provisioning::RootComponentPublicationEvidence,
) -> Result<(), InternalError> {
    let result = response
        .result
        .as_ref()
        .ok_or_else(|| receipt_invariant("Directory confirmation lacks its provisioned result"))?;
    if publication.component_directories.len()
        != usize::try_from(response.published_component_count)
            .map_err(|_| receipt_invariant("published Component count exceeds usize"))?
        || publication.component_group_directories.len() != result.placements.len()
    {
        return Err(InternalError::conflict());
    }
    for (member, evidence) in result
        .placements
        .iter()
        .flat_map(|placement| &placement.members)
        .zip(&publication.component_directories)
    {
        if evidence.component != member.binding.component
            || evidence.content_hash != member.component_registry_content_hash
        {
            return Err(InternalError::conflict());
        }
    }
    for (index, (planned, provisioned)) in
        batch.placements.iter().zip(&result.placements).enumerate()
    {
        let evidence = &publication.component_group_directories[index];
        let directory =
            component_group_directory_from_receipt(record, batch, planned, provisioned)?;
        let expected_hash =
            RootComponentProvisioningReceiptOps::component_group_directory_content_hash(
                &directory,
            )?;
        if evidence.group_placement != provisioned.group_placement
            || evidence.content_hash != expected_hash
        {
            return Err(InternalError::conflict());
        }
    }
    Ok(())
}

fn component_group_directory_from_receipt(
    record: &FleetComponentProvisioningRecord,
    batch: &FleetSubnetRootProvisioningBatch,
    planned: &canic_core::dto::component_provisioning::ComponentGroupPlacementPlan,
    provisioned: &canic_core::dto::component_provisioning::RootProvisionedGroupPlacement,
) -> Result<canic_core::dto::component_provisioning::ComponentGroupDirectory, InternalError> {
    let placement_matches = [
        planned.group_placement == provisioned.group_placement,
        planned.component_group == provisioned.component_group,
        planned.entries.len() == provisioned.members.len(),
    ]
    .into_iter()
    .all(|matches| matches);
    if !placement_matches {
        return Err(receipt_invariant(
            "Component Group Directory plan differs from provisioned placement",
        ));
    }
    let members = planned
        .entries
        .iter()
        .zip(&provisioned.members)
        .map(|(entry, member)| {
            if entry.member_path != member.member_path
                || entry.component_spec != member.component_spec
                || entry.purpose != member.purpose
            {
                return Err(receipt_invariant(
                    "Component Group Directory member differs from planned occurrence",
                ));
            }
            Ok(
                canic_core::dto::component_provisioning::ComponentGroupDirectoryMember {
                    member_path: member.member_path.clone(),
                    component_spec: member.component_spec.clone(),
                    purpose: member.purpose.clone(),
                    labels: entry.labels.clone(),
                    binding: member.binding.clone(),
                },
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(
        canic_core::dto::component_provisioning::ComponentGroupDirectory {
            provenance:
                canic_core::dto::component_provisioning::ComponentGroupDirectoryProvenance {
                    authority: batch.root.authority.clone(),
                    fleet_subnet_root: batch.root.fleet_subnet_root,
                    group_placement: provisioned.group_placement.clone(),
                    component_group: provisioned.component_group.clone(),
                    operation_id: record.operation_id,
                    plan_hash: record.plan_hash,
                    placement_receipt_content_hash:
                        RootComponentProvisioningReceiptOps::group_placement_content_hash(
                            record.operation_id,
                            record.plan_hash,
                            &batch.root,
                            provisioned,
                        )?,
                },
            members,
        },
    )
}

fn root_provision_previous_observed_at(
    progress: &FleetComponentProvisioningRootProvisionProgress,
) -> Result<u64, InternalError> {
    if let Some(current) = &progress.current {
        return Ok(current.recorded_at_ns);
    }
    if let Some(provision) = progress.provisions.last() {
        return Ok(provision.recorded_at_ns);
    }
    progress
        .roots_accepted_at_ns
        .ok_or_else(|| receipt_invariant("root provisioning lacks RootsAccepted time authority"))
}

fn component_provisioning_root_acceptance(
    record: &FleetComponentProvisioningRecord,
    root_index: u32,
) -> Result<FleetComponentProvisioningRootAcceptanceRecord, InternalError> {
    let progress = component_provisioning_root_acceptance_progress(record)?;
    let index = usize::try_from(root_index).map_err(|_| InternalError::resource_exhausted())?;
    progress
        .acceptances
        .get(index)
        .cloned()
        .ok_or_else(|| receipt_invariant("accepted root receipt is absent at its cursor"))
}

fn replay_recorded_root_provision(
    record: &FleetComponentProvisioningRecord,
    request: &FleetComponentProvisioningAdvanceRequest,
    response: &RootComponentProvisioningStatusResponse,
    progress: &FleetComponentProvisioningRootProvisionProgress,
) -> Result<Option<FleetComponentProvisioningStatusResponse>, InternalError> {
    let replayed = if request.expected_provisioned_root_count.checked_add(1)
        == Some(progress.provisioned_root_count)
    {
        let index = usize::try_from(request.expected_provisioned_root_count)
            .map_err(|_| InternalError::resource_exhausted())?;
        progress.provisions.get(index)
    } else if request.expected_provisioned_root_count == progress.provisioned_root_count
        && progress.current.as_ref().is_some_and(|current| {
            let actual = root_provisioning_progress(&current.response);
            request.expected_current_root.is_some_and(|expected| {
                expected.fleet_subnet_root == actual.fleet_subnet_root
                    && expected.component_count == actual.component_count
                    && RootProvisioningCounts::from_progress(expected).advances_one_step_to(
                        RootProvisioningCounts::from_progress(actual),
                        actual.component_count,
                    )
            })
        })
    {
        progress.current.as_ref()
    } else {
        None
    };
    let Some(replayed) = replayed else {
        return Ok(None);
    };
    if &replayed.response != response {
        return Err(InternalError::conflict());
    }
    component_provisioning_status_response(record).map(Some)
}

fn classify_root_acceptance_advance(
    request: &FleetComponentProvisioningAdvanceRequest,
    progress: &FleetComponentProvisioningRootAcceptanceProgress,
) -> Result<RootAcceptanceAdvance, InternalError> {
    if request.expected_accepted_root_count == progress.accepted_root_count {
        if progress.accepted_root_count == progress.root_batch_count
            && progress.roots_accepted_at_ns.is_some()
        {
            return Ok(RootAcceptanceAdvance::Current);
        }
        return Ok(if progress.in_flight.is_some() {
            RootAcceptanceAdvance::Reconcile
        } else {
            RootAcceptanceAdvance::Begin
        });
    }
    if request.expected_accepted_root_count.checked_add(1) == Some(progress.accepted_root_count) {
        return Ok(RootAcceptanceAdvance::Current);
    }
    Err(InternalError::conflict())
}

fn root_acceptance_call(
    record: &FleetComponentProvisioningRecord,
    root_index: u32,
) -> Result<FleetComponentProvisioningRootAcceptanceCallView, InternalError> {
    let batch = root_batch(record, root_index)?;
    Ok(FleetComponentProvisioningRootAcceptanceCallView {
        fleet_subnet_root: batch.root.fleet_subnet_root,
        request: RootComponentProvisioningAcceptanceRequest {
            fleet_registry: record.plan.fleet_registry.clone(),
            configuration_digest: record.plan.configuration_digest,
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            batch: batch.clone(),
        },
    })
}

fn root_batch(
    record: &FleetComponentProvisioningRecord,
    root_index: u32,
) -> Result<&FleetSubnetRootProvisioningBatch, InternalError> {
    let index = usize::try_from(root_index).map_err(|_| InternalError::resource_exhausted())?;
    record
        .plan
        .batches
        .get(index)
        .ok_or_else(InternalError::conflict)
}

fn replay_recorded_root_acceptance(
    record: &FleetComponentProvisioningRecord,
    request: &FleetComponentProvisioningAdvanceRequest,
    response: &RootComponentProvisioningStatusResponse,
    progress: &FleetComponentProvisioningRootAcceptanceProgress,
) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
    if request.expected_accepted_root_count.checked_add(1) != Some(progress.accepted_root_count) {
        return Err(InternalError::conflict());
    }
    let index = usize::try_from(request.expected_accepted_root_count)
        .map_err(|_| InternalError::resource_exhausted())?;
    let recorded = progress.acceptances.get(index).ok_or_else(|| {
        receipt_invariant("recorded root acceptance is absent at its durable cursor")
    })?;
    if &recorded.response != response {
        return Err(InternalError::conflict());
    }
    component_provisioning_status_response(record)
}

fn validate_component_provisioning_root_acceptance_state(
    record: &FleetComponentProvisioningRecord,
) -> Result<(), InternalError> {
    let progress = component_provisioning_root_acceptance_progress(record)?;
    if progress.planned_at_ns == 0 {
        return Err(receipt_invariant(
            "Fleet Component provisioning planned time is zero",
        ));
    }
    if progress.accepted_root_count > progress.root_batch_count {
        return Err(receipt_invariant(
            "Fleet Component accepted root count exceeds its complete plan",
        ));
    }
    let mut previous_recorded_at_ns = progress.planned_at_ns;
    for (index, acceptance) in progress.acceptances.iter().enumerate() {
        let root_index = u32::try_from(index)
            .map_err(|_| receipt_invariant("accepted root index does not fit u32"))?;
        let batch = root_batch(record, root_index)?;
        validate_root_acceptance_response(record, batch, &acceptance.response).map_err(|_| {
            receipt_invariant("stored root acceptance differs from its exact plan batch")
        })?;
        if acceptance.started_at_ns < previous_recorded_at_ns {
            return Err(receipt_invariant(
                "Fleet Component root acceptance time evidence is invalid",
            ));
        }
        validate_root_acceptance_observation(
            acceptance.started_at_ns,
            &acceptance.response,
            acceptance.recorded_at_ns,
        )
        .map_err(|_| {
            receipt_invariant("Fleet Component root acceptance time evidence is invalid")
        })?;
        previous_recorded_at_ns = acceptance.recorded_at_ns;
    }
    validate_root_acceptance_phase(record, &progress, previous_recorded_at_ns)
}

fn validate_root_acceptance_phase(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentProvisioningRootAcceptanceProgress,
    previous_recorded_at_ns: u64,
) -> Result<(), InternalError> {
    match progress.phase {
        FleetComponentProvisioningPhase::Planned => Ok(()),
        FleetComponentProvisioningPhase::AcceptingRoots => {
            if matches!(
                (progress.accepted_root_count, progress.in_flight),
                (0, None)
            ) {
                return Err(receipt_invariant(
                    "Fleet Component root acceptance has neither progress nor pre-call intent",
                ));
            }
            if progress.accepted_root_count >= progress.root_batch_count {
                return Err(receipt_invariant(
                    "Fleet Component root acceptance remained nonterminal after every root",
                ));
            }
            let Some(intent) = progress.in_flight else {
                return Ok(());
            };
            if intent.root_index != progress.accepted_root_count {
                return Err(receipt_invariant(
                    "Fleet Component root acceptance intent differs from its durable cursor",
                ));
            }
            let batch = root_batch(record, intent.root_index)?;
            if intent.fleet_subnet_root != batch.root.fleet_subnet_root {
                return Err(receipt_invariant(
                    "Fleet Component root acceptance intent names a different root",
                ));
            }
            if intent.started_at_ns < previous_recorded_at_ns {
                return Err(receipt_invariant(
                    "Fleet Component root acceptance intent time regressed",
                ));
            }
            Ok(())
        }
        FleetComponentProvisioningPhase::RootsAccepted => {
            if progress.accepted_root_count != progress.root_batch_count {
                return Err(receipt_invariant(
                    "Fleet Component RootsAccepted state lacks complete root evidence",
                ));
            }
            let completed_at_ns = progress
                .roots_accepted_at_ns
                .ok_or_else(|| receipt_invariant("Fleet Component RootsAccepted time is absent"))?;
            if completed_at_ns < previous_recorded_at_ns {
                return Err(receipt_invariant(
                    "Fleet Component RootsAccepted time precedes root evidence",
                ));
            }
            Ok(())
        }
        FleetComponentProvisioningPhase::ProvisioningRoots
        | FleetComponentProvisioningPhase::ComponentsProvisioned
        | FleetComponentProvisioningPhase::ServiceTopologyPublished
        | FleetComponentProvisioningPhase::ConfirmingDirectories
        | FleetComponentProvisioningPhase::DirectoriesConfirmed
        | FleetComponentProvisioningPhase::ActivatingRuntimes
        | FleetComponentProvisioningPhase::RuntimesActivated => {
            if progress.accepted_root_count != progress.root_batch_count {
                return Err(receipt_invariant(
                    "Fleet Component post-acceptance state lacks complete root evidence",
                ));
            }
            let completed_at_ns = progress
                .roots_accepted_at_ns
                .ok_or_else(|| receipt_invariant("Fleet Component RootsAccepted time is absent"))?;
            if completed_at_ns < previous_recorded_at_ns {
                return Err(receipt_invariant(
                    "Fleet Component RootsAccepted time precedes root evidence",
                ));
            }
            Ok(())
        }
    }
}

#[derive(Eq, PartialEq)]
struct RootAcceptanceResponseIdentity<'a> {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    fleet_registry: &'a FleetRegistryVersion,
    configuration_digest: ComponentDeploymentConfigurationDigest,
    fleet_subnet_root: Principal,
}

#[derive(Eq, PartialEq)]
struct RootAcceptanceResponseProgress<'a> {
    phase: RootComponentProvisioningPhase,
    placement_count: u32,
    component_count: u32,
    reserved_component_count: u32,
    claimed_component_count: u32,
    installed_component_count: u32,
    registry_committed_component_count: u32,
    result: Option<&'a canic_core::dto::component_provisioning::RootComponentProvisioningResult>,
    provisioned_at_ns: Option<u64>,
}

struct RootProvisionResponseValidation<'a> {
    configuration: &'a canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
    record: &'a FleetComponentProvisioningRecord,
    root_index: u32,
    acceptance: &'a FleetComponentProvisioningRootAcceptanceRecord,
    previous: &'a RootComponentProvisioningStatusResponse,
    response: &'a RootComponentProvisioningStatusResponse,
    started_at_ns: u64,
    recorded_at_ns: u64,
}

fn validate_root_acceptance_response(
    record: &FleetComponentProvisioningRecord,
    batch: &FleetSubnetRootProvisioningBatch,
    response: &RootComponentProvisioningStatusResponse,
) -> Result<(), InternalError> {
    let expected_identity = RootAcceptanceResponseIdentity {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        fleet_registry: &record.plan.fleet_registry,
        configuration_digest: record.plan.configuration_digest,
        fleet_subnet_root: batch.root.fleet_subnet_root,
    };
    let actual_identity = RootAcceptanceResponseIdentity {
        operation_id: response.operation_id,
        plan_hash: response.plan_hash,
        fleet_registry: &response.fleet_registry,
        configuration_digest: response.configuration_digest,
        fleet_subnet_root: response.fleet_subnet_root,
    };
    if actual_identity != expected_identity {
        return Err(InternalError::conflict());
    }
    let (placement_count, component_count) = root_batch_counts(batch)?;
    let expected_progress = RootAcceptanceResponseProgress {
        phase: RootComponentProvisioningPhase::Accepted,
        placement_count,
        component_count,
        reserved_component_count: 0,
        claimed_component_count: 0,
        installed_component_count: 0,
        registry_committed_component_count: 0,
        result: None,
        provisioned_at_ns: None,
    };
    let actual_progress = RootAcceptanceResponseProgress {
        phase: response.phase,
        placement_count: response.placement_count,
        component_count: response.component_count,
        reserved_component_count: response.reserved_component_count,
        claimed_component_count: response.claimed_component_count,
        installed_component_count: response.installed_component_count,
        registry_committed_component_count: response.registry_committed_component_count,
        result: response.result.as_ref(),
        provisioned_at_ns: response.provisioned_at_ns,
    };
    if actual_progress != expected_progress {
        return Err(InternalError::conflict());
    }
    if response.accepted_at_ns == 0 {
        return Err(InternalError::conflict());
    }
    let receipt_content_hash = RootComponentProvisioningReceiptOps::acceptance_content_hash(
        RootComponentProvisioningAcceptanceReceiptAuthority {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            fleet_registry: &record.plan.fleet_registry,
            configuration_digest: record.plan.configuration_digest,
            batch,
            placement_count,
            component_count,
            accepted_at_ns: response.accepted_at_ns,
        },
    )?;
    if response.receipt_content_hash != receipt_content_hash {
        return Err(InternalError::conflict());
    }
    Ok(())
}

const fn validate_root_acceptance_observation(
    started_at_ns: u64,
    response: &RootComponentProvisioningStatusResponse,
    recorded_at_ns: u64,
) -> Result<(), InternalError> {
    if response.accepted_at_ns < started_at_ns || recorded_at_ns < response.accepted_at_ns {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}

fn validate_root_provision_response(
    validation: RootProvisionResponseValidation<'_>,
) -> Result<(), InternalError> {
    let RootProvisionResponseValidation {
        configuration,
        record,
        root_index,
        acceptance,
        previous,
        response,
        started_at_ns,
        recorded_at_ns,
    } = validation;
    let batch = root_batch(record, root_index)?;
    if previous.phase != RootComponentProvisioningPhase::Accepted {
        return Err(receipt_invariant(
            "root provisioning predecessor is not in the Accepted phase",
        ));
    }
    if response.accepted_at_ns != acceptance.response.accepted_at_ns {
        return Err(InternalError::conflict());
    }
    match response.phase {
        RootComponentProvisioningPhase::Accepted => {
            validate_root_provision_current(record, batch, acceptance, response)?;
            let previous_counts = RootProvisioningCounts::from_response(previous);
            let next_counts = RootProvisioningCounts::from_response(response);
            if !previous_counts.advances_one_step_to(next_counts, response.component_count) {
                return Err(InternalError::conflict());
            }
        }
        RootComponentProvisioningPhase::Provisioned => {
            if !RootProvisioningCounts::from_response(previous)
                .is_terminal(previous.component_count)
            {
                return Err(InternalError::conflict());
            }
            FleetServiceBindingOps::validate_provisioned_root_receipt_compiled(
                configuration,
                &record.plan,
                record.operation_id,
                record.plan_hash,
                usize::try_from(root_index).map_err(|_| InternalError::resource_exhausted())?,
                response,
            )?;
            let provisioned_at_ns = response
                .provisioned_at_ns
                .ok_or_else(InternalError::conflict)?;
            if provisioned_at_ns < started_at_ns || recorded_at_ns < provisioned_at_ns {
                return Err(InternalError::invalid_input());
            }
        }
        RootComponentProvisioningPhase::Published
        | RootComponentProvisioningPhase::RuntimesActive => {
            return Err(InternalError::conflict());
        }
    }
    Ok(())
}

fn validate_root_provision_current(
    record: &FleetComponentProvisioningRecord,
    batch: &FleetSubnetRootProvisioningBatch,
    acceptance: &FleetComponentProvisioningRootAcceptanceRecord,
    response: &RootComponentProvisioningStatusResponse,
) -> Result<(), InternalError> {
    let expected_identity = RootAcceptanceResponseIdentity {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        fleet_registry: &record.plan.fleet_registry,
        configuration_digest: record.plan.configuration_digest,
        fleet_subnet_root: batch.root.fleet_subnet_root,
    };
    let actual_identity = RootAcceptanceResponseIdentity {
        operation_id: response.operation_id,
        plan_hash: response.plan_hash,
        fleet_registry: &response.fleet_registry,
        configuration_digest: response.configuration_digest,
        fleet_subnet_root: response.fleet_subnet_root,
    };
    let (placement_count, component_count) = root_batch_counts(batch)?;
    let progress_is_valid = response.phase == RootComponentProvisioningPhase::Accepted
        && response.placement_count == placement_count
        && response.component_count == component_count
        && response.result.is_none()
        && response.provisioned_at_ns.is_none()
        && RootProvisioningCounts::from_response(response).is_canonical(component_count);
    let acceptance_is_exact = response.accepted_at_ns == acceptance.response.accepted_at_ns
        && response.receipt_content_hash == acceptance.response.receipt_content_hash;
    if actual_identity != expected_identity || !progress_is_valid || !acceptance_is_exact {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn validate_component_provisioning_root_provision_state(
    configuration: &canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
    source_registry: &FleetRegistry,
    record: &FleetComponentProvisioningRecord,
) -> Result<(), InternalError> {
    let acceptance = component_provisioning_root_acceptance_progress(record)?;
    let progress = component_provisioning_root_provision_progress(record)?;
    match acceptance.phase {
        FleetComponentProvisioningPhase::Planned
        | FleetComponentProvisioningPhase::AcceptingRoots => {
            if progress.provisioned_root_count != 0
                || progress.current_response.is_some()
                || progress.in_flight.is_some()
                || progress.roots_accepted_at_ns.is_some()
            {
                return Err(receipt_invariant(
                    "root provisioning evidence exists before complete root acceptance",
                ));
            }
            return Ok(());
        }
        FleetComponentProvisioningPhase::RootsAccepted => {
            if progress.provisioned_root_count != 0
                || progress.current.is_some()
                || progress.in_flight.is_some()
                || progress.current_response.is_none()
            {
                return Err(receipt_invariant(
                    "RootsAccepted state contains invalid root provisioning evidence",
                ));
            }
            return Ok(());
        }
        FleetComponentProvisioningPhase::ProvisioningRoots
        | FleetComponentProvisioningPhase::ComponentsProvisioned
        | FleetComponentProvisioningPhase::ServiceTopologyPublished
        | FleetComponentProvisioningPhase::ConfirmingDirectories
        | FleetComponentProvisioningPhase::DirectoriesConfirmed
        | FleetComponentProvisioningPhase::ActivatingRuntimes
        | FleetComponentProvisioningPhase::RuntimesActivated => {}
    }
    let roots_accepted_at_ns = progress.roots_accepted_at_ns.ok_or_else(|| {
        receipt_invariant("root provisioning state lacks RootsAccepted time authority")
    })?;
    if progress.provisioned_root_count > acceptance.root_batch_count {
        return Err(receipt_invariant(
            "provisioned root count exceeds the complete plan",
        ));
    }
    let previous_observed_at_ns = validate_root_provision_receipts(
        configuration,
        record,
        &progress.provisions,
        roots_accepted_at_ns,
    )?;
    validate_current_root_provision_record(record, &progress, previous_observed_at_ns)?;
    validate_root_provision_intent(record, &progress)?;
    match acceptance.phase {
        FleetComponentProvisioningPhase::ProvisioningRoots => {
            if progress.provisioned_root_count >= acceptance.root_batch_count
                || progress.components_provisioned_at_ns.is_some()
            {
                return Err(receipt_invariant(
                    "root provisioning remained nonterminal after every planned root",
                ));
            }
        }
        FleetComponentProvisioningPhase::ComponentsProvisioned
        | FleetComponentProvisioningPhase::ServiceTopologyPublished
        | FleetComponentProvisioningPhase::ConfirmingDirectories
        | FleetComponentProvisioningPhase::DirectoriesConfirmed
        | FleetComponentProvisioningPhase::ActivatingRuntimes
        | FleetComponentProvisioningPhase::RuntimesActivated => {
            validate_terminal_component_provisioning(
                configuration,
                source_registry,
                record,
                &acceptance,
                &progress,
                previous_observed_at_ns,
            )?;
        }
        _ => unreachable!("pre-provisioning phases returned above"),
    }
    Ok(())
}

fn validate_terminal_component_provisioning(
    configuration: &canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
    source_registry: &FleetRegistry,
    record: &FleetComponentProvisioningRecord,
    acceptance: &FleetComponentProvisioningRootAcceptanceProgress,
    progress: &FleetComponentProvisioningRootProvisionProgress,
    previous_observed_at_ns: u64,
) -> Result<(), InternalError> {
    if progress.provisioned_root_count != acceptance.root_batch_count
        || progress.current_response.is_some()
        || progress.in_flight.is_some()
    {
        return Err(receipt_invariant(
            "ComponentsProvisioned state lacks complete terminal root evidence",
        ));
    }
    let completed_at_ns = progress
        .components_provisioned_at_ns
        .ok_or_else(|| receipt_invariant("ComponentsProvisioned time evidence is absent"))?;
    if completed_at_ns < previous_observed_at_ns {
        return Err(receipt_invariant(
            "ComponentsProvisioned time precedes terminal root evidence",
        ));
    }
    let receipts = progress
        .provisions
        .iter()
        .map(|provision| provision.response.clone())
        .collect::<Vec<_>>();
    compile_component_operation_services(configuration, source_registry, record, &receipts)
        .map_err(|_| {
            receipt_invariant(
                "complete root provisioning receipts do not compile canonical services",
            )
        })?;
    validate_service_publication_progress(acceptance.phase, progress, completed_at_ns)
}

fn validate_service_publication_progress(
    phase: FleetComponentProvisioningPhase,
    progress: &FleetComponentProvisioningRootProvisionProgress,
    components_provisioned_at_ns: u64,
) -> Result<(), InternalError> {
    match phase {
        FleetComponentProvisioningPhase::ComponentsProvisioned => {
            if progress.published_fleet_registry.is_some()
                || progress.service_topology_published_at_ns.is_some()
            {
                return Err(receipt_invariant(
                    "ComponentsProvisioned state contains premature publication evidence",
                ));
            }
        }
        FleetComponentProvisioningPhase::ServiceTopologyPublished
        | FleetComponentProvisioningPhase::ConfirmingDirectories
        | FleetComponentProvisioningPhase::DirectoriesConfirmed
        | FleetComponentProvisioningPhase::ActivatingRuntimes
        | FleetComponentProvisioningPhase::RuntimesActivated => {
            let published_at_ns = progress.service_topology_published_at_ns.ok_or_else(|| {
                receipt_invariant("ServiceTopologyPublished time evidence is absent")
            })?;
            if progress.published_fleet_registry.is_none()
                || published_at_ns < components_provisioned_at_ns
            {
                return Err(receipt_invariant(
                    "ServiceTopologyPublished state contains invalid publication evidence",
                ));
            }
        }
        _ => unreachable!("pre-provisioning phases returned above"),
    }
    Ok(())
}

fn validate_root_provision_receipts(
    configuration: &canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
    record: &FleetComponentProvisioningRecord,
    provisions: &[FleetComponentProvisioningRootProvisionRecord],
    roots_accepted_at_ns: u64,
) -> Result<u64, InternalError> {
    let mut previous_observed_at_ns = roots_accepted_at_ns;
    for (index, provision) in provisions.iter().enumerate() {
        let root_index = u32::try_from(index)
            .map_err(|_| receipt_invariant("provisioned root index does not fit u32"))?;
        let accepted = component_provisioning_root_acceptance(record, root_index)?;
        if provision.started_at_ns < previous_observed_at_ns
            || provision.recorded_at_ns < provision.started_at_ns
            || provision.response.accepted_at_ns != accepted.response.accepted_at_ns
        {
            return Err(receipt_invariant(
                "stored root Provisioned response time evidence is invalid",
            ));
        }
        let provisioned_at_ns = provision.response.provisioned_at_ns.ok_or_else(|| {
            receipt_invariant("stored root Provisioned response has no completion time")
        })?;
        if provisioned_at_ns < provision.started_at_ns
            || provision.recorded_at_ns < provisioned_at_ns
        {
            return Err(receipt_invariant(
                "stored root Provisioned response time evidence is invalid",
            ));
        }
        FleetServiceBindingOps::validate_provisioned_root_receipt_compiled(
            configuration,
            &record.plan,
            record.operation_id,
            record.plan_hash,
            index,
            &provision.response,
        )
        .map_err(|_| {
            receipt_invariant("stored root Provisioned response differs from its plan batch")
        })?;
        previous_observed_at_ns = provision.recorded_at_ns;
    }
    Ok(previous_observed_at_ns)
}

fn validate_current_root_provision_record(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentProvisioningRootProvisionProgress,
    previous_observed_at_ns: u64,
) -> Result<(), InternalError> {
    let Some(current) = &progress.current else {
        return Ok(());
    };
    if current.started_at_ns < previous_observed_at_ns
        || current.recorded_at_ns < current.started_at_ns
    {
        return Err(InternalError::invariant());
    }
    let batch = root_batch(record, progress.provisioned_root_count)?;
    let accepted = component_provisioning_root_acceptance(record, progress.provisioned_root_count)?;
    validate_root_provision_current(record, batch, &accepted, &current.response).map_err(|_| {
        receipt_invariant("current root provisioning response differs from its plan batch")
    })
}

fn validate_root_provision_intent(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentProvisioningRootProvisionProgress,
) -> Result<(), InternalError> {
    let Some(intent) = &progress.in_flight else {
        return Ok(());
    };
    if intent.root_index != progress.provisioned_root_count
        || intent.started_at_ns < root_provision_previous_observed_at(progress)?
    {
        return Err(receipt_invariant(
            "root provisioning pre-call intent differs from its durable cursor",
        ));
    }
    let response = progress
        .current_response
        .as_ref()
        .ok_or_else(|| receipt_invariant("root provisioning intent has no current root cursor"))?;
    let expected = root_provision_call(record, intent.root_index, response)?;
    if intent.fleet_subnet_root != expected.fleet_subnet_root || intent.request != expected.request
    {
        return Err(receipt_invariant(
            "root provisioning pre-call intent differs from its exact root request",
        ));
    }
    Ok(())
}

fn root_batch_counts(
    batch: &FleetSubnetRootProvisioningBatch,
) -> Result<(u32, u32), InternalError> {
    let placement_count = u32::try_from(batch.placements.len())
        .map_err(|_| receipt_invariant("root batch placement count does not fit u32"))?;
    let mut component_count = 0_u32;
    for placement in &batch.placements {
        let members = u32::try_from(placement.entries.len())
            .map_err(|_| receipt_invariant("root batch member count does not fit u32"))?;
        component_count = component_count
            .checked_add(members)
            .ok_or_else(|| receipt_invariant("root batch Component count overflowed"))?;
    }
    Ok((placement_count, component_count))
}

fn component_provisioning_plan_counts(
    plan: &FleetComponentProvisioningPlan,
) -> Result<FleetComponentProvisioningPlanCounts, InternalError> {
    let directory_confirmation_roots = u32::try_from(plan.directory_confirmation_roots.len())
        .map_err(|_| receipt_invariant("Directory confirmation root count does not fit u32"))?;
    let root_batches = u32::try_from(plan.batches.len())
        .map_err(|_| receipt_invariant("root batch count does not fit u32"))?;
    let mut group_placements = 0_u32;
    let mut components = 0_u32;
    for batch in &plan.batches {
        let batch_placements = u32::try_from(batch.placements.len())
            .map_err(|_| receipt_invariant("root batch placement count does not fit u32"))?;
        group_placements = group_placements
            .checked_add(batch_placements)
            .ok_or_else(|| {
                receipt_invariant("Fleet Component provisioning placement count overflowed")
            })?;
        for placement in &batch.placements {
            let members = u32::try_from(placement.entries.len())
                .map_err(|_| receipt_invariant("group placement member count does not fit u32"))?;
            components = components.checked_add(members).ok_or_else(|| {
                receipt_invariant("Fleet Component provisioning Component count overflowed")
            })?;
        }
    }
    Ok(FleetComponentProvisioningPlanCounts {
        directory_confirmation_roots,
        root_batches,
        group_placements,
        components,
    })
}

struct GroupedRootLifecycleReferences {
    operation_journal: bool,
    placement_ledger: bool,
    fleet_service: bool,
}

impl GroupedRootLifecycleReferences {
    const fn is_empty(&self) -> bool {
        !self.operation_journal && !self.placement_ledger && !self.fleet_service
    }
}

fn require_component_plan_roots_unreserved(
    current: &FleetCoordinatorRegistryRecord,
    plan: &FleetComponentProvisioningPlan,
) -> Result<(), InternalError> {
    let selects_reserved_root = plan.batches.iter().any(|batch| {
        !batch.placements.is_empty()
            && current.root_draining_reservations.iter().any(|record| {
                record.response.request.expected_root.fleet_subnet_root
                    == batch.root.fleet_subnet_root
            })
    });
    if selects_reserved_root {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn require_grouped_root_lifecycle_open(
    current: &FleetCoordinatorRegistryRecord,
    fleet_subnet_root: Principal,
) -> Result<(), InternalError> {
    let references = grouped_root_lifecycle_references(current, fleet_subnet_root);
    if !references.is_empty() {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn grouped_root_lifecycle_references(
    current: &FleetCoordinatorRegistryRecord,
    fleet_subnet_root: Principal,
) -> GroupedRootLifecycleReferences {
    let operation_journal = current
        .component_provisioning
        .iter()
        .chain(current.component_scale_out.iter())
        .any(|record| component_operation_references_root(record, fleet_subnet_root));
    let placement_ledger = current
        .component_group_deployments
        .iter()
        .flat_map(|deployment| &deployment.placements)
        .any(|placement| placement.fleet_subnet_root == fleet_subnet_root);
    let fleet_service = current
        .registry
        .services
        .iter()
        .flat_map(|service| &service.members)
        .any(|member| member.fleet_subnet_root == fleet_subnet_root);
    GroupedRootLifecycleReferences {
        operation_journal,
        placement_ledger,
        fleet_service,
    }
}

fn component_operation_references_root(
    record: &FleetComponentProvisioningRecord,
    fleet_subnet_root: Principal,
) -> bool {
    record.plan.batches.iter().any(|batch| {
        batch.root.fleet_subnet_root == fleet_subnet_root && !batch.placements.is_empty()
    })
}

fn require_snapshot_root(
    current: &FleetCoordinatorRegistryRecord,
    caller: Principal,
) -> Result<&FleetSubnetRootEntry, InternalError> {
    current
        .registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| {
            entry.fleet_subnet_root == caller && entry.status != FleetSubnetRootStatus::Removed
        })
        .ok_or_else(InternalError::forbidden)
}

fn require_joining_root(
    current: &FleetCoordinatorRegistryRecord,
    caller: Principal,
) -> Result<&FleetSubnetRootEntry, InternalError> {
    current
        .registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| {
            entry.fleet_subnet_root == caller && entry.status == FleetSubnetRootStatus::Joining
        })
        .ok_or_else(InternalError::forbidden)
}

fn require_all_roots_joining(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    if current.registry.fleet_subnet_roots.is_empty()
        || current
            .registry
            .fleet_subnet_roots
            .iter()
            .any(|entry| entry.status != FleetSubnetRootStatus::Joining)
    {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn validate_root_snapshot_acknowledgements(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    let version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &current.registry,
    )?;
    let mut previous: Option<Principal> = None;
    for acknowledgement in &current.root_snapshot_acknowledgements {
        if acknowledgement.version != version
            || previous
                .as_ref()
                .is_some_and(|root| root.as_slice() >= acknowledgement.fleet_subnet_root.as_slice())
            || require_joining_root(current, acknowledgement.fleet_subnet_root).is_err()
        {
            return Err(receipt_invariant(
                "Fleet Subnet Root snapshot acknowledgements are not canonical",
            ));
        }
        previous = Some(acknowledgement.fleet_subnet_root);
    }
    Ok(())
}

fn require_complete_snapshot_acknowledgements(
    current: &FleetCoordinatorRegistryRecord,
    version: &FleetRegistryVersion,
) -> Result<(), InternalError> {
    if current.root_snapshot_acknowledgements.len() != current.registry.fleet_subnet_roots.len()
        || current.registry.fleet_subnet_roots.iter().any(|entry| {
            !current
                .root_snapshot_acknowledgements
                .iter()
                .any(|acknowledgement| {
                    acknowledgement.fleet_subnet_root == entry.fleet_subnet_root
                        && &acknowledgement.version == version
                })
        })
    {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn validate_registry_lifecycle_history(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    let history = canonical_registry_lifecycle_history(current)?;
    if history
        .last()
        .is_none_or(|point| point.registry != current.registry)
    {
        return Err(receipt_invariant(
            "Fleet Registry head differs from its canonical lifecycle history",
        ));
    }
    Ok(())
}

#[derive(Clone)]
struct FleetRegistryHistoryPoint {
    registry: FleetRegistry,
    version: FleetRegistryVersion,
}

#[derive(Clone, Copy)]
enum FleetSubnetRootLifecycleReceipt<'a> {
    Draining(&'a FleetSubnetRootDrainingPublicationReceiptRecord),
    Removed(&'a FleetSubnetRootRemovalPublicationReceiptRecord),
}

impl FleetSubnetRootLifecycleReceipt<'_> {
    const fn revision(self) -> u64 {
        match self {
            Self::Draining(receipt) => receipt.response.version.revision,
            Self::Removed(receipt) => receipt.response.version.revision,
        }
    }
}

fn canonical_registry_lifecycle_history(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<Vec<FleetRegistryHistoryPoint>, InternalError> {
    let joining = historical_joining_registry(current)?;
    let (mut historical_registry, mut history) = initial_lifecycle_history(current, joining)?;
    apply_service_publication_receipts(current, &mut historical_registry, &mut history)?;
    for lifecycle in canonical_lifecycle_receipts(current)? {
        apply_lifecycle_receipt(current, lifecycle, &mut historical_registry, &mut history)?;
    }
    Ok(history)
}

fn initial_lifecycle_history(
    current: &FleetCoordinatorRegistryRecord,
    joining: FleetRegistry,
) -> Result<(FleetRegistry, Vec<FleetRegistryHistoryPoint>), InternalError> {
    let Some(receipt) = &current.registry_activation_receipt else {
        let has_lifecycle_receipts = !current.service_publication_receipts.is_empty()
            || !current.root_draining_publication_receipts.is_empty()
            || !current.root_removal_publication_receipts.is_empty();
        if has_lifecycle_receipts || current.registry != joining {
            return Err(receipt_invariant(
                "Fleet Registry contains transitioned roots without an activation receipt",
            ));
        }
        let version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &joining,
        )?;
        return Ok((
            joining.clone(),
            vec![FleetRegistryHistoryPoint {
                registry: joining,
                version,
            }],
        ));
    };
    if !current.root_snapshot_acknowledgements.is_empty() {
        return Err(receipt_invariant(
            "active Fleet Registry retains stale Joining acknowledgements",
        ));
    }
    let previous_version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &joining,
    )
    .map_err(|_| receipt_invariant("activation source version cannot be derived"))?;
    let historical_registry = FleetRegistryOps::compile_active(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &joining,
    )
    .map_err(|_| receipt_invariant("activation target Registry cannot be derived"))?;
    let version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &historical_registry,
    )
    .map_err(|_| receipt_invariant("activation target version cannot be derived"))?;
    if receipt.request.expected_registry != previous_version
        || receipt.response.previous_version != previous_version
        || receipt.response.version != version
    {
        return Err(receipt_invariant(
            "Fleet Registry activation receipt differs from canonical history",
        ));
    }
    let history = vec![FleetRegistryHistoryPoint {
        registry: historical_registry.clone(),
        version,
    }];
    Ok((historical_registry, history))
}

fn initial_active_registry(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<FleetRegistry, InternalError> {
    let joining = historical_joining_registry(current)?;
    let (active, _) = initial_lifecycle_history(current, joining)?;
    if current.registry_activation_receipt.is_none() {
        return Err(InternalError::conflict());
    }
    Ok(active)
}

fn component_operation_source_registry(
    current: &FleetCoordinatorRegistryRecord,
    record: &FleetComponentProvisioningRecord,
) -> Result<FleetRegistry, InternalError> {
    match record.plan.operation {
        FleetComponentProvisioningOperation::FreshInstall => initial_active_registry(current),
        FleetComponentProvisioningOperation::ScaleOut { .. } => {
            registry_snapshot_at_version(current, &record.plan.fleet_registry)
        }
    }
}

fn registry_snapshot_at_version(
    current: &FleetCoordinatorRegistryRecord,
    version: &FleetRegistryVersion,
) -> Result<FleetRegistry, InternalError> {
    canonical_registry_lifecycle_history(current)?
        .into_iter()
        .find(|point| &point.version == version)
        .map(|point| point.registry)
        .ok_or_else(|| {
            receipt_invariant("Fleet Component operation source Registry is absent from history")
        })
}

fn apply_service_publication_receipts(
    current: &FleetCoordinatorRegistryRecord,
    historical_registry: &mut FleetRegistry,
    history: &mut Vec<FleetRegistryHistoryPoint>,
) -> Result<(), InternalError> {
    for receipt in &current.service_publication_receipts {
        apply_service_publication_receipt(current, receipt, historical_registry, history)?;
    }
    Ok(())
}

fn apply_service_publication_receipt(
    current: &FleetCoordinatorRegistryRecord,
    receipt: &FleetServicePublicationReceiptRecord,
    historical_registry: &mut FleetRegistry,
    history: &mut Vec<FleetRegistryHistoryPoint>,
) -> Result<(), InternalError> {
    if !FleetServicePublicationAuthority::from_receipt(receipt).is_complete() {
        return Err(receipt_invariant(
            "Fleet-service publication receipt authority is incomplete",
        ));
    }
    let previous_version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        historical_registry,
    )?;
    if receipt.previous_version != previous_version {
        return Err(receipt_invariant(
            "Fleet-service publication source differs from canonical history",
        ));
    }
    let next_registry = if receipt.services == historical_registry.services {
        historical_registry.clone()
    } else if historical_registry.services.is_empty() {
        FleetRegistryOps::compile_initial_services(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            historical_registry,
            receipt.services.clone(),
        )
        .map_err(|_| {
            receipt_invariant("initial Fleet-service publication target cannot be rederived")
        })?
    } else {
        FleetRegistryOps::compile_service_additions(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            historical_registry,
            receipt.services.clone(),
        )
        .map_err(|_| {
            receipt_invariant("scale-out Fleet-service publication target cannot be rederived")
        })?
    };
    let version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &next_registry,
    )?;
    if receipt.version != version {
        return Err(receipt_invariant(
            "Fleet-service publication response differs from canonical history",
        ));
    }
    *historical_registry = next_registry.clone();
    if history
        .last()
        .is_none_or(|point| point.registry != next_registry)
    {
        history.push(FleetRegistryHistoryPoint {
            registry: next_registry,
            version,
        });
    }
    Ok(())
}

#[derive(Eq, PartialEq)]
struct FleetServicePublicationAuthority<'a> {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    configuration_digest: canic_core::ids::ComponentDeploymentConfigurationDigest,
    root_receipt_content_hashes: &'a [[u8; 32]],
    services: &'a [canic_core::dto::fleet_registry::FleetServiceBinding],
}

impl<'a> FleetServicePublicationAuthority<'a> {
    fn from_receipt(receipt: &'a FleetServicePublicationReceiptRecord) -> Self {
        Self {
            operation_id: receipt.operation_id,
            plan_hash: receipt.plan_hash,
            configuration_digest: receipt.configuration_digest,
            root_receipt_content_hashes: &receipt.root_receipt_content_hashes,
            services: &receipt.services,
        }
    }

    fn is_complete(&self) -> bool {
        let identity_is_complete = [
            self.operation_id != [0; 32],
            self.plan_hash != [0; 32],
            self.configuration_digest.as_bytes() != &[0; 32],
        ]
        .into_iter()
        .all(|fact| fact);
        let receipt_hashes_are_complete = [
            !self.root_receipt_content_hashes.is_empty(),
            self.root_receipt_content_hashes.len() <= MAX_FLEET_COMPONENT_PROVISIONING_PLAN_BATCHES,
            self.root_receipt_content_hashes
                .iter()
                .all(|hash| hash != &[0; 32]),
        ]
        .into_iter()
        .all(|fact| fact);
        identity_is_complete && receipt_hashes_are_complete
    }
}

fn apply_lifecycle_receipt(
    current: &FleetCoordinatorRegistryRecord,
    lifecycle: FleetSubnetRootLifecycleReceipt<'_>,
    historical_registry: &mut FleetRegistry,
    history: &mut Vec<FleetRegistryHistoryPoint>,
) -> Result<(), InternalError> {
    let previous_version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        historical_registry,
    )
    .map_err(|_| receipt_invariant("root lifecycle source version cannot be derived"))?;
    let (next_registry, expected_response) = match lifecycle {
        FleetSubnetRootLifecycleReceipt::Draining(receipt) => {
            apply_draining_receipt(current, historical_registry, previous_version, receipt)?
        }
        FleetSubnetRootLifecycleReceipt::Removed(receipt) => apply_removal_receipt(
            current,
            historical_registry,
            history,
            previous_version,
            receipt,
        )?,
    };
    if !expected_response.matches(lifecycle) {
        return Err(receipt_invariant(
            "root lifecycle publication response differs from canonical history",
        ));
    }
    let version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &next_registry,
    )
    .map_err(|_| receipt_invariant("root lifecycle target version cannot be derived"))?;
    *historical_registry = next_registry.clone();
    history.push(FleetRegistryHistoryPoint {
        registry: next_registry,
        version,
    });
    Ok(())
}

fn apply_draining_receipt(
    current: &FleetCoordinatorRegistryRecord,
    historical_registry: &FleetRegistry,
    previous_version: FleetRegistryVersion,
    receipt: &FleetSubnetRootDrainingPublicationReceiptRecord,
) -> Result<(FleetRegistry, FleetSubnetRootLifecycleResponse), InternalError> {
    let reservation = draining_reservation_for_publication(current, &receipt.request)
        .map_err(|_| receipt_invariant("root draining publication reservation is missing"))?;
    validate_draining_publication_request(
        historical_registry,
        &previous_version,
        &receipt.request,
        reservation,
    )
    .map_err(|_| {
        receipt_invariant("root draining publication request differs from canonical history")
    })?;
    let next_registry = FleetRegistryOps::compile_draining(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        historical_registry,
        receipt.request.root_draining.fleet_subnet_root,
    )
    .map_err(|_| receipt_invariant("root draining target Registry cannot be derived"))?;
    let version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &next_registry,
    )?;
    let response =
        FleetSubnetRootLifecycleResponse::Draining(FleetSubnetRootDrainingPublicationResponse {
            root_draining: receipt.request.root_draining.clone(),
            previous_version,
            version,
        });
    Ok((next_registry, response))
}

fn apply_removal_receipt(
    current: &FleetCoordinatorRegistryRecord,
    historical_registry: &FleetRegistry,
    history: &[FleetRegistryHistoryPoint],
    previous_version: FleetRegistryVersion,
    receipt: &FleetSubnetRootRemovalPublicationReceiptRecord,
) -> Result<(FleetRegistry, FleetSubnetRootLifecycleResponse), InternalError> {
    validate_removal_publication_request(
        historical_registry,
        &previous_version,
        &current.root_draining_publication_receipts,
        history,
        &receipt.request,
    )
    .map_err(|_| {
        receipt_invariant("root removal publication request differs from canonical history")
    })?;
    let next_registry = FleetRegistryOps::compile_removed(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        historical_registry,
        receipt.request.final_inventory.fleet_subnet_root,
    )
    .map_err(|_| receipt_invariant("root removal target Registry cannot be derived"))?;
    let version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &next_registry,
    )?;
    let response =
        FleetSubnetRootLifecycleResponse::Removed(FleetSubnetRootRemovalPublicationResponse {
            final_inventory: receipt.request.final_inventory.clone(),
            previous_version,
            version,
        });
    Ok((next_registry, response))
}

enum FleetSubnetRootLifecycleResponse {
    Draining(FleetSubnetRootDrainingPublicationResponse),
    Removed(FleetSubnetRootRemovalPublicationResponse),
}

impl FleetSubnetRootLifecycleResponse {
    fn matches(&self, receipt: FleetSubnetRootLifecycleReceipt<'_>) -> bool {
        match (self, receipt) {
            (Self::Draining(expected), FleetSubnetRootLifecycleReceipt::Draining(receipt)) => {
                expected == &receipt.response
            }
            (Self::Removed(expected), FleetSubnetRootLifecycleReceipt::Removed(receipt)) => {
                expected == &receipt.response
            }
            _ => false,
        }
    }
}

fn canonical_lifecycle_receipts(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<Vec<FleetSubnetRootLifecycleReceipt<'_>>, InternalError> {
    validate_lifecycle_receipt_identities(current)?;
    let mut receipts = current
        .root_draining_publication_receipts
        .iter()
        .map(FleetSubnetRootLifecycleReceipt::Draining)
        .chain(
            current
                .root_removal_publication_receipts
                .iter()
                .map(FleetSubnetRootLifecycleReceipt::Removed),
        )
        .collect::<Vec<_>>();
    receipts.sort_by_key(|receipt| receipt.revision());
    if receipts
        .windows(2)
        .any(|pair| pair[0].revision() >= pair[1].revision())
    {
        return Err(receipt_invariant(
            "root lifecycle publication revisions are not unique and increasing",
        ));
    }
    Ok(receipts)
}

fn validate_lifecycle_receipt_identities(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    let mut draining_identities = Vec::new();
    for receipt in &current.root_draining_publication_receipts {
        let identity = FleetSubnetRootDrainingIdentity::from_publication_request(&receipt.request);
        if draining_identities
            .iter()
            .any(|existing| identity.conflicts_with(*existing))
        {
            return Err(receipt_invariant(
                "root draining publication identity is not unique",
            ));
        }
        draining_identities.push(identity);
    }
    let mut removal_identities = Vec::new();
    for receipt in &current.root_removal_publication_receipts {
        let identity = FleetSubnetRootRemovalPublicationIdentity::from_request(&receipt.request);
        if removal_identities
            .iter()
            .any(|existing| identity.conflicts_with(*existing))
        {
            return Err(receipt_invariant(
                "root removal publication identity is not unique",
            ));
        }
        removal_identities.push(identity);
    }
    Ok(())
}

fn validate_root_join_receipts(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    if current.root_join_receipts.len() != current.registry.fleet_subnet_roots.len() {
        return Err(receipt_invariant(
            "Fleet Registry root rows and durable join receipts differ",
        ));
    }

    let historical_registry = historical_joining_registry(current)?;
    for receipt in &current.root_join_receipts {
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
    if historical_registry.fleet_subnet_roots.len() != current.registry.fleet_subnet_roots.len() {
        return Err(receipt_invariant(
            "Fleet Registry join receipt history is incomplete",
        ));
    }
    Ok(())
}

fn historical_joining_registry(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<FleetRegistry, InternalError> {
    let mut historical_registry = FleetRegistryOps::compile_genesis(
        &current.configured_app,
        current.authority.clone(),
        &current
            .component_deployment_configuration
            .component_topology,
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
            &current
                .component_deployment_configuration
                .component_topology,
            &historical_registry,
            receipt.entry.clone(),
        )
        .map_err(|_| receipt_invariant("Fleet Registry join receipt history is not canonical"))?;
        let historical_version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &historical_registry,
        )
        .map_err(|_| receipt_invariant("Fleet Registry join receipt version cannot be derived"))?;
        if receipt.version != historical_version {
            return Err(receipt_invariant(
                "Fleet Registry join receipt version differs from its historical snapshot",
            ));
        }
    }
    Ok(historical_registry)
}

#[derive(Eq, PartialEq)]
struct FleetSubnetRootImmutableAuthority<'a> {
    placement_subnet: SubnetId,
    fleet_subnet_root: Principal,
    component_admissions: &'a [canic_core::ids::ComponentSpecAdmission],
    component_topology_digest: ComponentTopologyDigest,
    active_release_set: FleetSubnetRootReleaseSet,
    limits: &'a canic_core::ids::FleetSubnetRootLimits,
}

impl<'a> From<&'a FleetSubnetRootEntry> for FleetSubnetRootImmutableAuthority<'a> {
    fn from(entry: &'a FleetSubnetRootEntry) -> Self {
        Self {
            placement_subnet: entry.placement_subnet,
            fleet_subnet_root: entry.fleet_subnet_root,
            component_admissions: &entry.component_admissions,
            component_topology_digest: entry.component_topology_digest,
            active_release_set: entry.active_release_set,
            limits: &entry.limits,
        }
    }
}

fn same_root_authority(left: &FleetSubnetRootEntry, right: &FleetSubnetRootEntry) -> bool {
    FleetSubnetRootImmutableAuthority::from(left) == FleetSubnetRootImmutableAuthority::from(right)
}

#[derive(Eq, PartialEq)]
struct FleetSubnetRootDrainingAuthority {
    fleet_subnet_root: Principal,
    placement_subnet: SubnetId,
    component_topology_digest: ComponentTopologyDigest,
    active_release_set: FleetSubnetRootReleaseSet,
}

impl FleetSubnetRootDrainingAuthority {
    const fn from_registry(entry: &FleetSubnetRootEntry) -> Self {
        Self {
            fleet_subnet_root: entry.fleet_subnet_root,
            placement_subnet: entry.placement_subnet,
            component_topology_digest: entry.component_topology_digest,
            active_release_set: entry.active_release_set,
        }
    }

    const fn from_publication(request: &FleetSubnetRootDrainingPublicationRequest) -> Self {
        let draining = &request.root_draining;
        Self {
            fleet_subnet_root: draining.fleet_subnet_root,
            placement_subnet: draining.placement_subnet,
            component_topology_digest: draining.component_topology_digest,
            active_release_set: draining.active_release_set,
        }
    }
}

fn draining_publication_identity_matches(
    receipt: &FleetSubnetRootDrainingPublicationReceiptRecord,
    request: &FleetSubnetRootDrainingPublicationRequest,
) -> bool {
    FleetSubnetRootDrainingIdentity::from_publication_request(&receipt.request).conflicts_with(
        FleetSubnetRootDrainingIdentity::from_publication_request(request),
    )
}

fn draining_reservation_for_publication<'a>(
    current: &'a FleetCoordinatorRegistryRecord,
    request: &FleetSubnetRootDrainingPublicationRequest,
) -> Result<&'a FleetSubnetRootDrainingReservationResponse, InternalError> {
    let publication_identity = FleetSubnetRootDrainingIdentity::from_publication_request(request);
    let reservation = current
        .root_draining_reservations
        .iter()
        .find(|record| {
            FleetSubnetRootDrainingIdentity::from_reservation_request(&record.response.request)
                .conflicts_with(publication_identity)
        })
        .ok_or_else(InternalError::unavailable)?;
    let reservation_identity =
        FleetSubnetRootDrainingIdentity::from_reservation_request(&reservation.response.request);
    if reservation_identity != publication_identity {
        return Err(InternalError::conflict());
    }
    Ok(&reservation.response)
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct FleetSubnetRootDrainingIdentity {
    fleet_subnet_root: Principal,
    operation_id: [u8; 32],
}

impl FleetSubnetRootDrainingIdentity {
    const fn from_publication_request(request: &FleetSubnetRootDrainingPublicationRequest) -> Self {
        Self {
            fleet_subnet_root: request.root_draining.fleet_subnet_root,
            operation_id: request.root_draining.operation_id,
        }
    }

    const fn from_reservation_request(request: &FleetSubnetRootDrainingReservationRequest) -> Self {
        Self {
            fleet_subnet_root: request.expected_root.fleet_subnet_root,
            operation_id: request.operation_id,
        }
    }

    const fn from_reservation_status(
        request: &FleetSubnetRootDrainingReservationStatusRequest,
    ) -> Self {
        Self {
            fleet_subnet_root: request.fleet_subnet_root,
            operation_id: request.operation_id,
        }
    }

    fn conflicts_with(self, other: Self) -> bool {
        self.fleet_subnet_root == other.fleet_subnet_root || self.operation_id == other.operation_id
    }
}

fn draining_reservation_identity_matches(
    response: &FleetSubnetRootDrainingReservationResponse,
    request: &FleetSubnetRootDrainingReservationRequest,
) -> bool {
    FleetSubnetRootDrainingIdentity::from_reservation_request(&response.request).conflicts_with(
        FleetSubnetRootDrainingIdentity::from_reservation_request(request),
    )
}

fn draining_reservation_status_matches(
    response: &FleetSubnetRootDrainingReservationResponse,
    request: &FleetSubnetRootDrainingReservationStatusRequest,
) -> bool {
    FleetSubnetRootDrainingIdentity::from_reservation_request(&response.request).conflicts_with(
        FleetSubnetRootDrainingIdentity::from_reservation_status(request),
    )
}

fn validate_root_draining_reservation_request(
    current: &FleetCoordinatorRegistryRecord,
    version: &FleetRegistryVersion,
    request: &FleetSubnetRootDrainingReservationRequest,
) -> Result<(), InternalError> {
    if request.expected_registry != *version {
        return Err(InternalError::conflict());
    }
    if request.expected_root.status != FleetSubnetRootStatus::Active {
        return Err(InternalError::invalid_input());
    }
    let Some(target) = current
        .registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| entry.fleet_subnet_root == request.expected_root.fleet_subnet_root)
    else {
        return Err(InternalError::conflict());
    };
    if target != &request.expected_root {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn validate_draining_publication_request(
    registry: &FleetRegistry,
    version: &FleetRegistryVersion,
    request: &FleetSubnetRootDrainingPublicationRequest,
    reservation: &FleetSubnetRootDrainingReservationResponse,
) -> Result<(), &'static str> {
    let draining = &request.root_draining;
    if request.expected_registry != *version {
        return Err("Fleet Subnet Root draining publication names stale Registry authority");
    }
    let reservation_matches_receipt = [
        reservation.request.operation_id == draining.operation_id,
        reservation.request.expected_registry == draining.active_registry,
        reservation.request.expected_root.fleet_subnet_root == draining.fleet_subnet_root,
        reservation.request.expected_root.status == FleetSubnetRootStatus::Active,
        reservation.reservation_hash != [0; 32],
        reservation.reservation_hash == draining.reservation_hash,
    ]
    .into_iter()
    .all(|valid| valid);
    if !reservation_matches_receipt {
        return Err("Fleet Subnet Root draining receipt differs from its retained reservation");
    }
    let target = registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| entry.fleet_subnet_root == draining.fleet_subnet_root)
        .ok_or("Fleet Subnet Root draining publication target is missing")?;
    if target.status != FleetSubnetRootStatus::Active {
        return Err("Fleet Subnet Root draining publication target is not Active");
    }
    if target != &reservation.request.expected_root {
        return Err("Fleet Subnet Root draining reservation differs from current root authority");
    }
    let expected_authority = FleetSubnetRootDrainingAuthority::from_registry(target);
    if FleetSubnetRootDrainingAuthority::from_publication(request) != expected_authority {
        return Err("Fleet Subnet Root draining receipt differs from Registry root authority");
    }
    if draining.operation_id == [0; 32]
        || draining.started_at_ns == 0
        || draining.next_allocation_sequence == 0
    {
        return Err("Fleet Subnet Root draining receipt contains non-positive operation facts");
    }
    let component_instances = draining
        .reserved_component_instances
        .checked_add(draining.committed_component_instances)
        .ok_or("Fleet Subnet Root draining Component Instance count overflowed")?;
    if component_instances > target.limits.maximum_component_instances {
        return Err("Fleet Subnet Root draining Component Instance count exceeds its limit");
    }
    if draining.next_allocation_sequence <= u64::from(component_instances) {
        return Err("Fleet Subnet Root draining allocation sequence precedes its live instances");
    }
    let allocated_canisters = component_instances
        .checked_add(draining.managed_descendants)
        .ok_or("Fleet Subnet Root draining managed canister count overflowed")?;
    if draining.known_created_component_canisters > allocated_canisters {
        return Err("Fleet Subnet Root draining created canisters exceed allocated canisters");
    }
    if draining.root_registry_encoded_bytes > target.limits.maximum_registry_bytes {
        return Err("Fleet Subnet Root draining Registry bytes exceed the root limit");
    }
    Ok(())
}

fn removal_publication_identity_matches(
    receipt: &FleetSubnetRootRemovalPublicationReceiptRecord,
    request: &FleetSubnetRootRemovalPublicationRequest,
) -> bool {
    FleetSubnetRootRemovalPublicationIdentity::from_request(&receipt.request).conflicts_with(
        FleetSubnetRootRemovalPublicationIdentity::from_request(request),
    )
}

#[derive(Clone, Copy)]
struct FleetSubnetRootRemovalPublicationIdentity {
    fleet_subnet_root: Principal,
    operation_id: [u8; 32],
}

impl FleetSubnetRootRemovalPublicationIdentity {
    const fn from_request(request: &FleetSubnetRootRemovalPublicationRequest) -> Self {
        Self {
            fleet_subnet_root: request.final_inventory.fleet_subnet_root,
            operation_id: request.final_inventory.operation_id,
        }
    }

    fn conflicts_with(self, other: Self) -> bool {
        self.fleet_subnet_root == other.fleet_subnet_root || self.operation_id == other.operation_id
    }
}

#[derive(Eq, PartialEq)]
struct FleetSubnetRootFinalInventoryAuthority {
    fleet_subnet_root: Principal,
    placement_subnet: SubnetId,
    component_topology_digest: ComponentTopologyDigest,
    active_release_set: FleetSubnetRootReleaseSet,
}

impl FleetSubnetRootFinalInventoryAuthority {
    const fn from_registry(entry: &FleetSubnetRootEntry) -> Self {
        Self {
            fleet_subnet_root: entry.fleet_subnet_root,
            placement_subnet: entry.placement_subnet,
            component_topology_digest: entry.component_topology_digest,
            active_release_set: entry.active_release_set,
        }
    }

    const fn from_publication(request: &FleetSubnetRootRemovalPublicationRequest) -> Self {
        let inventory = &request.final_inventory;
        Self {
            fleet_subnet_root: inventory.fleet_subnet_root,
            placement_subnet: inventory.placement_subnet,
            component_topology_digest: inventory.component_topology_digest,
            active_release_set: inventory.active_release_set,
        }
    }
}

fn validate_removal_publication_request(
    registry: &FleetRegistry,
    version: &FleetRegistryVersion,
    draining_receipts: &[FleetSubnetRootDrainingPublicationReceiptRecord],
    history: &[FleetRegistryHistoryPoint],
    request: &FleetSubnetRootRemovalPublicationRequest,
) -> Result<(), &'static str> {
    let inventory = &request.final_inventory;
    if request.expected_registry != *version {
        return Err("Fleet Subnet Root removal publication names stale Registry authority");
    }
    let target = registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| entry.fleet_subnet_root == inventory.fleet_subnet_root)
        .ok_or("Fleet Subnet Root removal publication target is missing")?;
    if target.status != FleetSubnetRootStatus::Draining {
        return Err("Fleet Subnet Root removal publication target is not Draining");
    }
    if FleetSubnetRootFinalInventoryAuthority::from_publication(request)
        != FleetSubnetRootFinalInventoryAuthority::from_registry(target)
    {
        return Err("Fleet Subnet Root final inventory differs from Registry root authority");
    }
    let draining = draining_receipts
        .iter()
        .find(|receipt| {
            receipt.request.root_draining.fleet_subnet_root == inventory.fleet_subnet_root
                && receipt.request.root_draining.operation_id == inventory.operation_id
        })
        .ok_or("Fleet Subnet Root final inventory lacks its draining publication")?;
    if inventory.finalized_at_ns < draining.request.root_draining.started_at_ns {
        return Err("Fleet Subnet Root final inventory predates its draining publication");
    }
    let source = history
        .iter()
        .find(|point| point.version == inventory.registry)
        .ok_or("Fleet Subnet Root final inventory Registry is not canonical history")?;
    let source_is_draining = source.registry.fleet_subnet_roots.iter().any(|entry| {
        entry.fleet_subnet_root == inventory.fleet_subnet_root
            && entry.status == FleetSubnetRootStatus::Draining
    });
    if !source_is_draining {
        return Err("Fleet Subnet Root was not Draining at its final inventory Registry");
    }
    let removed_instances_are_exact = u64::from(inventory.removed_component_instances)
        == inventory.next_allocation_sequence.saturating_sub(1);
    let expected_store_entries = inventory
        .wasm_store_catalog_entries
        .checked_add(1)
        .ok_or("Fleet Subnet Root final inventory Store count overflows")?;
    let terminal_facts_are_exact = [
        inventory.operation_id != [0; 32],
        inventory.next_allocation_sequence > 0,
        removed_instances_are_exact,
        inventory.terminal_component_history_hash != [0; 32],
        inventory.root_registry_encoded_bytes <= target.limits.maximum_registry_bytes,
        inventory.wasm_store != Principal::anonymous(),
        inventory.wasm_store_catalog_hash != [0; 32],
        inventory.wasm_store_catalog_entries > 0,
        inventory.wasm_store_release_count == expected_store_entries,
        inventory.wasm_store_template_count == expected_store_entries,
        inventory.wasm_store_occupied_bytes <= target.limits.maximum_wasm_store_bytes,
        inventory.wasm_store_gc_prepared_at_secs > 0,
        inventory.finalized_at_ns > 0,
        inventory.inventory_hash != [0; 32],
    ]
    .into_iter()
    .all(|valid| valid);
    if !terminal_facts_are_exact {
        return Err("Fleet Subnet Root final inventory contains invalid terminal authority");
    }
    Ok(())
}

fn validate_root_draining_reservations(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    if current.root_draining_reservations.len() > current.registry.fleet_subnet_roots.len() {
        return Err(receipt_invariant(
            "Fleet Subnet Root draining reservation count exceeds the Fleet root count",
        ));
    }
    let mut identities = Vec::new();
    for record in &current.root_draining_reservations {
        let response = &record.response;
        let request = &response.request;
        let identity = FleetSubnetRootDrainingIdentity::from_reservation_request(request);
        if identities
            .iter()
            .any(|existing| identity.conflicts_with(*existing))
        {
            return Err(receipt_invariant(
                "Fleet Subnet Root draining reservation identity is not unique",
            ));
        }
        identities.push(identity);

        let source_registry = registry_snapshot_at_version(current, &request.expected_registry)?;
        let source_root = source_registry
            .fleet_subnet_roots
            .iter()
            .find(|entry| entry.fleet_subnet_root == request.expected_root.fleet_subnet_root)
            .ok_or_else(|| {
                receipt_invariant("Fleet Subnet Root draining reservation source root is missing")
            })?;
        let response_is_exact = [
            request.operation_id != [0; 32],
            request.expected_root.status == FleetSubnetRootStatus::Active,
            source_root == &request.expected_root,
            response.coordinator == current.authority.binding.coordinator,
            response.prepared_at_ns > 0,
            response.reservation_hash != [0; 32],
            response.reservation_hash
                == FleetSubnetRootDrainingReservationOps::content_hash(response)?,
        ]
        .into_iter()
        .all(|valid| valid);
        if !response_is_exact {
            return Err(receipt_invariant(
                "Fleet Subnet Root draining reservation is not canonical",
            ));
        }
        require_grouped_root_lifecycle_open(current, source_root.fleet_subnet_root).map_err(
            |_| {
                receipt_invariant(
                    "Fleet Subnet Root draining reservation conflicts with grouped authority",
                )
            },
        )?;
    }
    Ok(())
}

const fn receipt_invariant(_message: &'static str) -> InternalError {
    InternalError::invariant()
}
