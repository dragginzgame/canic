//! Module: ops::fleet_coordinator
//!
//! Responsibility: validate, compile, commit, and read Fleet Coordinator Registry state.
//! Does not own: endpoint authorization, multi-step lifecycle orchestration, or root effects.
//! Boundary: workflow supplies protected init facts and receives canonical Registry projections.

mod admission;
mod component_provisioning_directory;
mod component_provisioning_progress;
mod component_provisioning_projection;
mod component_provisioning_reconciliation;
mod component_provisioning_retry;
mod component_provisioning_root_progress;
mod component_provisioning_validation;
mod deployment_ledger;
mod funding_rotation;
mod registry_history;
mod root_deletion;
mod root_funding;
mod root_lifecycle;
mod service_publication;

use admission::apply_admission_publication_to_registry;
use component_provisioning_directory::*;
use component_provisioning_progress::*;
use component_provisioning_projection::*;
use component_provisioning_reconciliation::*;
use component_provisioning_retry::*;
use component_provisioning_root_progress::*;
use component_provisioning_validation::*;
use registry_history::{
    canonical_registry_lifecycle_history, component_operation_source_registry,
    initial_active_registry, registry_snapshot_at_version, validate_registry_lifecycle_history,
    validate_root_join_receipts,
};
use root_deletion::validate_root_deletion_history;
use root_lifecycle::{
    draining_publication_identity_matches, draining_reservation_for_publication,
    draining_reservation_identity_matches, draining_reservation_status_matches,
    removal_publication_identity_matches, require_all_roots_joining,
    require_complete_snapshot_acknowledgements, require_component_plan_roots_unreserved,
    require_grouped_root_lifecycle_open, require_joining_root, require_snapshot_root,
    validate_draining_publication_request, validate_removal_publication_request,
    validate_root_draining_reservation_request, validate_root_draining_reservations,
    validate_root_snapshot_acknowledgements,
};
use service_publication::*;

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
            FleetComponentProvisioningRetryStage, FleetComponentProvisioningRootFailure,
            FleetComponentProvisioningRootProgress, FleetComponentProvisioningStatusRequest,
            FleetComponentProvisioningStatusResponse, FleetComponentPublicationRootProgress,
            FleetComponentSynchronizationRootProgress, FleetSubnetRootProvisioningBatch,
            RootComponentActivationRequest, RootComponentDirectorySynchronizationRequest,
            RootComponentDirectorySynchronizationResponse,
            RootComponentProvisioningAcceptanceRequest, RootComponentProvisioningAdvanceRequest,
            RootComponentProvisioningPhase, RootComponentProvisioningStatusResponse,
            RootComponentPublicationRequest, RootEstateFundingRequired,
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
        ComponentDeploymentConfigurationDigest, ComponentGroupDeploymentId, FleetRegistryAuthority,
        MAX_FLEET_ROOT_FUNDING_SLOTS,
    },
    shared_support::fleet_admission_authority::MAX_FLEET_ADMISSION_PUBLICATIONS,
    shared_support::fleet_funding_policy::{
        fleet_funding_policy_rotation_successor_policy_set_hash,
        validate_coordinator_root_funding_policy, validate_fleet_root_funding_admission,
        validate_fleet_root_funding_capacity,
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
    /// Return whether another Coordinator operation domain retains this exact identity.
    pub(crate) fn retains_operation_id(operation_id: [u8; 32]) -> Result<bool, InternalError> {
        if operation_id == [0; 32] {
            return Ok(false);
        }
        if Self::current()?
            .admission_publications
            .iter()
            .any(|publication| publication.operation_id == operation_id)
        {
            return Ok(true);
        }
        match Self::operation_status(operation_id) {
            Ok(_) => Ok(true),
            Err(error)
                if error.public_error().code()
                    == canic_core::diagnostics::codes::STATE_UNAVAILABLE.raw_code() =>
            {
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

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
        if let Some(status) = Self::funding_policy_rotation_status(operation_id)? {
            return Ok(CoordinatorOperationStatusResponse::FundingPolicyRotation(
                status,
            ));
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
        let initial_admission_policy = args.admission.clone();
        let registry = FleetRegistryOps::compile_genesis(
            &args.configured_app,
            args.authority.clone(),
            component_topology,
            args.admission,
        )?;
        Ok(FleetCoordinatorRegistryRecord {
            configured_app: args.configured_app,
            authority: args.authority,
            component_deployment_configuration: args.component_deployment_configuration,
            root_funding: args.root_funding,
            initial_admission_policy,
            registry,
            root_join_receipts: Vec::new(),
            root_snapshot_acknowledgements: Vec::new(),
            registry_activation_receipt: None,
            admission_publications: Vec::new(),
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
            last_root_failure: None,
            estate_funding_required: None,
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
            last_root_failure: None,
            estate_funding_required: None,
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

    pub(crate) fn record_component_provisioning_root_failure(
        request: FleetComponentProvisioningStatusRequest,
        diagnostic_code: u16,
        failed_at_ns: u64,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        if diagnostic_code == 0 || failed_at_ns == 0 {
            return Err(InternalError::invalid_input());
        }
        let current = Self::current()?;
        let record = active_provisioning_record_for_status(&current, &request)?
            .ok_or_else(InternalError::unavailable)?;
        let Some(authority) = current_component_provisioning_retry_authority(&record.state) else {
            return component_provisioning_status_response(record);
        };
        if failed_at_ns < authority.started_at_ns
            || record
                .last_root_failure
                .is_some_and(|failure| failed_at_ns < failure.failed_at_ns)
        {
            return Err(InternalError::invalid_input());
        }
        let failure = FleetComponentProvisioningRootFailure {
            fleet_subnet_root: authority.fleet_subnet_root,
            stage: authority.stage,
            diagnostic_code,
            failed_at_ns,
        };
        if record.last_root_failure == Some(failure) {
            return component_provisioning_status_response(record);
        }
        let mut next = current.clone();
        let next_record =
            component_provisioning_operation_record_mut(&mut next, request.operation_id)?;
        next_record.last_root_failure = Some(failure);
        let next = Self::validate_current(next)?;
        let response = component_provisioning_status_response(
            component_provisioning_operation_record(&next, request.operation_id)?,
        )?;
        Self::commit_transition(&current, next)?;
        Ok(response)
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
            return replay_recorded_root_acceptance(
                &current.component_deployment_configuration,
                record,
                request,
                &response,
                recorded_at_ns,
                &progress,
            );
        }
        if progress.accepted_root_count != request.expected_accepted_root_count {
            return Err(InternalError::conflict());
        }
        let intent = progress.in_flight.ok_or_else(InternalError::conflict)?;
        let batch = root_batch(record, intent.root_index)?;
        let response = canonical_root_acceptance_observation(
            &current.component_deployment_configuration,
            record,
            intent.root_index,
            batch,
            &response,
            intent.started_at_ns,
            recorded_at_ns,
        )?;
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
        let next_record =
            component_provisioning_operation_record_mut(&mut next, request.operation_id)?;
        next_record.estate_funding_required = None;
        next_record.state = FleetComponentProvisioningStateRecord::ProvisioningRoots {
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
        if response.estate_funding_required.is_some() {
            return Err(InternalError::conflict());
        }
        let acceptance = component_provisioning_root_acceptance(record, intent.root_index)?;
        validate_root_provision_response(RootProvisionResponseValidation {
            configuration: &current.component_deployment_configuration,
            record,
            root_index: intent.root_index,
            acceptance: &acceptance,
            previous,
            response: &response,
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
        next_record.estate_funding_required = None;
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

    pub(crate) fn record_component_provisioning_estate_funding_pause(
        request: &FleetComponentProvisioningAdvanceRequest,
        response: RootComponentProvisioningStatusResponse,
        recorded_at_ns: u64,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        let current = Self::current()?;
        let record = require_component_provisioning_operation_record(&current, request)?;
        let mut progress = component_provisioning_root_provision_progress(record)?;
        if classify_root_provision_advance(request, &progress)? != RootProvisionAdvance::Reconcile {
            return Err(InternalError::conflict());
        }
        let intent = progress
            .in_flight
            .take()
            .ok_or_else(|| receipt_invariant("estate funding pause lost its Root intent"))?;
        if recorded_at_ns < intent.started_at_ns {
            return Err(InternalError::invalid_input());
        }
        let previous = progress.current_response.as_ref().ok_or_else(|| {
            receipt_invariant("estate funding pause has no durable Root predecessor")
        })?;
        require_same_root_progress_ignoring_estate_funding(previous, &response)?;
        let funding = response
            .estate_funding_required
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        validate_estate_funding_pause(funding, &intent, recorded_at_ns)?;

        let acceptance_progress = component_provisioning_root_acceptance_progress(record)?;
        let roots_accepted_at_ns = progress
            .roots_accepted_at_ns
            .ok_or_else(|| receipt_invariant("estate funding pause lost its RootsAccepted time"))?;
        let mut next = current.clone();
        let next_record =
            component_provisioning_operation_record_mut(&mut next, request.operation_id)?;
        next_record.state = FleetComponentProvisioningStateRecord::ProvisioningRoots {
            planned_at_ns: acceptance_progress.planned_at_ns,
            acceptances: acceptance_progress.acceptances,
            roots_accepted_at_ns,
            provisions: progress.provisions,
            current: progress.current.map(Box::new),
            in_flight: None,
        };
        next_record.last_root_failure = None;
        next_record.estate_funding_required = Some(funding.clone());
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
        if current.admission_publications.len() > MAX_FLEET_ADMISSION_PUBLICATIONS
            || current
                .admission_publications
                .windows(2)
                .any(|pair| pair[0].version.revision >= pair[1].version.revision)
        {
            return Err(InternalError::invariant());
        }
        let mut admission_operation_ids = BTreeSet::new();
        if current.admission_publications.iter().any(|publication| {
            publication.operation_id == [0; 32]
                || !admission_operation_ids.insert(publication.operation_id)
        }) {
            return Err(InternalError::invariant());
        }
        match current.root_funding.as_ref() {
            Some(policy) => {
                validate_coordinator_root_funding_policy(policy)
                    .map_err(|_error| InternalError::invariant())?;
                for root in &current.registry.fleet_subnet_roots {
                    validate_fleet_root_funding_admission(policy, &root.funding)
                        .map_err(|_error| InternalError::invariant())?;
                }
                if current.registry_activation_receipt.is_some() {
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
        if crate::storage::stable::fleet_coordinator::FleetCoordinatorFundingStore::export()
            .current
            .is_some_and(|funding| funding.rotation_current.is_some())
        {
            return Err(InternalError::conflict());
        }
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

#[derive(Clone, Copy)]
struct FleetComponentProvisioningPlanCounts {
    directory_confirmation_roots: u32,
    root_batches: u32,
    group_placements: u32,
    components: u32,
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

fn require_same_root_progress_ignoring_estate_funding(
    previous: &RootComponentProvisioningStatusResponse,
    response: &RootComponentProvisioningStatusResponse,
) -> Result<(), InternalError> {
    let mut previous = previous.clone();
    previous.estate_funding_required = None;
    let mut response = response.clone();
    response.estate_funding_required = None;
    if previous != response {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn validate_estate_funding_pause(
    funding: &RootEstateFundingRequired,
    intent: &FleetComponentProvisioningRootProvisionIntentRecord,
    recorded_at_ns: u64,
) -> Result<(), InternalError> {
    let required = funding
        .creation_amount
        .to_u128()
        .checked_add(funding.ledger_fee.to_u128())
        .ok_or_else(InternalError::resource_exhausted)?;
    let creation_amount = funding
        .readiness_floor
        .to_u128()
        .checked_add(funding.execution_margin.to_u128())
        .and_then(|amount| amount.checked_add(funding.management_creation_fee.to_u128()))
        .ok_or_else(InternalError::resource_exhausted)?;
    let exact_arithmetic = funding.required.to_u128() == required
        && funding.creation_amount.to_u128() == creation_amount
        && funding.shortfall.to_u128() == required.saturating_sub(funding.available.to_u128())
        && funding.available < funding.required;
    let exact_authority = funding.root == intent.fleet_subnet_root
        && funding.operation_id != [0; 32]
        && funding.readiness_floor.to_u128() > 0
        && funding.execution_margin.to_u128() > 0;
    let exact_timing = funding.retry_at_ns > 0
        && funding
            .last_attempt_at_ns
            .is_none_or(|attempted_at_ns| attempted_at_ns > 0 && attempted_at_ns <= recorded_at_ns);
    if !exact_arithmetic || !exact_authority || !exact_timing {
        return Err(InternalError::conflict());
    }
    Ok(())
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
    configuration: &canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
    record: &FleetComponentProvisioningRecord,
    request: &FleetComponentProvisioningAdvanceRequest,
    response: &RootComponentProvisioningStatusResponse,
    recorded_at_ns: u64,
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
    let root_index = request.expected_accepted_root_count;
    let batch = root_batch(record, root_index)?;
    let canonical = canonical_root_acceptance_observation(
        configuration,
        record,
        root_index,
        batch,
        response,
        recorded.started_at_ns,
        recorded_at_ns,
    )?;
    if recorded.response != canonical {
        return Err(InternalError::conflict());
    }
    component_provisioning_status_response(record)
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

const fn receipt_invariant(_message: &'static str) -> InternalError {
    InternalError::invariant()
}

#[cfg(test)]
mod runtime_activation_progress_tests {
    use super::*;

    const fn progress(
        fleet_subnet_root: Principal,
        component_count: u32,
        activated_component_count: u32,
        root_runtime_active: bool,
    ) -> FleetComponentActivationRootProgress {
        FleetComponentActivationRootProgress {
            fleet_subnet_root,
            component_count,
            activated_component_count,
            root_runtime_active,
        }
    }

    #[test]
    fn coalesced_runtime_activation_progress_is_a_strict_monotonic_successor() {
        let root = Principal::from_slice(&[1]);
        let zero = progress(root, 5, 0, false);
        let one = progress(root, 5, 1, false);
        let five = progress(root, 5, 5, false);

        assert!(first_component_activation_progress(progress(
            root, 5, 3, false
        )));
        assert!(first_component_activation_progress(progress(
            root, 5, 5, true
        )));
        assert!(activation_progress_advances(
            one,
            progress(root, 5, 5, false)
        ));
        assert!(activation_progress_advances(
            one,
            progress(root, 5, 5, true)
        ));
        assert!(activation_progress_advances(
            five,
            progress(root, 5, 5, true)
        ));
        assert!(!activation_progress_advances(zero, zero));
    }

    #[test]
    fn invalid_or_regressing_runtime_activation_progress_fails_closed() {
        let root = Principal::from_slice(&[1]);
        let other_root = Principal::from_slice(&[2]);
        let previous = progress(root, 5, 3, false);

        for invalid in [
            progress(root, 5, 2, false),
            progress(root, 5, 6, false),
            progress(root, 5, 4, true),
            progress(other_root, 5, 5, true),
            progress(root, 6, 5, true),
        ] {
            assert!(!activation_progress_advances(previous, invalid));
        }
        assert!(!activation_progress_advances(
            progress(root, 5, 5, true),
            progress(root, 5, 5, false)
        ));
        assert!(!first_component_activation_progress(progress(
            root, 5, 0, false
        )));
    }
}

#[cfg(test)]
mod directory_publication_progress_tests {
    use super::*;

    const fn progress(
        fleet_subnet_root: Principal,
        component_count: u32,
        published_component_count: u32,
    ) -> FleetComponentPublicationRootProgress {
        FleetComponentPublicationRootProgress {
            fleet_subnet_root,
            component_count,
            published_component_count,
        }
    }

    #[test]
    fn coalesced_directory_publication_progress_is_a_strict_monotonic_successor() {
        let root = Principal::from_slice(&[1]);
        let zero = progress(root, 5, 0);
        let three = progress(root, 5, 3);
        let five = progress(root, 5, 5);

        for first in [zero, three, five] {
            assert!(publication_progress_replays(None, Some(first)));
        }
        assert!(publication_progress_advances(zero, three));
        assert!(publication_progress_advances(three, five));
        assert!(!publication_progress_advances(three, three));
    }

    #[test]
    fn invalid_or_regressing_directory_publication_progress_fails_closed() {
        let root = Principal::from_slice(&[1]);
        let other_root = Principal::from_slice(&[2]);
        let previous = progress(root, 5, 3);

        for invalid in [
            progress(root, 5, 2),
            progress(root, 5, 6),
            progress(other_root, 5, 5),
            progress(root, 6, 5),
        ] {
            assert!(!publication_progress_advances(previous, invalid));
        }
        assert!(!publication_progress_replays(None, None));
        assert!(!publication_progress_replays(
            None,
            Some(progress(root, 5, 6))
        ));
    }
}
