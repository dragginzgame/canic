//! Module: ops::fleet_coordinator
//!
//! Responsibility: validate, compile, commit, and read Fleet Coordinator Registry state.
//! Does not own: endpoint authorization, multi-step lifecycle orchestration, or root effects.
//! Boundary: workflow supplies protected init facts and receives canonical Registry projections.

mod deployment_ledger;

use crate::{
    dto::fleet_coordinator::FleetCoordinatorInitArgs,
    storage::stable::fleet_coordinator::{
        FleetComponentDirectoryConfirmationIntentRecord, FleetComponentDirectoryConfirmationRecord,
        FleetComponentProvisioningRecord, FleetComponentProvisioningRootAcceptanceIntentRecord,
        FleetComponentProvisioningRootAcceptanceRecord,
        FleetComponentProvisioningRootProvisionIntentRecord,
        FleetComponentProvisioningRootProvisionRecord, FleetComponentProvisioningStateRecord,
        FleetComponentRuntimeActivationIntentRecord, FleetComponentRuntimeActivationRecord,
        FleetCoordinatorCommitError, FleetCoordinatorCommitOutcome, FleetCoordinatorRegistryRecord,
        FleetCoordinatorRegistryStore, FleetRegistryActivationReceiptRecord,
        FleetServicePublicationReceiptRecord, FleetSubnetRootDrainingPublicationReceiptRecord,
        FleetSubnetRootJoinReceiptRecord, FleetSubnetRootRemovalPublicationReceiptRecord,
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
use candid::{CandidType, Principal};
#[cfg(test)]
use canic_core::control_plane_support::config::ConfigModel;
use canic_core::{
    control_plane_support::{
        error::{InternalError, InternalErrorOrigin},
        ops::{
            component_provisioning_plan::{
                ComponentProvisioningPlanOps, MAX_FLEET_COMPONENT_PROVISIONING_PLAN_BATCHES,
            },
            component_provisioning_receipt::{
                RootComponentProvisioningAcceptanceReceiptAuthority,
                RootComponentProvisioningPublishedReceiptAuthority,
                RootComponentProvisioningReceiptOps,
                RootComponentProvisioningRuntimesActiveReceiptAuthority,
            },
            fleet_registry::FleetRegistryOps,
            fleet_service_binding::FleetServiceBindingOps,
        },
    },
    dto::fleet_subnet_root::FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES,
    dto::{
        component_provisioning::{
            FleetComponentActivationRootProgress, FleetComponentProvisioningAdvanceRequest,
            FleetComponentProvisioningOperation, FleetComponentProvisioningPhase,
            FleetComponentProvisioningPlan, FleetComponentProvisioningPrepareRequest,
            FleetComponentProvisioningRootProgress, FleetComponentProvisioningStatusRequest,
            FleetComponentProvisioningStatusResponse, FleetComponentPublicationRootProgress,
            FleetSubnetRootProvisioningBatch, RootComponentActivationRequest,
            RootComponentProvisioningAcceptanceRequest, RootComponentProvisioningAdvanceRequest,
            RootComponentProvisioningPhase, RootComponentProvisioningStatusResponse,
            RootComponentPublicationRequest,
        },
        fleet_registry::{
            FleetRegistry, FleetRegistryActivationRequest, FleetRegistryActivationResponse,
            FleetRegistryManifest, FleetRegistrySnapshotResponse, FleetRegistryVersion,
            FleetSubnetRootDeletionCompletionRequest, FleetSubnetRootDeletionExecutionRequest,
            FleetSubnetRootDeletionExecutionResponse,
            FleetSubnetRootDeletionReadinessIntentRequest,
            FleetSubnetRootDeletionReadinessIntentResponse,
            FleetSubnetRootDeletionReadinessRequest, FleetSubnetRootDeletionReadinessResponse,
            FleetSubnetRootDeletionResponse, FleetSubnetRootDeletionStatusRequest,
            FleetSubnetRootDrainingPublicationRequest, FleetSubnetRootDrainingPublicationResponse,
            FleetSubnetRootEntry, FleetSubnetRootJoinRequest, FleetSubnetRootJoinResponse,
            FleetSubnetRootRemovalPublicationRequest, FleetSubnetRootRemovalPublicationResponse,
            FleetSubnetRootSnapshotAcknowledgement, FleetSubnetRootSnapshotAcknowledgementRequest,
            FleetSubnetRootStatus,
        },
    },
    ids::{
        ComponentDeploymentConfigurationDigest, ComponentTopologyDigest, FleetSubnetRootReleaseSet,
        SubnetId,
    },
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const ROOT_DELETION_READINESS_INTENT_HASH_DOMAIN: &[u8] =
    b"canic.fleet-subnet-root.deletion-readiness-intent.v1";
const ROOT_DELETION_READINESS_HASH_DOMAIN: &[u8] = b"canic.fleet-subnet-root.deletion-readiness.v1";
const ROOT_DELETION_EXECUTION_HASH_DOMAIN: &[u8] = b"canic.fleet-subnet-root.deletion-execution.v1";
const ROOT_DELETION_HASH_DOMAIN: &[u8] = b"canic.fleet-subnet-root.deletion.v1";
const SECONDS_PER_DAY: u128 = 86_400;

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
        args.component_deployment_configuration
            .digest()
            .map_err(|error| InternalError::invalid_input(error.to_string()))?;
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
            registry,
            root_join_receipts: Vec::new(),
            root_snapshot_acknowledgements: Vec::new(),
            registry_activation_receipt: None,
            component_provisioning: None,
            component_group_deployments: Vec::new(),
            component_scale_out: None,
            service_publication_receipt: None,
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
        if current.registry_activation_receipt.is_some() {
            return Err(InternalError::conflict(
                "initial Fleet Registry activation already committed",
            ));
        }

        let current_version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
        )?;
        if request.expected_registry != current_version {
            return Err(InternalError::conflict(
                "Fleet Subnet Root join expected Registry version is stale",
            ));
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
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "Fleet Registry contains a root without its durable join receipt",
            ));
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

    pub(crate) fn snapshot_for_root(
        caller: Principal,
    ) -> Result<FleetRegistrySnapshotResponse, InternalError> {
        let current = Self::current()?;
        require_snapshot_root(&current, caller)?;
        let manifest = FleetRegistryOps::manifest(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
        )?;
        let version = FleetRegistryVersion {
            authority: manifest.authority.clone(),
            revision: manifest.revision,
            content_hash: manifest.content_hash,
        };
        Ok(FleetRegistrySnapshotResponse {
            registry: current.registry,
            manifest,
            version,
        })
    }

    pub(crate) fn acknowledge_root_snapshot(
        caller: Principal,
        request: FleetSubnetRootSnapshotAcknowledgementRequest,
    ) -> Result<FleetSubnetRootSnapshotAcknowledgement, InternalError> {
        let current = Self::current()?;
        require_all_roots_joining(&current)?;
        require_joining_root(&current, caller)?;
        let current_version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
        )?;
        if request.version != current_version {
            return Err(InternalError::conflict(
                "Fleet Subnet Root snapshot acknowledgement is stale",
            ));
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
            return Err(InternalError::conflict(
                "Fleet Subnet Root already acknowledged different Registry authority",
            ));
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
            return Err(InternalError::conflict(
                "Fleet Registry activation already committed against different authority",
            ));
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
            return Err(InternalError::conflict(
                "Fleet Registry activation expected version is stale",
            ));
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
            return Err(InternalError::invalid_input(
                "Fleet Component provisioning operation ID must be nonzero",
            ));
        }
        if planned_at_ns == 0 {
            return Err(InternalError::invalid_input(
                "Fleet Component provisioning planned time must be nonzero",
            ));
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
            return Err(InternalError::conflict(
                "Fleet Component provisioning already contains different protected plan authority",
            ));
        }
        if current.service_publication_receipt.is_some() {
            return Err(InternalError::conflict(
                "Fleet Component provisioning plan must precede Fleet-service publication",
            ));
        }
        let source_registry = initial_active_registry(&current)?;
        if current.registry != source_registry {
            return Err(InternalError::conflict(
                "Fleet Component provisioning plan must precede later Registry transitions",
            ));
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
        if let Some(existing) = &current.component_scale_out {
            if existing.operation_id == request.operation_id && existing.plan == request.plan {
                return component_provisioning_status_response(existing);
            }
            return Err(InternalError::conflict(
                "Fleet Component scale-out already contains different protected plan authority",
            ));
        }
        let fresh = current.component_provisioning.as_ref().ok_or_else(|| {
            InternalError::unavailable(
                "Fleet Component scale-out requires terminal fresh provisioning",
            )
        })?;
        if fresh.operation_id == request.operation_id {
            return Err(InternalError::conflict(
                "Fleet Component scale-out operation ID is already used by fresh provisioning",
            ));
        }
        if !matches!(
            fresh.state,
            FleetComponentProvisioningStateRecord::RuntimesActivated { .. }
        ) {
            return Err(InternalError::unavailable(
                "Fleet Component scale-out requires terminal fresh provisioning",
            ));
        }
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
        let record = provisioning_record_for_status(&current, &request)?;
        component_provisioning_status_response(record)
    }

    pub(crate) fn advance_component_provisioning_root_acceptance(
        request: FleetComponentProvisioningAdvanceRequest,
        started_at_ns: u64,
    ) -> Result<FleetComponentProvisioningRootAcceptanceDisposition, InternalError> {
        let current = Self::current()?;
        let record = require_component_provisioning_operation_record(&current, &request)?;
        let progress = component_provisioning_root_acceptance_progress(record)?;
        match classify_root_acceptance_advance(&request, &progress)? {
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
            return Err(InternalError::invalid_input(
                "Fleet Component root acceptance start time must be nonzero",
            ));
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
        request: FleetComponentProvisioningAdvanceRequest,
        response: RootComponentProvisioningStatusResponse,
        recorded_at_ns: u64,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        let current = Self::current()?;
        let record = require_component_provisioning_operation_record(&current, &request)?;
        let mut progress = component_provisioning_root_acceptance_progress(record)?;
        if progress.accepted_root_count > request.expected_accepted_root_count {
            return replay_recorded_root_acceptance(record, &request, &response, &progress);
        }
        if progress.accepted_root_count != request.expected_accepted_root_count {
            return Err(InternalError::conflict(
                "Fleet Component root acceptance cursor differs from durable progress",
            ));
        }
        let intent = progress.in_flight.ok_or_else(|| {
            InternalError::conflict(
                "Fleet Component root acceptance response has no durable pre-call intent",
            )
        })?;
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
        let accepted_root_count = u32::try_from(progress.acceptances.len()).map_err(|_| {
            InternalError::resource_exhausted("Fleet Component root acceptance count exceeds u32")
        })?;
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
            return Err(InternalError::invalid_input(
                "Fleet Component root provisioning start time must be nonzero",
            ));
        }
        let roots_accepted_at_ns = progress.roots_accepted_at_ns.ok_or_else(|| {
            InternalError::conflict(
                "Fleet Component root provisioning cannot precede complete root acceptance",
            )
        })?;
        let previous_observed_at_ns = root_provision_previous_observed_at(&progress)?;
        if started_at_ns < previous_observed_at_ns {
            return Err(InternalError::invalid_input(
                "Fleet Component root provisioning start time regressed",
            ));
        }
        let response = progress.current_response.as_ref().ok_or_else(|| {
            InternalError::conflict("Fleet Component root provisioning cursor is terminal")
        })?;
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
            return Err(InternalError::conflict(
                "Fleet Component root provisioning response has no exact durable pre-call intent",
            ));
        }
        let intent = progress
            .in_flight
            .take()
            .ok_or_else(|| receipt_invariant("root provisioning response intent disappeared"))?;
        if recorded_at_ns < intent.started_at_ns {
            return Err(InternalError::invalid_input(
                "Fleet Component root provisioning observation time regressed",
            ));
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
        request: FleetComponentProvisioningAdvanceRequest,
        started_at_ns: u64,
    ) -> Result<FleetComponentProvisioningRootAcceptanceDisposition, InternalError> {
        require_test_component_deployment_configuration(config)?;
        Self::advance_component_provisioning_root_acceptance(request, started_at_ns)
    }

    #[cfg(test)]
    pub(crate) fn record_component_provisioning_root_acceptance_for_test(
        config: &ConfigModel,
        request: FleetComponentProvisioningAdvanceRequest,
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
    ) -> Result<(), InternalError> {
        require_test_component_deployment_configuration(config)?;
        let current = Self::current()?;
        require_grouped_root_lifecycle_open(&current)
    }

    pub(crate) fn publish_component_provisioning_services(
        request: &FleetComponentProvisioningAdvanceRequest,
        published_at_ns: u64,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        let current = Self::current()?;
        let record = require_component_provisioning_record(&current, request)?;
        let progress = component_provisioning_root_provision_progress(record)?;
        match classify_root_provision_advance(request, &progress)? {
            RootProvisionAdvance::Current => {
                return component_provisioning_status_response(record);
            }
            RootProvisionAdvance::Publish => {}
            RootProvisionAdvance::Begin | RootProvisionAdvance::Reconcile => {
                return Err(InternalError::conflict(
                    "Fleet-service publication cannot precede complete root provisioning",
                ));
            }
        }
        if published_at_ns == 0 {
            return Err(InternalError::invalid_input(
                "Fleet-service publication time must be nonzero",
            ));
        }
        let provisioned = components_provisioned_state(record)?;
        if published_at_ns < provisioned.components_provisioned_at_ns {
            return Err(InternalError::invalid_input(
                "Fleet-service publication time precedes complete root provisioning",
            ));
        }
        let publication = compile_initial_service_publication(&current, record, &provisioned)?;
        let mut next = current.clone();
        next.registry = publication.registry;
        next.service_publication_receipt = Some(publication.receipt.clone());
        component_provisioning_record_mut(&mut next)?.state =
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
        let record = require_component_provisioning_record(&current, request)?;
        let progress = component_directory_confirmation_progress(record)?;
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
            return Err(InternalError::invalid_input(
                "Directory confirmation start time is invalid",
            ));
        }
        let root_index = progress.confirmed_root_count;
        let previous = progress
            .current
            .as_ref()
            .map(|record| record.response.clone())
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
        let call = FleetComponentDirectoryConfirmationCallView {
            fleet_subnet_root: root,
            request: RootComponentPublicationRequest {
                operation_id: record.operation_id,
                plan_hash: record.plan_hash,
                published_fleet_registry: progress.published_fleet_registry.clone(),
                expected_published_component_count: previous.published_component_count,
            },
        };
        let intent = FleetComponentDirectoryConfirmationIntentRecord {
            root_index,
            fleet_subnet_root: root,
            request: call.request.clone(),
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
        let record = require_component_provisioning_record(&current, request)?;
        let mut progress = component_directory_confirmation_progress(record)?;
        if classify_directory_confirmation_advance(request, &progress)?
            != DirectoryConfirmationAdvance::Reconcile
        {
            return Err(InternalError::conflict(
                "Directory confirmation response has no exact durable pre-call intent",
            ));
        }
        let intent = progress
            .in_flight
            .take()
            .ok_or_else(|| receipt_invariant("Directory confirmation intent disappeared"))?;
        if recorded_at_ns < intent.started_at_ns {
            return Err(InternalError::invalid_input(
                "Directory confirmation observation time regressed",
            ));
        }
        let previous = progress
            .current
            .as_ref()
            .map(|record| &record.response)
            .map_or_else(
                || root_provisioned_response(&progress, intent.root_index),
                Ok,
            )?;
        let fleet_directory_content_hash =
            expected_fleet_directory_content_hash(&current, intent.fleet_subnet_root)?;
        validate_directory_confirmation_response(
            record,
            &progress.published_fleet_registry,
            intent.fleet_subnet_root,
            fleet_directory_content_hash,
            previous,
            &response,
            recorded_at_ns,
        )?;
        let observed = FleetComponentDirectoryConfirmationRecord {
            started_at_ns: intent.started_at_ns,
            response,
            recorded_at_ns,
        };
        if observed.response.phase == RootComponentProvisioningPhase::Published {
            progress.confirmations.push(observed);
            progress.current = None;
        } else {
            progress.current = Some(observed);
        }
        let confirmed_root_count = u32::try_from(progress.confirmations.len())
            .map_err(|_| receipt_invariant("Directory confirmation count does not fit u32"))?;
        let mut next = current.clone();
        component_provisioning_record_mut(&mut next)?.state =
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
        let next = Self::validate_current(next)?;
        let result = component_provisioning_status_response(component_provisioning_record(&next)?)?;
        Self::commit_transition(&current, next)?;
        Ok(result)
    }

    pub(crate) fn advance_component_runtime_activation(
        request: &FleetComponentProvisioningAdvanceRequest,
        started_at_ns: u64,
    ) -> Result<FleetComponentRuntimeActivationDisposition, InternalError> {
        let current = Self::current()?;
        let record = require_component_provisioning_record(&current, request)?;
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
            return Err(InternalError::invalid_input(
                "runtime activation start time is invalid",
            ));
        }
        let root_index = progress.activated_root_count;
        let publication = root_publication_response(&progress, root_index)?;
        let current_progress = progress.current.map_or_else(
            || root_activation_progress(publication),
            |record| record.progress,
        );
        let root = confirmation_root(record, root_index)?;
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
        component_provisioning_record_mut(&mut next)?.state =
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
        let record = require_component_provisioning_record(&current, request)?;
        let mut progress = component_runtime_activation_progress(record)?;
        if classify_runtime_activation_advance(request, &progress)?
            != RuntimeActivationAdvance::Reconcile
        {
            return Err(InternalError::conflict(
                "runtime activation response has no exact durable pre-call intent",
            ));
        }
        let intent = progress
            .in_flight
            .take()
            .ok_or_else(|| receipt_invariant("runtime activation intent disappeared"))?;
        if recorded_at_ns < intent.started_at_ns {
            return Err(InternalError::invalid_input(
                "runtime activation observation time regressed",
            ));
        }
        let publication = root_publication_response(&progress, intent.root_index)?;
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
        component_provisioning_record_mut(&mut next)?.state = if runtimes_are_terminal {
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
        };
        if runtimes_are_terminal {
            next.component_group_deployments = deployment_ledger::compile_initial(
                &next.component_deployment_configuration,
                component_provisioning_record(&next)?,
            )?;
        }
        let next = Self::validate_current(next)?;
        let result = component_provisioning_status_response(component_provisioning_record(&next)?)?;
        Self::commit_transition(&current, next)?;
        Ok(result)
    }

    pub(crate) fn publish_root_draining(
        request: FleetSubnetRootDrainingPublicationRequest,
    ) -> Result<FleetSubnetRootDrainingPublicationResponse, InternalError> {
        let current = Self::current()?;
        require_grouped_root_lifecycle_open(&current)?;
        if let Some(receipt) = current
            .root_draining_publication_receipts
            .iter()
            .find(|receipt| draining_publication_identity_matches(receipt, &request))
        {
            if receipt.request == request {
                return Ok(receipt.response.clone());
            }
            return Err(InternalError::conflict(
                "Fleet Subnet Root draining publication identity already has different authority",
            ));
        }
        if current.registry_activation_receipt.is_none() {
            return Err(InternalError::conflict(
                "Fleet Subnet Root draining publication requires an active Fleet Registry",
            ));
        }
        let previous_version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
        )?;
        if request.expected_registry != previous_version {
            return Err(InternalError::conflict(
                "Fleet Subnet Root draining publication expected Registry version is stale",
            ));
        }
        validate_draining_publication_request(&current.registry, &previous_version, &request)
            .map_err(InternalError::invalid_input)?;

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
            return Err(InternalError::forbidden(
                "Fleet Subnet Root removal publication caller differs from its terminal inventory",
            ));
        }
        let current = Self::current()?;
        require_grouped_root_lifecycle_open(&current)?;
        if let Some(receipt) = current
            .root_removal_publication_receipts
            .iter()
            .find(|receipt| removal_publication_identity_matches(receipt, &request))
        {
            if receipt.request == request {
                return Ok(receipt.response.clone());
            }
            return Err(InternalError::conflict(
                "Fleet Subnet Root removal publication identity already has different authority",
            ));
        }
        let previous_version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
        )?;
        if request.expected_registry != previous_version {
            return Err(InternalError::conflict(
                "Fleet Subnet Root removal publication expected Registry version is stale",
            ));
        }
        let history = canonical_registry_lifecycle_history(&current)?;
        validate_removal_publication_request(
            &current.registry,
            &previous_version,
            &current.root_draining_publication_receipts,
            &history,
            &request,
        )
        .map_err(InternalError::invalid_input)?;

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

    pub(crate) fn prepare_root_deletion_readiness(
        caller: Principal,
        coordinator: Principal,
        request: FleetSubnetRootDeletionReadinessIntentRequest,
        recorded_at_ns: u64,
    ) -> Result<FleetSubnetRootDeletionReadinessIntentResponse, InternalError> {
        require_root_deletion_caller(caller, request.fleet_subnet_root)?;
        let current = Self::current()?;
        require_coordinator_identity(&current, coordinator)?;
        if let Some(existing) = find_root_deletion_readiness_intent(
            &current,
            request.operation_id,
            request.fleet_subnet_root,
        )? {
            if existing.request == request {
                return Ok(existing.clone());
            }
            return Err(InternalError::conflict(
                "Fleet Subnet Root deletion-readiness intent already has different authority",
            ));
        }
        let removal = require_removed_root_publication(
            &current,
            request.operation_id,
            request.fleet_subnet_root,
        )?;
        let expected_target = root_deletion_retained_cycles_target(
            request.observed_idle_cycles_burned_per_day,
            request.observed_freezing_threshold_seconds,
        )?;
        let request_is_valid = [
            request.final_inventory_hash == removal.response.final_inventory.inventory_hash,
            request.store_deletion_hash != [0; 32],
            request.observed_cycles_before_reclamation > 0,
            request.retained_cycles_target > 0,
            request.retained_cycles_target == expected_target,
            request.observed_reserved_cycles == 0,
            request.prepared_at_ns >= removal.response.final_inventory.finalized_at_ns,
            recorded_at_ns >= request.prepared_at_ns,
        ]
        .into_iter()
        .all(|valid| valid);
        if !request_is_valid {
            return Err(InternalError::invalid_input(
                "Fleet Subnet Root deletion-readiness intent is incomplete",
            ));
        }
        let mut response = FleetSubnetRootDeletionReadinessIntentResponse {
            request,
            coordinator,
            recorded_at_ns,
            intent_hash: [0; 32],
        };
        response.intent_hash = response_hash(
            ROOT_DELETION_READINESS_INTENT_HASH_DOMAIN,
            &response,
            "root deletion-readiness intent",
        )?;
        let mut next = current.clone();
        next.root_deletion_readiness_intents.push(response.clone());
        sort_root_deletion_records(&mut next.root_deletion_readiness_intents, |receipt| {
            receipt.request.fleet_subnet_root
        });
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(response)
    }

    pub(crate) fn record_root_deletion_readiness(
        caller: Principal,
        coordinator: Principal,
        request: FleetSubnetRootDeletionReadinessRequest,
        recorded_at_ns: u64,
    ) -> Result<FleetSubnetRootDeletionReadinessResponse, InternalError> {
        require_root_deletion_caller(caller, request.fleet_subnet_root)?;
        let current = Self::current()?;
        require_coordinator_identity(&current, coordinator)?;
        if let Some(existing) =
            find_root_deletion_readiness(&current, request.operation_id, request.fleet_subnet_root)?
        {
            if existing.request == request {
                return Ok(existing.clone());
            }
            return Err(InternalError::conflict(
                "Fleet Subnet Root deletion readiness already has different authority",
            ));
        }
        let intent = find_root_deletion_readiness_intent(
            &current,
            request.operation_id,
            request.fleet_subnet_root,
        )?
        .ok_or_else(|| {
            InternalError::unavailable(
                "Fleet Subnet Root deletion-readiness intent has not been prepared",
            )
        })?;
        let request_is_valid = [
            request.expected_intent_hash == intent.intent_hash,
            request.observed_cycles_after_reclamation
                <= intent.request.observed_cycles_before_reclamation,
            request.observed_cycles_after_reclamation <= intent.request.retained_cycles_target,
            request.cycles_reclaimed_at_ns >= intent.request.prepared_at_ns,
            recorded_at_ns >= request.cycles_reclaimed_at_ns,
        ]
        .into_iter()
        .all(|valid| valid);
        if !request_is_valid {
            return Err(InternalError::conflict(
                "Fleet Subnet Root deletion readiness differs from durable intent",
            ));
        }
        let mut response = FleetSubnetRootDeletionReadinessResponse {
            request,
            coordinator,
            final_inventory_hash: intent.request.final_inventory_hash,
            store_deletion_hash: intent.request.store_deletion_hash,
            observed_cycles_before_reclamation: intent.request.observed_cycles_before_reclamation,
            retained_cycles_target: intent.request.retained_cycles_target,
            observed_reserved_cycles: intent.request.observed_reserved_cycles,
            observed_idle_cycles_burned_per_day: intent.request.observed_idle_cycles_burned_per_day,
            observed_freezing_threshold_seconds: intent.request.observed_freezing_threshold_seconds,
            prepared_at_ns: intent.request.prepared_at_ns,
            recorded_at_ns,
            readiness_hash: [0; 32],
        };
        response.readiness_hash = response_hash(
            ROOT_DELETION_READINESS_HASH_DOMAIN,
            &response,
            "root deletion readiness",
        )?;
        let mut next = current.clone();
        next.root_deletion_readiness_receipts.push(response.clone());
        sort_root_deletion_records(&mut next.root_deletion_readiness_receipts, |receipt| {
            receipt.request.fleet_subnet_root
        });
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(response)
    }

    pub(crate) fn begin_root_deletion_execution(
        executor: Principal,
        coordinator: Principal,
        request: FleetSubnetRootDeletionExecutionRequest,
        prepared_at_ns: u64,
    ) -> Result<FleetSubnetRootDeletionExecutionResponse, InternalError> {
        let current = Self::current()?;
        require_coordinator_identity(&current, coordinator)?;
        if let Some(existing) =
            find_root_deletion_execution(&current, request.operation_id, request.fleet_subnet_root)?
        {
            if existing.executor == executor && existing.request == request {
                return Ok(existing.clone());
            }
            return Err(InternalError::conflict(
                "Fleet Subnet Root deletion execution already has different authority",
            ));
        }
        let readiness = find_root_deletion_readiness(
            &current,
            request.operation_id,
            request.fleet_subnet_root,
        )?
        .ok_or_else(|| {
            InternalError::unavailable("Fleet Subnet Root deletion readiness is not complete")
        })?;
        validate_root_deletion_execution_request(executor, &request, readiness, prepared_at_ns)?;
        let mut response = FleetSubnetRootDeletionExecutionResponse {
            request,
            executor,
            prepared_at_ns,
            execution_hash: [0; 32],
        };
        response.execution_hash = response_hash(
            ROOT_DELETION_EXECUTION_HASH_DOMAIN,
            &response,
            "root deletion execution",
        )?;
        let mut next = current.clone();
        next.root_deletion_execution_intents.push(response.clone());
        sort_root_deletion_records(&mut next.root_deletion_execution_intents, |receipt| {
            receipt.request.fleet_subnet_root
        });
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(response)
    }

    pub(crate) fn root_deletion_execution_status(
        request: FleetSubnetRootDeletionStatusRequest,
    ) -> Result<FleetSubnetRootDeletionExecutionResponse, InternalError> {
        let current = Self::current()?;
        find_root_deletion_execution(&current, request.operation_id, request.fleet_subnet_root)?
            .cloned()
            .ok_or_else(|| InternalError::unavailable("Fleet Subnet Root deletion has not begun"))
    }

    pub(crate) fn complete_root_deletion(
        executor: Principal,
        coordinator: Principal,
        request: FleetSubnetRootDeletionCompletionRequest,
        completed_at_ns: u64,
    ) -> Result<FleetSubnetRootDeletionResponse, InternalError> {
        let current = Self::current()?;
        require_coordinator_identity(&current, coordinator)?;
        if let Some(existing) =
            find_root_deletion(&current, request.operation_id, request.fleet_subnet_root)?
        {
            let retry_is_exact = [
                existing.executor == executor,
                existing.execution_hash == request.expected_execution_hash,
                existing.observed_absent_at_ns == request.observed_absent_at_ns,
            ]
            .into_iter()
            .all(|valid| valid);
            if retry_is_exact {
                return Ok(existing.clone());
            }
            return Err(InternalError::conflict(
                "Fleet Subnet Root deletion completion already has different authority",
            ));
        }
        let execution = find_root_deletion_execution(
            &current,
            request.operation_id,
            request.fleet_subnet_root,
        )?
        .ok_or_else(|| {
            InternalError::unavailable("Fleet Subnet Root deletion execution has not begun")
        })?;
        let completion_is_exact = [
            execution.executor == executor,
            execution.execution_hash == request.expected_execution_hash,
            request.observed_absent_at_ns >= execution.prepared_at_ns,
            completed_at_ns >= request.observed_absent_at_ns,
        ]
        .into_iter()
        .all(|valid| valid);
        if !completion_is_exact {
            return Err(InternalError::conflict(
                "Fleet Subnet Root deletion completion differs from execution intent",
            ));
        }
        let mut response = FleetSubnetRootDeletionResponse {
            operation_id: request.operation_id,
            fleet_subnet_root: request.fleet_subnet_root,
            coordinator,
            executor,
            readiness_hash: execution.request.expected_readiness_hash,
            execution_hash: execution.execution_hash,
            observed_module_hash: execution.request.observed_module_hash,
            observed_controllers: execution.request.observed_controllers.clone(),
            observed_cycles_after_reclamation: execution.request.observed_cycles_after_reclamation,
            observed_absent_at_ns: request.observed_absent_at_ns,
            completed_at_ns,
            deletion_hash: [0; 32],
        };
        response.deletion_hash =
            response_hash(ROOT_DELETION_HASH_DOMAIN, &response, "root deletion")?;
        let mut next = current.clone();
        next.root_deletion_receipts.push(response.clone());
        sort_root_deletion_records(&mut next.root_deletion_receipts, |receipt| {
            receipt.fleet_subnet_root
        });
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(response)
    }

    pub(crate) fn root_deletion_status(
        request: FleetSubnetRootDeletionStatusRequest,
    ) -> Result<FleetSubnetRootDeletionResponse, InternalError> {
        let current = Self::current()?;
        find_root_deletion(&current, request.operation_id, request.fleet_subnet_root)?
            .cloned()
            .ok_or_else(|| InternalError::unavailable("Fleet Subnet Root deletion is not complete"))
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
            .ok_or_else(|| {
                InternalError::unavailable("Fleet Coordinator genesis is not initialized")
            })?;
        Self::validate_current(current)
    }

    fn validate_current(
        current: FleetCoordinatorRegistryRecord,
    ) -> Result<FleetCoordinatorRegistryRecord, InternalError> {
        let current = Self::validate_current_registry(current)?;
        validate_component_provisioning_record(&current)?;
        validate_component_scale_out_progress(&current)?;
        deployment_ledger::validate(
            &current.component_deployment_configuration,
            &current.registry,
            current.component_provisioning.as_ref(),
            current.component_scale_out.as_ref(),
            &current.component_group_deployments,
        )?;
        Ok(current)
    }

    fn validate_current_registry(
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
        validate_root_join_receipts(&current)?;
        validate_root_snapshot_acknowledgements(&current)?;
        validate_registry_lifecycle_history(&current)?;
        validate_root_deletion_history(&current)?;
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

#[cfg(test)]
fn require_test_component_deployment_configuration(
    config: &ConfigModel,
) -> Result<(), InternalError> {
    let expected = config
        .compile_component_deployment_configuration()
        .map_err(|error| InternalError::invalid_input(error.to_string()))?;
    let current = FleetCoordinatorRegistryStore::export()
        .current
        .ok_or_else(|| {
            InternalError::unavailable("Fleet Coordinator genesis is not initialized")
        })?;
    if current.component_deployment_configuration != expected {
        return Err(InternalError::conflict(
            "test Component deployment configuration differs from durable Coordinator authority",
        ));
    }
    Ok(())
}

fn validate_component_provisioning_record(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    let Some(record) = &current.component_provisioning else {
        if current.service_publication_receipt.is_some() {
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
    let source_registry = initial_active_registry(current)?;
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
    ) {
        return Err(receipt_invariant(
            "Fleet Component scale-out crossed its implemented Store-backed installation boundary",
        ));
    }
    validate_component_provisioning_root_acceptance_state(record)?;
    validate_component_provisioning_root_provision_state(
        &current.component_deployment_configuration,
        &current.registry,
        record,
    )?;
    validate_scale_out_installation_fence(record)?;
    component_provisioning_plan_counts(&record.plan)?;
    Ok(())
}

fn validate_scale_out_installation_fence(
    record: &FleetComponentProvisioningRecord,
) -> Result<(), InternalError> {
    let progress = component_provisioning_root_provision_progress(record)?;
    let crossed_later_boundary = [
        progress.provisioned_root_count != 0,
        progress.components_provisioned_at_ns.is_some(),
        progress.published_fleet_registry.is_some(),
        progress.service_topology_published_at_ns.is_some(),
    ]
    .into_iter()
    .any(|crossed| crossed);
    if crossed_later_boundary {
        return Err(receipt_invariant(
            "Fleet Component scale-out contains evidence beyond Store-backed installation",
        ));
    }
    if let Some(response) = &progress.current_response {
        let counts = RootProvisioningCounts::from_response(response);
        if response.phase != RootComponentProvisioningPhase::Accepted
            || !counts.is_store_install_boundary(response.component_count)
        {
            return Err(receipt_invariant(
                "Fleet Component scale-out root progress crossed the Store-backed installation fence",
            ));
        }
    }
    if let Some(intent) = &progress.in_flight {
        let counts = RootProvisioningCounts::from_request(intent.request);
        let component_count = progress
            .current_response
            .as_ref()
            .map(|response| response.component_count)
            .ok_or_else(|| receipt_invariant("scale-out install intent lacks root progress"))?;
        if !counts.is_store_install_boundary(component_count)
            || counts.store_installation_is_complete(component_count)
        {
            return Err(receipt_invariant(
                "Fleet Component scale-out intent crossed the Store-backed installation fence",
            ));
        }
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
    if progress.confirmation_root_count
        != u32::try_from(record.plan.batches.len())
            .map_err(|_| receipt_invariant("root batch count does not fit u32"))?
        || progress.confirmed_root_count > progress.confirmation_root_count
    {
        return Err(receipt_invariant(
            "fresh Directory confirmation roots differ from selected root batches",
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
    let publication = root_publication_response(progress, root_index)?;
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
    let evidence_is_exact = [
        activation.component_count == publication.component_count,
        activation.fleet_activation_operation_id != [0; 32],
        activation.initial_inventory_hash != [0; 32],
        activation.root_activated_at_ns == runtimes_activated_at_ns,
        runtimes_activated_at_ns >= activation_started_at_ns,
        stored.recorded_at_ns >= runtimes_activated_at_ns,
    ]
    .into_iter()
    .all(|matches| matches);
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
    let root = confirmation_root(record, progress.activated_root_count)?;
    let current = progress.current.map_or_else(
        || {
            root_publication_response(progress, progress.activated_root_count)
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
        let previous = root_provisioned_response(progress, root_index)?;
        let fleet_directory_content_hash =
            expected_fleet_directory_content_hash(coordinator, root)?;
        validate_directory_confirmation_response(
            record,
            &progress.published_fleet_registry,
            root,
            fleet_directory_content_hash,
            previous,
            &confirmation.response,
            confirmation.recorded_at_ns,
        )
        .map_err(|_| receipt_invariant("stored Directory confirmation receipt is invalid"))?;
        if confirmation.response.phase != RootComponentProvisioningPhase::Published
            || confirmation.started_at_ns < previous_recorded_at_ns
            || confirmation.recorded_at_ns < confirmation.started_at_ns
        {
            return Err(receipt_invariant(
                "stored Directory confirmation time or terminal phase is invalid",
            ));
        }
        previous_recorded_at_ns = confirmation.recorded_at_ns;
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
        let previous = root_provisioned_response(progress, progress.confirmed_root_count)?;
        let fleet_directory_content_hash =
            expected_fleet_directory_content_hash(coordinator, root)?;
        validate_directory_confirmation_response(
            record,
            &progress.published_fleet_registry,
            root,
            fleet_directory_content_hash,
            previous,
            &current.response,
            current.recorded_at_ns,
        )
        .map_err(|_| receipt_invariant("stored in-progress Directory confirmation is invalid"))?;
        if current.response.phase != RootComponentProvisioningPhase::Provisioned
            || current.started_at_ns < previous_recorded_at_ns
            || current.recorded_at_ns < current.started_at_ns
        {
            return Err(receipt_invariant(
                "in-progress Directory confirmation time or phase is invalid",
            ));
        }
        previous_recorded_at_ns = current.recorded_at_ns;
    }
    Ok(previous_recorded_at_ns)
}

fn validate_directory_confirmation_intent(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentDirectoryConfirmationProgress,
    previous_recorded_at_ns: u64,
) -> Result<(), InternalError> {
    if let Some(intent) = &progress.in_flight {
        let root = confirmation_root(record, progress.confirmed_root_count)?;
        let previous = progress
            .current
            .as_ref()
            .map(|current| &current.response)
            .map_or_else(
                || root_provisioned_response(progress, progress.confirmed_root_count),
                Ok,
            )?;
        let intent_is_exact = [
            intent.root_index == progress.confirmed_root_count,
            intent.fleet_subnet_root == root,
            intent.request.operation_id == record.operation_id,
            intent.request.plan_hash == record.plan_hash,
            intent.request.published_fleet_registry == progress.published_fleet_registry,
            intent.request.expected_published_component_count == previous.published_component_count,
            intent.started_at_ns >= previous_recorded_at_ns,
        ]
        .into_iter()
        .all(|matches| matches);
        if !intent_is_exact {
            return Err(receipt_invariant(
                "Directory confirmation pre-call intent is invalid",
            ));
        }
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
        current_publication: directory
            .as_ref()
            .and_then(|progress| progress.current.as_ref())
            .map(|record| root_publication_progress(&record.response)),
        publication_in_flight_root: directory
            .as_ref()
            .and_then(|progress| progress.in_flight.as_ref())
            .map(|intent| intent.fleet_subnet_root),
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
    let source_registry = initial_active_registry(current)?;
    let root_receipts = publication
        .provisions
        .iter()
        .map(|provision| provision.response.clone())
        .collect::<Vec<_>>();
    let receipt_hashes = root_receipts
        .iter()
        .map(|root_receipt| root_receipt.receipt_content_hash)
        .collect::<Vec<_>>();
    let services = FleetServiceBindingOps::compile_initial_compiled(
        &current.component_deployment_configuration,
        &source_registry,
        &record.plan,
        record.operation_id,
        &root_receipts,
    )
    .map_err(|_| {
        receipt_invariant("published root provisioning receipts do not compile canonical services")
    })?;
    if receipt.previous_version != record.plan.fleet_registry
        || receipt.version != *publication.published_registry
        || receipt.root_receipt_content_hashes != receipt_hashes
        || receipt.services != services
    {
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
    match (
        service_publication_state(record),
        current.service_publication_receipt.as_ref(),
    ) {
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

fn require_component_provisioning_record<'a>(
    current: &'a FleetCoordinatorRegistryRecord,
    request: &FleetComponentProvisioningAdvanceRequest,
) -> Result<&'a FleetComponentProvisioningRecord, InternalError> {
    let record = component_provisioning_record(current).map_err(|_| {
        InternalError::unavailable("Fleet Component provisioning plan is not prepared")
    })?;
    if record.operation_id != request.operation_id {
        return Err(InternalError::conflict(
            "Fleet Component provisioning advance names different protected plan authority",
        ));
    }
    if record.plan_hash != request.plan_hash {
        return Err(InternalError::conflict(
            "Fleet Component provisioning advance names different protected plan authority",
        ));
    }
    Ok(record)
}

fn require_component_provisioning_operation_record<'a>(
    current: &'a FleetCoordinatorRegistryRecord,
    request: &FleetComponentProvisioningAdvanceRequest,
) -> Result<&'a FleetComponentProvisioningRecord, InternalError> {
    provisioning_record_for_status(
        current,
        &FleetComponentProvisioningStatusRequest {
            operation_id: request.operation_id,
            plan_hash: request.plan_hash,
        },
    )
}

fn provisioning_record_for_status<'a>(
    current: &'a FleetCoordinatorRegistryRecord,
    request: &FleetComponentProvisioningStatusRequest,
) -> Result<&'a FleetComponentProvisioningRecord, InternalError> {
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
            return Ok(record);
        }
        return Err(InternalError::conflict(
            "Fleet Component provisioning status names a reused operation with a different plan hash",
        ));
    }
    Err(InternalError::unavailable(
        "Fleet Component provisioning operation is not prepared",
    ))
}

fn component_provisioning_record(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<&FleetComponentProvisioningRecord, InternalError> {
    current
        .component_provisioning
        .as_ref()
        .ok_or_else(|| receipt_invariant("Fleet Component provisioning record disappeared"))
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

struct InitialServicePublication {
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

fn compile_initial_service_publication(
    current: &FleetCoordinatorRegistryRecord,
    record: &FleetComponentProvisioningRecord,
    provisioned: &ComponentsProvisionedState,
) -> Result<InitialServicePublication, InternalError> {
    let source_registry = initial_active_registry(current)?;
    if current.registry != source_registry {
        return Err(InternalError::conflict(
            "initial Fleet-service publication must precede later Registry transitions",
        ));
    }
    if current.service_publication_receipt.is_some() {
        return Err(receipt_invariant(
            "ComponentsProvisioned state already contains Fleet-service publication evidence",
        ));
    }
    let root_receipts = provisioned
        .provisions
        .iter()
        .map(|provision| provision.response.clone())
        .collect::<Vec<_>>();
    let services = FleetServiceBindingOps::compile_initial_compiled(
        &current.component_deployment_configuration,
        &source_registry,
        &record.plan,
        record.operation_id,
        &root_receipts,
    )?;
    let topology = &current
        .component_deployment_configuration
        .component_topology;
    let previous_version =
        FleetRegistryOps::version(&current.authority, topology, &current.registry)?;
    if previous_version != record.plan.fleet_registry {
        return Err(InternalError::conflict(
            "initial Fleet-service publication expected Registry version is stale",
        ));
    }
    let registry = if services.is_empty() {
        current.registry.clone()
    } else {
        FleetRegistryOps::compile_initial_services(
            &current.authority,
            topology,
            &current.registry,
            services.clone(),
        )?
    };
    let version = FleetRegistryOps::version(&current.authority, topology, &registry)?;
    let root_receipt_content_hashes = root_receipts
        .iter()
        .map(|receipt| receipt.receipt_content_hash)
        .collect();
    Ok(InitialServicePublication {
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
            return Err(InternalError::conflict(
                "Directory confirmation requires published Fleet-service topology",
            ));
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
            return Err(InternalError::conflict(
                "runtime activation requires confirmed Directories",
            ));
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
            Err(InternalError::conflict(
                "runtime activation cursor differs from terminal progress",
            ))
        };
    }
    if request.expected_runtime_activated_root_count < progress.activated_root_count {
        return if terminal_runtime_activation_replay(request, progress)? {
            Ok(RuntimeActivationAdvance::Current)
        } else {
            Err(InternalError::conflict(
                "runtime-activated root cursor differs from durable progress",
            ))
        };
    }
    if request.expected_runtime_activated_root_count != progress.activated_root_count {
        return Err(InternalError::conflict(
            "runtime-activated root cursor differs from durable progress",
        ));
    }
    let actual = progress.current.map(|record| record.progress);
    if request.expected_current_activation != actual {
        let replays_last = request
            .expected_current_activation
            .zip(actual)
            .is_some_and(|(expected, actual)| activation_progress_advances(expected, actual));
        return if replays_last {
            Ok(RuntimeActivationAdvance::Current)
        } else {
            Err(InternalError::conflict(
                "runtime activation Component cursor differs from durable progress",
            ))
        };
    }
    if progress.in_flight.is_some() {
        Ok(RuntimeActivationAdvance::Reconcile)
    } else {
        Ok(RuntimeActivationAdvance::Begin)
    }
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
            && request.expected_current_publication.is_none();
        let replays_terminal_call = terminal_directory_confirmation_replay(request, progress)?;
        return if current_is_exact || replays_terminal_call {
            Ok(DirectoryConfirmationAdvance::Current)
        } else {
            Err(InternalError::conflict(
                "Directory confirmation cursor differs from terminal progress",
            ))
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
            Err(InternalError::conflict(
                "Directory confirmation root cursor differs from durable progress",
            ))
        };
    }
    if request.expected_directory_confirmed_root_count != progress.confirmed_root_count {
        return Err(InternalError::conflict(
            "Directory confirmation root cursor differs from durable progress",
        ));
    }
    let actual_current = progress
        .current
        .as_ref()
        .map(|record| root_publication_progress(&record.response));
    if request.expected_current_publication != actual_current {
        let replays_last = match (&request.expected_current_publication, &actual_current) {
            (Some(expected), Some(actual)) => {
                expected.fleet_subnet_root == actual.fleet_subnet_root
                    && expected.component_count == actual.component_count
                    && expected.published_component_count.checked_add(1)
                        == Some(actual.published_component_count)
            }
            _ => false,
        };
        if replays_last {
            return Ok(DirectoryConfirmationAdvance::Current);
        }
        return Err(InternalError::conflict(
            "Directory confirmation Component cursor differs from durable progress",
        ));
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
    let terminal_progress = root_publication_progress(&terminal.response);
    Ok(request
        .expected_current_publication
        .as_ref()
        .map_or(terminal_progress.component_count == 0, |expected| {
            expected == &terminal_progress
        }))
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

    const fn from_request(request: RootComponentProvisioningAdvanceRequest) -> Self {
        Self {
            reserved: request.expected_reserved_component_count,
            claimed: request.expected_claimed_component_count,
            installed: request.expected_installed_component_count,
            registry_committed: request.expected_registry_committed_component_count,
        }
    }

    fn is_store_install_boundary(self, component_count: u32) -> bool {
        let claim_has_reserved_identity = match self.claimed {
            0 => self.reserved <= component_count,
            _ => self.reserved == component_count,
        };
        let install_has_claimed_canister = match self.installed {
            0 => self.claimed <= component_count,
            _ => self.claimed == component_count,
        };
        [
            claim_has_reserved_identity,
            install_has_claimed_canister,
            self.claimed <= component_count,
            self.installed <= component_count,
            self.registry_committed == 0,
        ]
        .into_iter()
        .all(|matches| matches)
    }

    const fn store_installation_is_complete(self, component_count: u32) -> bool {
        self.installed == component_count
    }

    const fn is_terminal(self, component_count: u32) -> bool {
        self.reserved == component_count
            && self.claimed == component_count
            && self.installed == component_count
            && self.registry_committed == component_count
    }

    const fn is_canonical(self, component_count: u32) -> bool {
        self.reserved <= component_count
            && self.claimed <= component_count
            && self.installed <= component_count
            && self.registry_committed <= component_count
            && (self.claimed == 0 || self.reserved == component_count)
            && (self.installed == 0 || self.claimed == component_count)
            && (self.registry_committed == 0 || self.installed == component_count)
    }

    fn advances_one_step_to(self, next: Self, component_count: u32) -> bool {
        if !self.is_canonical(component_count) || !next.is_canonical(component_count) {
            return false;
        }
        let reservation = self.claimed == 0
            && self.installed == 0
            && self.registry_committed == 0
            && next.claimed == 0
            && next.installed == 0
            && next.registry_committed == 0
            && self.reserved.checked_add(1) == Some(next.reserved);
        let claim = self.reserved == next.reserved
            && self.installed == 0
            && self.registry_committed == 0
            && next.installed == 0
            && next.registry_committed == 0
            && self.claimed.checked_add(1) == Some(next.claimed);
        let install = self.reserved == next.reserved
            && self.claimed == next.claimed
            && self.registry_committed == 0
            && next.registry_committed == 0
            && self.installed.checked_add(1) == Some(next.installed);
        let registry = self.reserved == next.reserved
            && self.claimed == next.claimed
            && self.installed == next.installed
            && self.registry_committed.checked_add(1) == Some(next.registry_committed);
        reservation || claim || install || registry
    }
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
            return Err(InternalError::conflict(
                "Fleet Component root provisioning expected cursor differs from durable progress",
            ));
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
        return Err(InternalError::conflict(
            "Fleet Component root provisioning expected cursor differs from durable progress",
        ));
    }
    if request.expected_provisioned_root_count.checked_add(1)
        == Some(progress.provisioned_root_count)
    {
        let index = usize::try_from(request.expected_provisioned_root_count).map_err(|_| {
            InternalError::resource_exhausted("provisioned root index does not fit usize")
        })?;
        let provision = progress.provisions.get(index).ok_or_else(|| {
            receipt_invariant("terminal root provisioning receipt is absent at its cursor")
        })?;
        if request.expected_current_root.as_ref()
            == Some(&root_provisioning_progress(&provision.response))
        {
            return Ok(RootProvisionAdvance::Current);
        }
    }
    Err(InternalError::conflict(
        "Fleet Component provisioned root count differs from durable progress",
    ))
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
    FleetComponentDirectoryConfirmationCallView {
        fleet_subnet_root: intent.fleet_subnet_root,
        request: intent.request.clone(),
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

fn root_publication_response(
    progress: &FleetComponentRuntimeActivationProgress,
    root_index: u32,
) -> Result<&RootComponentProvisioningStatusResponse, InternalError> {
    let index = usize::try_from(root_index)
        .map_err(|_| receipt_invariant("runtime activation root index exceeds usize"))?;
    progress
        .confirmations
        .get(index)
        .map(|record| &record.response)
        .ok_or_else(|| receipt_invariant("runtime activation lacks root publication evidence"))
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
        return Err(InternalError::conflict(
            "runtime activation response did not advance exactly one bounded cursor",
        ));
    }
    let activation_started_at_ns = response.activation_started_at_ns.ok_or_else(|| {
        InternalError::conflict("runtime activation response lacks its durable start time")
    })?;
    if previous_activation_started_at_ns
        .is_some_and(|expected| expected != activation_started_at_ns)
    {
        return Err(InternalError::conflict(
            "runtime activation response changed its durable start time",
        ));
    }
    let published_at_ns = response.published_at_ns.ok_or_else(|| {
        receipt_invariant("runtime activation publication lacks its completion time")
    })?;
    if activation_started_at_ns < published_at_ns || recorded_at_ns < activation_started_at_ns {
        return Err(InternalError::conflict(
            "runtime activation response has invalid time evidence",
        ));
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
                return Err(InternalError::conflict(
                    "in-progress runtime activation changed terminal publication authority",
                ));
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
            return Err(InternalError::conflict(
                "runtime activation response has an invalid root phase",
            ));
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
        return Err(InternalError::conflict(
            "runtime activation response changed protected publication authority",
        ));
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
    let activation = response.activation.ok_or_else(|| {
        InternalError::conflict("terminal runtime activation lacks activation evidence")
    })?;
    let runtimes_activated_at_ns = response.runtimes_activated_at_ns.ok_or_else(|| {
        InternalError::conflict("terminal runtime activation lacks completion time")
    })?;
    let evidence_is_exact = [
        response.root_runtime_active,
        response.activated_component_count == response.component_count,
        activation.component_count == response.component_count,
        activation.fleet_activation_operation_id != [0; 32],
        activation.initial_inventory_hash != [0; 32],
        activation.root_activated_at_ns == runtimes_activated_at_ns,
        runtimes_activated_at_ns >= activation_started_at_ns,
        recorded_at_ns >= runtimes_activated_at_ns,
    ]
    .into_iter()
    .all(|matches| matches);
    if !evidence_is_exact {
        return Err(InternalError::conflict(
            "terminal runtime activation evidence is invalid",
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
    if response.receipt_content_hash != expected {
        return Err(InternalError::conflict(
            "terminal runtime activation receipt hash is invalid",
        ));
    }
    Ok(())
}

fn expected_fleet_directory_content_hash(
    current: &FleetCoordinatorRegistryRecord,
    root: Principal,
) -> Result<[u8; 32], InternalError> {
    let directory = FleetRegistryOps::directory_for_root(
        &current.registry.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &current.registry,
        root,
    )?;
    RootComponentProvisioningReceiptOps::fleet_directory_content_hash(&directory)
}

fn validate_directory_confirmation_response(
    record: &FleetComponentProvisioningRecord,
    published_registry: &FleetRegistryVersion,
    root: Principal,
    expected_fleet_directory_content_hash: [u8; 32],
    previous: &RootComponentProvisioningStatusResponse,
    response: &RootComponentProvisioningStatusResponse,
    recorded_at_ns: u64,
) -> Result<(), InternalError> {
    let batch = record
        .plan
        .batches
        .iter()
        .find(|batch| batch.root.fleet_subnet_root == root)
        .ok_or_else(|| receipt_invariant("Directory confirmation root has no planned batch"))?;
    let expected_authority = RootDirectoryConfirmationAuthority::expected(record, root, previous);
    if RootDirectoryConfirmationAuthority::observed(response) != expected_authority {
        return Err(InternalError::conflict(
            "Directory confirmation response changed protected provisioning authority",
        ));
    }
    let count_advances = response.published_component_count == previous.published_component_count
        || previous.published_component_count.checked_add(1)
            == Some(response.published_component_count);
    if !count_advances || response.published_component_count > response.component_count {
        return Err(InternalError::conflict(
            "Directory confirmation response skipped its bounded Component cursor",
        ));
    }
    let publication = response.publication.as_ref().ok_or_else(|| {
        InternalError::conflict("Directory confirmation response lacks publication evidence")
    })?;
    if &publication.fleet_registry != published_registry
        || publication.fleet_directory_content_hash != expected_fleet_directory_content_hash
    {
        return Err(InternalError::conflict(
            "Directory confirmation response names different Fleet publication authority",
        ));
    }
    validate_root_publication_evidence(record, batch, response, publication)?;
    match response.phase {
        RootComponentProvisioningPhase::Provisioned => {
            if response.published_at_ns.is_some()
                || response.receipt_content_hash != previous.receipt_content_hash
            {
                return Err(InternalError::conflict(
                    "in-progress Directory confirmation changed terminal receipt evidence",
                ));
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
                return Err(InternalError::conflict(
                    "Published Directory confirmation has invalid terminal progress",
                ));
            }
            let expected = RootComponentProvisioningReceiptOps::published_content_hash(
                RootComponentProvisioningPublishedReceiptAuthority {
                    operation_id: record.operation_id,
                    plan_hash: record.plan_hash,
                    configuration_digest: record.plan.configuration_digest,
                    root: &batch.root,
                    result,
                    publication,
                    accepted_at_ns: response.accepted_at_ns,
                    provisioned_at_ns,
                    published_at_ns,
                },
            )?;
            if response.receipt_content_hash != expected {
                return Err(InternalError::conflict(
                    "Published Directory confirmation receipt hash is invalid",
                ));
            }
        }
        _ => {
            return Err(InternalError::conflict(
                "Directory confirmation response has an invalid root phase",
            ));
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
        return Err(InternalError::conflict(
            "Directory confirmation evidence count differs from root progress",
        ));
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
            return Err(InternalError::conflict(
                "Component Directory publication evidence differs from Registry authority",
            ));
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
            return Err(InternalError::conflict(
                "Component Group Directory publication evidence is invalid",
            ));
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
    let index = usize::try_from(root_index)
        .map_err(|_| InternalError::resource_exhausted("accepted root index does not fit usize"))?;
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
        let index = usize::try_from(request.expected_provisioned_root_count).map_err(|_| {
            InternalError::resource_exhausted("provisioned root index does not fit usize")
        })?;
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
        return Err(InternalError::conflict(
            "Fleet Component root provisioning retry returned different evidence",
        ));
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
    Err(InternalError::conflict(
        "Fleet Component root acceptance expected count differs from durable progress",
    ))
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
    let index = usize::try_from(root_index).map_err(|_| {
        InternalError::resource_exhausted("Fleet Component root index does not fit usize")
    })?;
    record.plan.batches.get(index).ok_or_else(|| {
        InternalError::conflict("Fleet Component root acceptance cursor is terminal")
    })
}

fn replay_recorded_root_acceptance(
    record: &FleetComponentProvisioningRecord,
    request: &FleetComponentProvisioningAdvanceRequest,
    response: &RootComponentProvisioningStatusResponse,
    progress: &FleetComponentProvisioningRootAcceptanceProgress,
) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
    if request.expected_accepted_root_count.checked_add(1) != Some(progress.accepted_root_count) {
        return Err(InternalError::conflict(
            "Fleet Component root acceptance response is older than one durable step",
        ));
    }
    let index = usize::try_from(request.expected_accepted_root_count).map_err(|_| {
        InternalError::resource_exhausted("Fleet Component root index does not fit usize")
    })?;
    let recorded = progress.acceptances.get(index).ok_or_else(|| {
        receipt_invariant("recorded root acceptance is absent at its durable cursor")
    })?;
    if &recorded.response != response {
        return Err(InternalError::conflict(
            "Fleet Component root acceptance retry returned different evidence",
        ));
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
        return Err(InternalError::conflict(
            "root acceptance response differs from protected Coordinator authority",
        ));
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
        return Err(InternalError::conflict(
            "root acceptance response does not describe the exact initial Accepted state",
        ));
    }
    if response.accepted_at_ns == 0 {
        return Err(InternalError::conflict(
            "root acceptance response does not describe the exact initial Accepted state",
        ));
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
        return Err(InternalError::conflict(
            "root acceptance response receipt hash is not canonical",
        ));
    }
    Ok(())
}

fn validate_root_acceptance_observation(
    started_at_ns: u64,
    response: &RootComponentProvisioningStatusResponse,
    recorded_at_ns: u64,
) -> Result<(), InternalError> {
    if response.accepted_at_ns < started_at_ns || recorded_at_ns < response.accepted_at_ns {
        return Err(InternalError::invalid_input(
            "Fleet Component root acceptance time evidence is invalid",
        ));
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
        return Err(InternalError::conflict(
            "root provisioning response changed its accepted-time authority",
        ));
    }
    match response.phase {
        RootComponentProvisioningPhase::Accepted => {
            validate_root_provision_current(record, batch, acceptance, response)?;
            let previous_counts = RootProvisioningCounts::from_response(previous);
            let next_counts = RootProvisioningCounts::from_response(response);
            if !previous_counts.advances_one_step_to(next_counts, response.component_count) {
                return Err(InternalError::conflict(
                    "root provisioning response did not advance exactly one bounded cursor",
                ));
            }
        }
        RootComponentProvisioningPhase::Provisioned => {
            if !RootProvisioningCounts::from_response(previous)
                .is_terminal(previous.component_count)
            {
                return Err(InternalError::conflict(
                    "root provisioning terminal response preceded complete root-local cursors",
                ));
            }
            FleetServiceBindingOps::validate_provisioned_root_receipt_compiled(
                configuration,
                &record.plan,
                record.operation_id,
                record.plan_hash,
                usize::try_from(root_index).map_err(|_| {
                    InternalError::resource_exhausted("root provisioning index does not fit usize")
                })?,
                response,
            )?;
            let provisioned_at_ns = response.provisioned_at_ns.ok_or_else(|| {
                InternalError::conflict("root Provisioned response has no completion time")
            })?;
            if provisioned_at_ns < started_at_ns || recorded_at_ns < provisioned_at_ns {
                return Err(InternalError::invalid_input(
                    "root Provisioned response time evidence is invalid",
                ));
            }
        }
        RootComponentProvisioningPhase::Published
        | RootComponentProvisioningPhase::RuntimesActive => {
            return Err(InternalError::conflict(
                "root provisioning advance crossed the Coordinator publication barrier",
            ));
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
        return Err(InternalError::conflict(
            "root provisioning response differs from its exact accepted plan authority",
        ));
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
    FleetServiceBindingOps::compile_initial_compiled(
        configuration,
        source_registry,
        &record.plan,
        record.operation_id,
        &receipts,
    )
    .map_err(|_| {
        receipt_invariant("complete root provisioning receipts do not compile canonical services")
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
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            format!(
                "current root provisioning observation time evidence is invalid: previous {previous_observed_at_ns}, started {}, recorded {}",
                current.started_at_ns, current.recorded_at_ns
            ),
        ));
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

fn require_grouped_root_lifecycle_open(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    if current.component_provisioning.is_some() {
        return Err(InternalError::conflict(
            "Fleet Subnet Root lifecycle is fenced while grouped Component provisioning authority exists",
        ));
    }
    Ok(())
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
        .ok_or_else(|| {
            InternalError::forbidden(
                "caller is not a current Fleet Subnet Root in the Fleet Registry",
            )
        })
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
        .ok_or_else(|| {
            InternalError::forbidden(
                "caller is not a Joining Fleet Subnet Root in the current Registry",
            )
        })
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
        return Err(InternalError::conflict(
            "Fleet Registry snapshot synchronization requires a non-empty all-Joining root set",
        ));
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
        return Err(InternalError::conflict(
            "Fleet Registry activation requires every current root acknowledgement",
        ));
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
    apply_service_publication_receipt(current, &mut historical_registry, &mut history)?;
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
        let has_lifecycle_receipts = current.service_publication_receipt.is_some()
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
        return Err(InternalError::conflict(
            "initial Fleet-service publication requires an active Fleet Registry",
        ));
    }
    Ok(active)
}

fn apply_service_publication_receipt(
    current: &FleetCoordinatorRegistryRecord,
    historical_registry: &mut FleetRegistry,
    history: &mut Vec<FleetRegistryHistoryPoint>,
) -> Result<(), InternalError> {
    let Some(receipt) = &current.service_publication_receipt else {
        return Ok(());
    };
    if !FleetServicePublicationAuthority::from_receipt(receipt).is_complete() {
        return Err(receipt_invariant(
            "initial Fleet-service publication receipt authority is incomplete",
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
            "initial Fleet-service publication source differs from canonical history",
        ));
    }
    let next_registry = if receipt.services.is_empty() {
        historical_registry.clone()
    } else {
        FleetRegistryOps::compile_initial_services(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            historical_registry,
            receipt.services.clone(),
        )
        .map_err(|_| {
            receipt_invariant("initial Fleet-service publication target Registry cannot be derived")
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
            "initial Fleet-service publication response differs from canonical history",
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
        if self.operation_id == [0; 32]
            || self.plan_hash == [0; 32]
            || self.configuration_digest.as_bytes() == &[0; 32]
        {
            return false;
        }
        !self.root_receipt_content_hashes.is_empty()
            && self.root_receipt_content_hashes.len()
                <= MAX_FLEET_COMPONENT_PROVISIONING_PLAN_BATCHES
            && self
                .root_receipt_content_hashes
                .iter()
                .all(|hash| hash != &[0; 32])
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
    validate_draining_publication_request(historical_registry, &previous_version, &receipt.request)
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
        let identity = FleetSubnetRootDrainingPublicationIdentity::from_request(&receipt.request);
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
struct FleetSubnetRootDrainingAuthority<'a> {
    fleet_subnet_root: Principal,
    placement_subnet: SubnetId,
    active_registry: &'a FleetRegistryVersion,
    component_topology_digest: ComponentTopologyDigest,
    active_release_set: FleetSubnetRootReleaseSet,
}

impl<'a> FleetSubnetRootDrainingAuthority<'a> {
    const fn from_registry(
        entry: &'a FleetSubnetRootEntry,
        version: &'a FleetRegistryVersion,
    ) -> Self {
        Self {
            fleet_subnet_root: entry.fleet_subnet_root,
            placement_subnet: entry.placement_subnet,
            active_registry: version,
            component_topology_digest: entry.component_topology_digest,
            active_release_set: entry.active_release_set,
        }
    }

    const fn from_publication(request: &'a FleetSubnetRootDrainingPublicationRequest) -> Self {
        let draining = &request.root_draining;
        Self {
            fleet_subnet_root: draining.fleet_subnet_root,
            placement_subnet: draining.placement_subnet,
            active_registry: &draining.active_registry,
            component_topology_digest: draining.component_topology_digest,
            active_release_set: draining.active_release_set,
        }
    }
}

fn draining_publication_identity_matches(
    receipt: &FleetSubnetRootDrainingPublicationReceiptRecord,
    request: &FleetSubnetRootDrainingPublicationRequest,
) -> bool {
    FleetSubnetRootDrainingPublicationIdentity::from_request(&receipt.request).conflicts_with(
        FleetSubnetRootDrainingPublicationIdentity::from_request(request),
    )
}

#[derive(Clone, Copy)]
struct FleetSubnetRootDrainingPublicationIdentity {
    fleet_subnet_root: Principal,
    operation_id: [u8; 32],
}

impl FleetSubnetRootDrainingPublicationIdentity {
    const fn from_request(request: &FleetSubnetRootDrainingPublicationRequest) -> Self {
        Self {
            fleet_subnet_root: request.root_draining.fleet_subnet_root,
            operation_id: request.root_draining.operation_id,
        }
    }

    fn conflicts_with(self, other: Self) -> bool {
        self.fleet_subnet_root == other.fleet_subnet_root || self.operation_id == other.operation_id
    }
}

fn validate_draining_publication_request(
    registry: &FleetRegistry,
    version: &FleetRegistryVersion,
    request: &FleetSubnetRootDrainingPublicationRequest,
) -> Result<(), &'static str> {
    let draining = &request.root_draining;
    if request.expected_registry != *version || draining.active_registry != *version {
        return Err("Fleet Subnet Root draining publication names stale Registry authority");
    }
    let target = registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| entry.fleet_subnet_root == draining.fleet_subnet_root)
        .ok_or("Fleet Subnet Root draining publication target is missing")?;
    if target.status != FleetSubnetRootStatus::Active {
        return Err("Fleet Subnet Root draining publication target is not Active");
    }
    let expected_authority = FleetSubnetRootDrainingAuthority::from_registry(target, version);
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
    let terminal_facts_are_exact = [
        inventory.operation_id != [0; 32],
        inventory.next_allocation_sequence > 0,
        removed_instances_are_exact,
        inventory.terminal_component_history_hash != [0; 32],
        inventory.root_registry_encoded_bytes <= target.limits.maximum_registry_bytes,
        inventory.wasm_store != Principal::anonymous(),
        inventory.wasm_store_catalog_hash != [0; 32],
        inventory.wasm_store_catalog_entries > 0,
        inventory.wasm_store_release_count == inventory.wasm_store_catalog_entries,
        inventory.wasm_store_template_count > 0,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RootDeletionIdentity {
    operation_id: [u8; 32],
    fleet_subnet_root: Principal,
}

impl RootDeletionIdentity {
    const fn new(operation_id: [u8; 32], fleet_subnet_root: Principal) -> Self {
        Self {
            operation_id,
            fleet_subnet_root,
        }
    }

    fn conflicts_with(self, other: Self) -> bool {
        self.operation_id == other.operation_id || self.fleet_subnet_root == other.fleet_subnet_root
    }
}

fn require_root_deletion_caller(
    caller: Principal,
    fleet_subnet_root: Principal,
) -> Result<(), InternalError> {
    if caller != fleet_subnet_root {
        return Err(InternalError::forbidden(
            "Fleet Subnet Root deletion readiness caller differs from its root",
        ));
    }
    Ok(())
}

fn require_coordinator_identity(
    current: &FleetCoordinatorRegistryRecord,
    coordinator: Principal,
) -> Result<(), InternalError> {
    if current.authority.binding.coordinator != coordinator {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "Fleet Coordinator runtime principal differs from protected authority",
        ));
    }
    Ok(())
}

fn require_removed_root_publication(
    current: &FleetCoordinatorRegistryRecord,
    operation_id: [u8; 32],
    fleet_subnet_root: Principal,
) -> Result<&FleetSubnetRootRemovalPublicationReceiptRecord, InternalError> {
    let target_is_removed = current.registry.fleet_subnet_roots.iter().any(|entry| {
        entry.fleet_subnet_root == fleet_subnet_root
            && entry.status == FleetSubnetRootStatus::Removed
    });
    if !target_is_removed {
        return Err(InternalError::conflict(
            "Fleet Subnet Root deletion requires an exact Removed Registry row",
        ));
    }
    current
        .root_removal_publication_receipts
        .iter()
        .find(|receipt| {
            receipt.request.final_inventory.operation_id == operation_id
                && receipt.request.final_inventory.fleet_subnet_root == fleet_subnet_root
        })
        .ok_or_else(|| {
            InternalError::unavailable(
                "Fleet Subnet Root deletion lacks its logical-removal publication receipt",
            )
        })
}

fn find_root_deletion_record<'a, T>(
    records: &'a [T],
    requested: RootDeletionIdentity,
    identity: impl Fn(&T) -> RootDeletionIdentity,
    label: &'static str,
) -> Result<Option<&'a T>, InternalError> {
    let mut matching = records
        .iter()
        .filter(|record| identity(record).conflicts_with(requested));
    let Some(found) = matching.next() else {
        return Ok(None);
    };
    if identity(found) != requested {
        return Err(InternalError::conflict(format!(
            "{label} identity already has different authority"
        )));
    }
    if matching.next().is_some() {
        return Err(receipt_invariant(
            "Fleet Subnet Root deletion identity is not unique",
        ));
    }
    Ok(Some(found))
}

fn find_root_deletion_readiness_intent(
    current: &FleetCoordinatorRegistryRecord,
    operation_id: [u8; 32],
    fleet_subnet_root: Principal,
) -> Result<Option<&FleetSubnetRootDeletionReadinessIntentResponse>, InternalError> {
    find_root_deletion_record(
        &current.root_deletion_readiness_intents,
        RootDeletionIdentity::new(operation_id, fleet_subnet_root),
        |record| {
            RootDeletionIdentity::new(
                record.request.operation_id,
                record.request.fleet_subnet_root,
            )
        },
        "Fleet Subnet Root deletion-readiness intent",
    )
}

fn find_root_deletion_readiness(
    current: &FleetCoordinatorRegistryRecord,
    operation_id: [u8; 32],
    fleet_subnet_root: Principal,
) -> Result<Option<&FleetSubnetRootDeletionReadinessResponse>, InternalError> {
    find_root_deletion_record(
        &current.root_deletion_readiness_receipts,
        RootDeletionIdentity::new(operation_id, fleet_subnet_root),
        |record| {
            RootDeletionIdentity::new(
                record.request.operation_id,
                record.request.fleet_subnet_root,
            )
        },
        "Fleet Subnet Root deletion readiness",
    )
}

fn find_root_deletion_execution(
    current: &FleetCoordinatorRegistryRecord,
    operation_id: [u8; 32],
    fleet_subnet_root: Principal,
) -> Result<Option<&FleetSubnetRootDeletionExecutionResponse>, InternalError> {
    find_root_deletion_record(
        &current.root_deletion_execution_intents,
        RootDeletionIdentity::new(operation_id, fleet_subnet_root),
        |record| {
            RootDeletionIdentity::new(
                record.request.operation_id,
                record.request.fleet_subnet_root,
            )
        },
        "Fleet Subnet Root deletion execution",
    )
}

fn find_root_deletion(
    current: &FleetCoordinatorRegistryRecord,
    operation_id: [u8; 32],
    fleet_subnet_root: Principal,
) -> Result<Option<&FleetSubnetRootDeletionResponse>, InternalError> {
    find_root_deletion_record(
        &current.root_deletion_receipts,
        RootDeletionIdentity::new(operation_id, fleet_subnet_root),
        |record| RootDeletionIdentity::new(record.operation_id, record.fleet_subnet_root),
        "Fleet Subnet Root deletion receipt",
    )
}

fn sort_root_deletion_records<T>(records: &mut [T], root: impl Fn(&T) -> Principal) {
    records.sort_by(|left, right| root(left).as_slice().cmp(root(right).as_slice()));
}

fn canonical_root_deletion_record_order<T>(records: &[T], root: impl Fn(&T) -> Principal) -> bool {
    records
        .windows(2)
        .all(|pair| root(&pair[0]).as_slice() < root(&pair[1]).as_slice())
}

fn canonical_controller_set(controllers: &[Principal]) -> bool {
    !controllers.is_empty() && controllers.windows(2).all(|pair| pair[0] < pair[1])
}

fn root_deletion_retained_cycles_target(
    idle_cycles_burned_per_day: u128,
    freezing_threshold_seconds: u128,
) -> Result<u128, InternalError> {
    let freezing_reserve = idle_cycles_burned_per_day
        .checked_mul(freezing_threshold_seconds)
        .ok_or_else(|| {
            InternalError::invalid_input("Fleet Subnet Root freezing reserve overflows u128")
        })?
        .div_ceil(SECONDS_PER_DAY);
    freezing_reserve
        .checked_add(FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES)
        .ok_or_else(|| {
            InternalError::invalid_input("Fleet Subnet Root deletion reserve overflows u128")
        })
}

fn validate_root_deletion_execution_request(
    executor: Principal,
    request: &FleetSubnetRootDeletionExecutionRequest,
    readiness: &FleetSubnetRootDeletionReadinessResponse,
    prepared_at_ns: u64,
) -> Result<(), InternalError> {
    let expected_target = root_deletion_retained_cycles_target(
        request.observed_idle_cycles_burned_per_day,
        request.observed_freezing_threshold_seconds,
    )?;
    let execution_is_exact = [
        request.expected_readiness_hash == readiness.readiness_hash,
        request.observed_module_hash != [0; 32],
        canonical_controller_set(&request.observed_controllers),
        request.observed_controllers.contains(&executor),
        request.observed_cycles_after_reclamation <= readiness.observed_cycles_before_reclamation,
        request.observed_cycles_after_reclamation <= readiness.retained_cycles_target,
        request.observed_reserved_cycles == readiness.observed_reserved_cycles,
        request.observed_idle_cycles_burned_per_day
            == readiness.observed_idle_cycles_burned_per_day,
        request.observed_freezing_threshold_seconds
            == readiness.observed_freezing_threshold_seconds,
        expected_target == readiness.retained_cycles_target,
        prepared_at_ns >= readiness.recorded_at_ns,
    ]
    .into_iter()
    .all(|valid| valid);
    if !execution_is_exact {
        return Err(InternalError::conflict(
            "Fleet Subnet Root deletion execution differs from durable readiness authority",
        ));
    }
    Ok(())
}

fn response_hash<T: CandidType + Serialize>(
    domain: &[u8],
    response_with_zero_hash: &T,
    label: &str,
) -> Result<[u8; 32], InternalError> {
    let payload = candid::encode_one(response_with_zero_hash).map_err(|error| {
        InternalError::invariant(
            InternalErrorOrigin::Ops,
            format!("Fleet Subnet Root {label} cannot be encoded: {error}"),
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(payload);
    Ok(hasher.finalize().into())
}

fn validate_root_deletion_history(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    validate_root_deletion_record_order(current)?;
    validate_root_deletion_readiness_intents(current)?;
    validate_root_deletion_readiness_receipts(current)?;
    validate_root_deletion_execution_intents(current)?;
    validate_root_deletion_receipts(current)
}

fn validate_root_deletion_record_order(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    let records_are_ordered = [
        canonical_root_deletion_record_order(&current.root_deletion_readiness_intents, |record| {
            record.request.fleet_subnet_root
        }),
        canonical_root_deletion_record_order(&current.root_deletion_readiness_receipts, |record| {
            record.request.fleet_subnet_root
        }),
        canonical_root_deletion_record_order(&current.root_deletion_execution_intents, |record| {
            record.request.fleet_subnet_root
        }),
        canonical_root_deletion_record_order(&current.root_deletion_receipts, |record| {
            record.fleet_subnet_root
        }),
    ]
    .into_iter()
    .all(|valid| valid);
    if !records_are_ordered {
        return Err(receipt_invariant(
            "Fleet Subnet Root deletion records are not in canonical order",
        ));
    }
    Ok(())
}

fn validate_root_deletion_readiness_intents(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    for response in &current.root_deletion_readiness_intents {
        let removal = require_removed_root_publication(
            current,
            response.request.operation_id,
            response.request.fleet_subnet_root,
        )?;
        let mut expected = response.clone();
        expected.intent_hash = [0; 32];
        let expected_target = root_deletion_retained_cycles_target(
            response.request.observed_idle_cycles_burned_per_day,
            response.request.observed_freezing_threshold_seconds,
        )?;
        let receipt_is_exact = [
            response.coordinator == current.authority.binding.coordinator,
            response.request.final_inventory_hash
                == removal.response.final_inventory.inventory_hash,
            response.request.store_deletion_hash != [0; 32],
            response.request.observed_cycles_before_reclamation > 0,
            response.request.retained_cycles_target > 0,
            response.request.retained_cycles_target == expected_target,
            response.request.observed_reserved_cycles == 0,
            response.request.prepared_at_ns >= removal.response.final_inventory.finalized_at_ns,
            response.recorded_at_ns >= response.request.prepared_at_ns,
            response.intent_hash
                == response_hash(
                    ROOT_DELETION_READINESS_INTENT_HASH_DOMAIN,
                    &expected,
                    "root deletion-readiness intent",
                )?,
        ]
        .into_iter()
        .all(|valid| valid);
        if !receipt_is_exact {
            return Err(receipt_invariant(
                "Fleet Subnet Root deletion-readiness intent is not canonical",
            ));
        }
    }
    Ok(())
}

fn validate_root_deletion_readiness_receipts(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    for response in &current.root_deletion_readiness_receipts {
        let intent = find_root_deletion_readiness_intent(
            current,
            response.request.operation_id,
            response.request.fleet_subnet_root,
        )?
        .ok_or_else(|| {
            receipt_invariant("Fleet Subnet Root deletion readiness lacks its intent")
        })?;
        let mut expected = response.clone();
        expected.readiness_hash = [0; 32];
        let receipt_is_exact = [
            response.request.expected_intent_hash == intent.intent_hash,
            response.coordinator == intent.coordinator,
            response.final_inventory_hash == intent.request.final_inventory_hash,
            response.store_deletion_hash == intent.request.store_deletion_hash,
            response.observed_cycles_before_reclamation
                == intent.request.observed_cycles_before_reclamation,
            response.retained_cycles_target == intent.request.retained_cycles_target,
            response.observed_reserved_cycles == intent.request.observed_reserved_cycles,
            response.observed_idle_cycles_burned_per_day
                == intent.request.observed_idle_cycles_burned_per_day,
            response.observed_freezing_threshold_seconds
                == intent.request.observed_freezing_threshold_seconds,
            response.request.observed_cycles_after_reclamation
                <= response.observed_cycles_before_reclamation,
            response.request.observed_cycles_after_reclamation <= response.retained_cycles_target,
            response.request.cycles_reclaimed_at_ns >= response.prepared_at_ns,
            response.recorded_at_ns >= response.request.cycles_reclaimed_at_ns,
            response.readiness_hash
                == response_hash(
                    ROOT_DELETION_READINESS_HASH_DOMAIN,
                    &expected,
                    "root deletion readiness",
                )?,
        ]
        .into_iter()
        .all(|valid| valid);
        if !receipt_is_exact {
            return Err(receipt_invariant(
                "Fleet Subnet Root deletion readiness is not canonical",
            ));
        }
    }
    Ok(())
}

fn validate_root_deletion_execution_intents(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    for response in &current.root_deletion_execution_intents {
        let readiness = find_root_deletion_readiness(
            current,
            response.request.operation_id,
            response.request.fleet_subnet_root,
        )?
        .ok_or_else(|| receipt_invariant("Fleet Subnet Root deletion execution lacks readiness"))?;
        validate_root_deletion_execution_request(
            response.executor,
            &response.request,
            readiness,
            response.prepared_at_ns,
        )
        .map_err(|_| receipt_invariant("Fleet Subnet Root deletion execution is not canonical"))?;
        let mut expected = response.clone();
        expected.execution_hash = [0; 32];
        if response.execution_hash
            != response_hash(
                ROOT_DELETION_EXECUTION_HASH_DOMAIN,
                &expected,
                "root deletion execution",
            )?
        {
            return Err(receipt_invariant(
                "Fleet Subnet Root deletion execution hash is not canonical",
            ));
        }
    }
    Ok(())
}

fn validate_root_deletion_receipts(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    for response in &current.root_deletion_receipts {
        let execution = find_root_deletion_execution(
            current,
            response.operation_id,
            response.fleet_subnet_root,
        )?
        .ok_or_else(|| receipt_invariant("Fleet Subnet Root deletion lacks execution intent"))?;
        let mut expected = response.clone();
        expected.deletion_hash = [0; 32];
        let receipt_is_exact = [
            response.coordinator == current.authority.binding.coordinator,
            response.executor == execution.executor,
            response.readiness_hash == execution.request.expected_readiness_hash,
            response.execution_hash == execution.execution_hash,
            response.observed_module_hash == execution.request.observed_module_hash,
            response.observed_controllers == execution.request.observed_controllers,
            response.observed_cycles_after_reclamation
                == execution.request.observed_cycles_after_reclamation,
            response.observed_absent_at_ns >= execution.prepared_at_ns,
            response.completed_at_ns >= response.observed_absent_at_ns,
            response.deletion_hash
                == response_hash(ROOT_DELETION_HASH_DOMAIN, &expected, "root deletion")?,
        ]
        .into_iter()
        .all(|valid| valid);
        if !receipt_is_exact {
            return Err(receipt_invariant(
                "Fleet Subnet Root deletion receipt is not canonical",
            ));
        }
    }
    Ok(())
}

fn receipt_invariant(message: &'static str) -> InternalError {
    InternalError::invariant(InternalErrorOrigin::Storage, message)
}
