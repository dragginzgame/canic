//! Module: ops::fleet_coordinator
//!
//! Responsibility: validate, compile, commit, and read Fleet Coordinator Registry state.
//! Does not own: endpoint authorization, multi-step lifecycle orchestration, or root effects.
//! Boundary: workflow supplies protected init facts and receives canonical Registry projections.

use crate::{
    dto::fleet_coordinator::FleetCoordinatorInitArgs,
    storage::stable::fleet_coordinator::{
        FleetComponentProvisioningRecord, FleetComponentProvisioningRootAcceptanceIntentRecord,
        FleetComponentProvisioningRootAcceptanceRecord, FleetComponentProvisioningStateRecord,
        FleetCoordinatorCommitError, FleetCoordinatorCommitOutcome, FleetCoordinatorRegistryRecord,
        FleetCoordinatorRegistryStore, FleetRegistryActivationReceiptRecord,
        FleetServicePublicationReceiptRecord, FleetSubnetRootDrainingPublicationReceiptRecord,
        FleetSubnetRootJoinReceiptRecord, FleetSubnetRootRemovalPublicationReceiptRecord,
    },
    view::fleet_coordinator::{
        FleetComponentProvisioningRootAcceptanceCallView,
        FleetComponentProvisioningRootAcceptanceDisposition,
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
                RootComponentProvisioningReceiptOps,
            },
            fleet_registry::FleetRegistryOps,
            fleet_service_binding::FleetServiceBindingOps,
        },
    },
    dto::fleet_subnet_root::FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES,
    dto::{
        component_provisioning::{
            FleetComponentProvisioningAdvanceRequest, FleetComponentProvisioningOperation,
            FleetComponentProvisioningPhase, FleetComponentProvisioningPlan,
            FleetComponentProvisioningPrepareRequest, FleetComponentProvisioningStatusRequest,
            FleetComponentProvisioningStatusResponse, FleetSubnetRootProvisioningBatch,
            RootComponentProvisioningAcceptanceRequest, RootComponentProvisioningPhase,
            RootComponentProvisioningStatusResponse,
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
        if let Some(existing) = &current.component_provisioning {
            if existing.operation_id == request.operation_id && existing.plan == request.plan {
                return component_provisioning_status_response(existing);
            }
            return Err(InternalError::conflict(
                "Fleet Component provisioning already contains different protected plan authority",
            ));
        }
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
        if request.plan.operation != FleetComponentProvisioningOperation::FreshInstall {
            return Err(InternalError::invalid_input(
                "Fleet Component provisioning preparation currently accepts only fresh installation",
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

    pub(crate) fn component_provisioning_status(
        request: FleetComponentProvisioningStatusRequest,
    ) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
        let current = Self::current()?;
        let record = current.component_provisioning.as_ref().ok_or_else(|| {
            InternalError::unavailable("Fleet Component provisioning plan is not prepared")
        })?;
        if record.operation_id != request.operation_id || record.plan_hash != request.plan_hash {
            return Err(InternalError::conflict(
                "Fleet Component provisioning status names different protected plan authority",
            ));
        }
        component_provisioning_status_response(record)
    }

    pub(crate) fn advance_component_provisioning_root_acceptance(
        request: FleetComponentProvisioningAdvanceRequest,
        started_at_ns: u64,
    ) -> Result<FleetComponentProvisioningRootAcceptanceDisposition, InternalError> {
        let current = Self::current()?;
        let record = require_component_provisioning_record(&current, &request)?;
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
            let next_record = component_provisioning_record_mut(&mut next)?;
            next_record.state = FleetComponentProvisioningStateRecord::RootsAccepted {
                planned_at_ns: progress.planned_at_ns,
                acceptances: progress.acceptances,
                roots_accepted_at_ns: started_at_ns,
            };
            let next = Self::validate_current(next)?;
            let response =
                component_provisioning_status_response(component_provisioning_record(&next)?)?;
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
        component_provisioning_record_mut(&mut next)?.state =
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
        let record = require_component_provisioning_record(&current, &request)?;
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
        let next_record = component_provisioning_record_mut(&mut next)?;
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
        let result = component_provisioning_status_response(component_provisioning_record(&next)?)?;
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
    pub(crate) fn require_root_lifecycle_open_for_test(
        config: &ConfigModel,
    ) -> Result<(), InternalError> {
        require_test_component_deployment_configuration(config)?;
        let current = Self::current()?;
        require_grouped_root_lifecycle_open(&current)
    }

    #[expect(
        dead_code,
        reason = "the next Coordinator workflow slice will call the closed publication compiler"
    )]
    pub(crate) fn publish_initial_services(
        plan: &FleetComponentProvisioningPlan,
        operation_id: [u8; 32],
        root_receipts: &[RootComponentProvisioningStatusResponse],
    ) -> Result<FleetRegistryVersion, InternalError> {
        let current = Self::current()?;
        let source_registry = initial_active_registry(&current)?;
        let services = FleetServiceBindingOps::compile_initial_compiled(
            &current.component_deployment_configuration,
            &source_registry,
            plan,
            operation_id,
            root_receipts,
        )?;
        let plan_hash = ComponentProvisioningPlanOps::hash_compiled(
            &current.component_deployment_configuration,
            &source_registry,
            plan,
        )?;
        let receipt_hashes = root_receipts
            .iter()
            .map(|receipt| receipt.receipt_content_hash)
            .collect::<Vec<_>>();
        Self::commit_compiled_initial_services(
            plan.fleet_registry.clone(),
            operation_id,
            plan_hash,
            plan.configuration_digest,
            receipt_hashes,
            services,
        )
    }

    fn commit_compiled_initial_services(
        expected_registry: FleetRegistryVersion,
        operation_id: [u8; 32],
        plan_hash: [u8; 32],
        configuration_digest: canic_core::ids::ComponentDeploymentConfigurationDigest,
        receipt_hashes: Vec<[u8; 32]>,
        services: Vec<canic_core::dto::fleet_registry::FleetServiceBinding>,
    ) -> Result<FleetRegistryVersion, InternalError> {
        let current = Self::current()?;
        if let Some(receipt) = &current.service_publication_receipt {
            if service_publication_matches(
                receipt,
                operation_id,
                plan_hash,
                configuration_digest,
                &receipt_hashes,
                &services,
            ) {
                return Ok(receipt.version.clone());
            }
            return Err(InternalError::conflict(
                "initial Fleet-service publication already committed against different authority",
            ));
        }
        let source_registry = initial_active_registry(&current)?;
        if current.registry != source_registry {
            return Err(InternalError::conflict(
                "initial Fleet-service publication must precede later Registry transitions",
            ));
        }

        let previous_version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &current.registry,
        )?;
        if previous_version != expected_registry {
            return Err(InternalError::conflict(
                "initial Fleet-service publication expected Registry version is stale",
            ));
        }
        let next_registry = if services.is_empty() {
            current.registry.clone()
        } else {
            FleetRegistryOps::compile_initial_services(
                &current.authority,
                &current
                    .component_deployment_configuration
                    .component_topology,
                &current.registry,
                services.clone(),
            )?
        };
        let version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &next_registry,
        )?;
        let mut next = current.clone();
        next.registry = next_registry;
        next.service_publication_receipt = Some(FleetServicePublicationReceiptRecord {
            operation_id,
            plan_hash,
            configuration_digest,
            root_receipt_content_hashes: receipt_hashes,
            services,
            previous_version,
            version: version.clone(),
        });
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(version)
    }

    #[cfg(test)]
    pub(crate) fn commit_compiled_initial_services_for_test(
        expected_registry: FleetRegistryVersion,
        operation_id: [u8; 32],
        plan_hash: [u8; 32],
        configuration_digest: canic_core::ids::ComponentDeploymentConfigurationDigest,
        receipt_hashes: Vec<[u8; 32]>,
        services: Vec<canic_core::dto::fleet_registry::FleetServiceBinding>,
    ) -> Result<FleetRegistryVersion, InternalError> {
        Self::commit_compiled_initial_services(
            expected_registry,
            operation_id,
            plan_hash,
            configuration_digest,
            receipt_hashes,
            services,
        )
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
    validate_service_publication_authority(current, record)?;
    component_provisioning_plan_counts(&record.plan)?;
    Ok(())
}

fn component_provisioning_status_response(
    record: &FleetComponentProvisioningRecord,
) -> Result<FleetComponentProvisioningStatusResponse, InternalError> {
    let counts = component_provisioning_plan_counts(&record.plan)?;
    let progress = component_provisioning_root_acceptance_progress(record)?;
    Ok(FleetComponentProvisioningStatusResponse {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        fleet_registry: record.plan.fleet_registry.clone(),
        configuration_digest: record.plan.configuration_digest,
        operation: record.plan.operation.clone(),
        phase: progress.phase,
        directory_confirmation_root_count: counts.directory_confirmation_roots,
        root_batch_count: counts.root_batches,
        accepted_root_count: progress.accepted_root_count,
        acceptance_in_flight_root: progress.in_flight.map(|intent| intent.fleet_subnet_root),
        group_placement_count: counts.group_placements,
        component_count: counts.components,
        planned_at_ns: progress.planned_at_ns,
        roots_accepted_at_ns: progress.roots_accepted_at_ns,
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

#[derive(Clone, Copy, Eq, PartialEq)]
enum RootAcceptanceAdvance {
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
    let Some(receipt) = &current.service_publication_receipt else {
        return Ok(());
    };
    if component_provisioning_authority(record) != service_publication_authority(receipt) {
        return Err(receipt_invariant(
            "Fleet-service publication receipt differs from its provisioning plan",
        ));
    }
    Ok(())
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

fn component_provisioning_root_acceptance_progress(
    record: &FleetComponentProvisioningRecord,
) -> Result<FleetComponentProvisioningRootAcceptanceProgress, InternalError> {
    let root_batch_count = u32::try_from(record.plan.batches.len())
        .map_err(|_| receipt_invariant("root batch count does not fit u32"))?;
    let (planned_at_ns, phase, acceptances, in_flight, roots_accepted_at_ns) = match &record.state {
        FleetComponentProvisioningStateRecord::Planned { planned_at_ns } => (
            *planned_at_ns,
            FleetComponentProvisioningPhase::Planned,
            Vec::new(),
            None,
            None,
        ),
        FleetComponentProvisioningStateRecord::AcceptingRoots {
            planned_at_ns,
            acceptances,
            in_flight,
        } => (
            *planned_at_ns,
            FleetComponentProvisioningPhase::AcceptingRoots,
            acceptances.clone(),
            *in_flight,
            None,
        ),
        FleetComponentProvisioningStateRecord::RootsAccepted {
            planned_at_ns,
            acceptances,
            roots_accepted_at_ns,
        } => (
            *planned_at_ns,
            FleetComponentProvisioningPhase::RootsAccepted,
            acceptances.clone(),
            None,
            Some(*roots_accepted_at_ns),
        ),
    };
    let accepted_root_count = u32::try_from(acceptances.len())
        .map_err(|_| receipt_invariant("accepted root count does not fit u32"))?;
    Ok(FleetComponentProvisioningRootAcceptanceProgress {
        planned_at_ns,
        phase,
        acceptances,
        accepted_root_count,
        root_batch_count,
        in_flight,
        roots_accepted_at_ns,
    })
}

fn classify_root_acceptance_advance(
    request: &FleetComponentProvisioningAdvanceRequest,
    progress: &FleetComponentProvisioningRootAcceptanceProgress,
) -> Result<RootAcceptanceAdvance, InternalError> {
    if request.expected_accepted_root_count == progress.accepted_root_count {
        if progress.phase == FleetComponentProvisioningPhase::RootsAccepted {
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

fn service_publication_matches(
    receipt: &FleetServicePublicationReceiptRecord,
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    configuration_digest: canic_core::ids::ComponentDeploymentConfigurationDigest,
    root_receipt_content_hashes: &[[u8; 32]],
    services: &[canic_core::dto::fleet_registry::FleetServiceBinding],
) -> bool {
    let expected = FleetServicePublicationAuthority::from_receipt(receipt);
    let received = FleetServicePublicationAuthority {
        operation_id,
        plan_hash,
        configuration_digest,
        root_receipt_content_hashes,
        services,
    };
    expected == received
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
