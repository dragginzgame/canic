//! Module: ops::fleet_coordinator::funding_rotation
//!
//! Responsibility: own one staged and replayable Fleet funding-policy rotation.
//! Does not own: endpoint authorization, clocks, timers, or inter-canister calls.
//! Boundary: every transition is digest-bound and the current record is the sole Fleet fence.

use super::FleetCoordinatorOps;
use crate::{
    dto::fleet_coordinator::{
        FleetFundingPolicyRotationStatusPhase, FleetFundingPolicyRotationStatusResponse,
    },
    storage::stable::fleet_coordinator::{
        FleetComponentProvisioningRecord, FleetComponentProvisioningStateRecord,
        FleetCoordinatorCommitError, FleetCoordinatorFundingRecord, FleetCoordinatorFundingStore,
        FleetCoordinatorRegistryStore, FleetFundingPolicyRotationCheckpointRecord,
        FleetFundingPolicyRotationPhaseRecord, FleetFundingPolicyRotationRecord,
        FleetFundingPolicyRotationRootCheckpointRecord, FleetRootFundingWindowRecord,
    },
    view::fleet_coordinator::FleetFundingPolicyRotationStep,
};
use canic_core::{
    control_plane_support::{error::InternalError, ops::fleet_registry::FleetRegistryOps},
    dto::{
        fleet_funding::{
            FleetFundingPolicyRotationApplyRequest, FleetFundingPolicyRotationBeginRequest,
            FleetFundingPolicyRotationPlan, FleetFundingPolicyRotationReceipt,
            FleetFundingPolicyRotationRootActivateRequest,
            FleetFundingPolicyRotationRootPrepareRequest, FleetFundingPolicyRotationRootReceipt,
            FleetFundingPolicyRotationStageRootRequest, FleetFundingPolicyUsage,
            MAX_FLEET_FUNDING_POLICY_ROTATION_HISTORY_ROOTS,
            MAX_FLEET_FUNDING_POLICY_ROTATION_ROOTS,
        },
        fleet_registry::{FleetRegistryVersion, FleetSubnetRootStatus},
        role::OperationReceipt,
    },
    shared_support::fleet_funding_policy::{
        coordinator_root_funding_policy_hash, fleet_funding_policy_rotation_operation_id,
        fleet_funding_policy_rotation_plan_digest, fleet_funding_policy_rotation_roots_digest,
        fleet_funding_policy_rotation_successor_policy_set_hash,
        fleet_subnet_root_funding_policy_hash, validate_coordinator_root_funding_policy,
        validate_fleet_funding_policy_rotation_plan, validate_fleet_subnet_root_funding_authority,
    },
};

impl FleetCoordinatorOps {
    pub(crate) fn funding_policy_rotation_status(
        operation_id: [u8; 32],
    ) -> Result<Option<FleetFundingPolicyRotationStatusResponse>, InternalError> {
        let funding = raw_funding()?;
        if let Some(rotation) = funding
            .rotation_current
            .as_ref()
            .filter(|rotation| rotation.operation_id == operation_id)
        {
            let expected_root_count = rotation.header.affected_root_count;
            let phase = match &rotation.phase {
                FleetFundingPolicyRotationPhaseRecord::Staging => {
                    FleetFundingPolicyRotationStatusPhase::Staging {
                        staged_root_count: rotation_root_count(rotation.roots.len())?,
                        expected_root_count,
                    }
                }
                FleetFundingPolicyRotationPhaseRecord::PreparingRoots { prepared } => {
                    FleetFundingPolicyRotationStatusPhase::PreparingRoots {
                        prepared_root_count: rotation_root_count(prepared.len())?,
                        expected_root_count,
                    }
                }
                FleetFundingPolicyRotationPhaseRecord::ActivatingRoots {
                    successor_registry,
                    activated,
                    ..
                } => FleetFundingPolicyRotationStatusPhase::ActivatingRoots {
                    activated_root_count: rotation_root_count(activated.len())?,
                    expected_root_count,
                    successor_registry: successor_registry.as_ref().clone(),
                },
            };
            return Ok(Some(FleetFundingPolicyRotationStatusResponse {
                operation_id,
                plan_digest: rotation.plan_digest,
                predecessor_generation: rotation.header.predecessor_generation,
                successor_generation: rotation.header.successor_generation,
                phase,
            }));
        }
        Ok(
            completed_rotation_checkpoint_by_operation(&funding, operation_id)?.map(|checkpoint| {
                FleetFundingPolicyRotationStatusResponse {
                    operation_id,
                    plan_digest: checkpoint.receipt.plan_digest,
                    predecessor_generation: checkpoint.receipt.predecessor_generation,
                    successor_generation: checkpoint.receipt.successor_generation,
                    phase: FleetFundingPolicyRotationStatusPhase::Completed(Box::new(
                        checkpoint.receipt.clone(),
                    )),
                }
            }),
        )
    }

    #[expect(
        clippy::too_many_lines,
        reason = "begin validates the complete immutable plan header before its sole commit"
    )]
    pub(crate) fn begin_funding_policy_rotation(
        request: FleetFundingPolicyRotationBeginRequest,
        now_ns: u64,
    ) -> Result<OperationReceipt, InternalError> {
        let registry = Self::current()?;
        let current = raw_funding()?;
        if let Some(checkpoint) =
            completed_rotation_checkpoint(&current, request.operation_id, request.plan_digest)?
        {
            if checkpoint.plan.header != request.header {
                return Err(InternalError::conflict());
            }
            return Ok(OperationReceipt {
                operation_id: request.operation_id,
            });
        }
        if let Some(active) = current.rotation_current.as_ref() {
            if active.operation_id == request.operation_id
                && active.plan_digest == request.plan_digest
                && active.header == request.header
            {
                return Ok(OperationReceipt {
                    operation_id: request.operation_id,
                });
            }
            return Err(InternalError::conflict());
        }
        let registry_version = registry_version(&registry)?;
        let coordinator_policy = registry
            .root_funding
            .as_ref()
            .ok_or_else(InternalError::invariant)?;
        let expected_operation_id = fleet_funding_policy_rotation_operation_id(
            registry.authority.binding.coordinator,
            request.plan_digest,
        );
        let usage = funding_usage(
            current.historical_automatic_grants,
            current.historical_automatic_cycles.to_u128(),
            current.automatic_grants,
            current.automatic_cycles.to_u128(),
        );
        let all_roots_active = registry
            .registry
            .fleet_subnet_roots
            .iter()
            .all(|root| root.status == FleetSubnetRootStatus::Active);
        let retained_rotation_roots = rotation_history_root_count(&current)?;
        validate_coordinator_root_funding_policy(&request.header.proposed_coordinator_policy)
            .map_err(|_| InternalError::invalid_input())?;
        if request.operation_id != expected_operation_id
            || request.operation_id == [0; 32]
            || request.plan_digest == [0; 32]
            || request.header.predecessor_registry != registry_version
            || request.header.predecessor_generation != current.policy_generation
            || request.header.predecessor_generation.checked_add(1)
                != Some(request.header.successor_generation)
            || request.header.predecessor_coordinator_policy_hash
                != coordinator_root_funding_policy_hash(coordinator_policy)
            || request
                .header
                .proposed_coordinator_policy
                .budget
                .window_secs
                != coordinator_policy.budget.window_secs
            || request.header.predecessor_usage != usage
            || request.header.affected_root_count as usize
                != registry.registry.fleet_subnet_roots.len()
            || request.header.affected_root_count == 0
            || request.header.affected_root_count as usize > MAX_FLEET_FUNDING_POLICY_ROTATION_ROOTS
            || retained_rotation_roots
                .checked_add(request.header.affected_root_count as usize)
                .is_none_or(|count| count > MAX_FLEET_FUNDING_POLICY_ROTATION_HISTORY_ROOTS)
            || request.header.coordinator_placement.subnet
                != registry.authority.binding.coordinator_subnet
            || request.header.coordinator_placement.node_count == 0
            || request
                .header
                .coordinator_placement
                .cost_multiplier_numerator
                == 0
            || request
                .header
                .coordinator_placement
                .cost_multiplier_denominator
                == 0
            || request.header.coordinator_placement.fiduciary
                != request
                    .header
                    .coordinator_placement
                    .acknowledge_fiduciary_cost
            || request.header.maximum_new_automatic_cycles
                != request
                    .header
                    .proposed_coordinator_policy
                    .maximum_automatic_cycles
            || request.header.apply_operator_debit.to_u128() != 0
            || !all_roots_active
            || current.roots.iter().any(|root| root.current.is_some())
            || !retained_window_spend_fits(
                current.fleet_window.as_ref(),
                request
                    .header
                    .proposed_coordinator_policy
                    .budget
                    .maximum_cycles
                    .to_u128(),
            )
            || component_operation_in_progress(registry.component_provisioning.as_ref())
            || component_operation_in_progress(registry.component_scale_out.as_ref())
        {
            return Err(InternalError::conflict());
        }
        let mut next = current.clone();
        next.rotation_current = Some(FleetFundingPolicyRotationRecord {
            operation_id: request.operation_id,
            plan_digest: request.plan_digest,
            header: request.header,
            roots: Vec::new(),
            phase: FleetFundingPolicyRotationPhaseRecord::Staging,
            opened_at_ns: now_ns,
            updated_at_ns: now_ns,
        });
        commit_funding(&current, next)?;
        Ok(OperationReceipt {
            operation_id: request.operation_id,
        })
    }

    pub(crate) fn stage_funding_policy_rotation_root(
        request: FleetFundingPolicyRotationStageRootRequest,
        now_ns: u64,
    ) -> Result<OperationReceipt, InternalError> {
        let registry = Self::current()?;
        let current = raw_funding()?;
        if let Some(checkpoint) =
            completed_rotation_checkpoint(&current, request.operation_id, request.plan_digest)?
        {
            if !checkpoint.plan.roots.contains(&request.root) {
                return Err(InternalError::conflict());
            }
            return Ok(OperationReceipt {
                operation_id: request.operation_id,
            });
        }
        let active = current
            .rotation_current
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        if active.operation_id != request.operation_id
            || active.plan_digest != request.plan_digest
            || !matches!(active.phase, FleetFundingPolicyRotationPhaseRecord::Staging)
        {
            return Err(InternalError::conflict());
        }
        let registry_root = registry
            .registry
            .fleet_subnet_roots
            .iter()
            .find(|root| root.fleet_subnet_root == request.root.fleet_subnet_root)
            .ok_or_else(InternalError::forbidden)?;
        let ledger = current
            .roots
            .binary_search_by_key(&request.root.fleet_subnet_root, |root| {
                root.fleet_subnet_root
            })
            .ok()
            .map(|index| &current.roots[index]);
        let usage = funding_usage(
            ledger.map_or(0, |root| root.historical_automatic_grants),
            ledger.map_or(0, |root| root.historical_automatic_cycles.to_u128()),
            ledger.map_or(0, |root| root.automatic_grants),
            ledger.map_or(0, |root| root.automatic_cycles.to_u128()),
        );
        let mut proposed_authority = registry_root.funding.clone();
        proposed_authority.root_funding = request.root.proposed_policy.clone();
        validate_fleet_subnet_root_funding_authority(&proposed_authority, false)
            .map_err(|_| InternalError::invalid_input())?;
        if request.root.predecessor_policy_hash
            != fleet_subnet_root_funding_policy_hash(&registry_root.funding)
            || request.root.predecessor_usage != usage
            || request.root.placement.subnet != registry_root.placement_subnet
            || request.root.placement.node_count == 0
            || request.root.placement.cost_multiplier_numerator == 0
            || request.root.placement.cost_multiplier_denominator == 0
            || request.root.placement.fiduciary != request.root.placement.acknowledge_fiduciary_cost
            || request.root.proposed_policy.cooldown_secs
                != registry_root.funding.root_funding.cooldown_secs
            || request.root.proposed_policy.budget.window_secs
                != registry_root.funding.root_funding.budget.window_secs
            || !retained_window_spend_fits(
                ledger.and_then(|ledger| ledger.window.as_ref()),
                request.root.proposed_policy.budget.maximum_cycles.to_u128(),
            )
        {
            return Err(InternalError::conflict());
        }
        let mut next = current.clone();
        let rotation = next
            .rotation_current
            .as_mut()
            .ok_or_else(InternalError::invariant)?;
        match rotation
            .roots
            .binary_search_by_key(&request.root.fleet_subnet_root, |root| {
                root.fleet_subnet_root
            }) {
            Ok(index) if rotation.roots[index] == request.root => {}
            Ok(_) => return Err(InternalError::conflict()),
            Err(index) => {
                if rotation.roots.len() >= rotation.header.affected_root_count as usize {
                    return Err(InternalError::resource_exhausted());
                }
                rotation.roots.insert(index, request.root);
                rotation.updated_at_ns = now_ns;
                commit_funding(&current, next)?;
            }
        }
        Ok(OperationReceipt {
            operation_id: request.operation_id,
        })
    }

    pub(crate) fn apply_funding_policy_rotation(
        request: FleetFundingPolicyRotationApplyRequest,
        now_ns: u64,
    ) -> Result<OperationReceipt, InternalError> {
        let current = raw_funding()?;
        if let Some(checkpoint) =
            completed_rotation_checkpoint(&current, request.operation_id, request.plan_digest)?
        {
            if checkpoint.receipt.predecessor_generation != request.expected_predecessor_generation
            {
                return Err(InternalError::conflict());
            }
            return Ok(OperationReceipt {
                operation_id: request.operation_id,
            });
        }
        let active = current
            .rotation_current
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        if active.operation_id != request.operation_id
            || active.plan_digest != request.plan_digest
            || active.header.predecessor_generation != request.expected_predecessor_generation
        {
            return Err(InternalError::conflict());
        }
        if !matches!(active.phase, FleetFundingPolicyRotationPhaseRecord::Staging) {
            return Ok(OperationReceipt {
                operation_id: request.operation_id,
            });
        }
        let plan = FleetFundingPolicyRotationPlan {
            header: active.header.clone(),
            roots: active.roots.clone(),
        };
        if plan.roots.len() != plan.header.affected_root_count as usize
            || fleet_funding_policy_rotation_roots_digest(&plan.roots) != plan.header.roots_digest
            || fleet_funding_policy_rotation_plan_digest(&plan) != active.plan_digest
        {
            return Err(InternalError::conflict());
        }
        validate_fleet_funding_policy_rotation_plan(&plan)
            .map_err(|_| InternalError::invalid_input())?;
        let mut next = current.clone();
        let rotation = next
            .rotation_current
            .as_mut()
            .ok_or_else(InternalError::invariant)?;
        rotation.phase = FleetFundingPolicyRotationPhaseRecord::PreparingRoots {
            prepared: Vec::new(),
        };
        rotation.updated_at_ns = now_ns;
        commit_funding(&current, next)?;
        Ok(OperationReceipt {
            operation_id: request.operation_id,
        })
    }

    pub(crate) fn funding_policy_rotation_step()
    -> Result<FleetFundingPolicyRotationStep, InternalError> {
        let current = raw_funding()?;
        let rotation = current
            .rotation_current
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        match &rotation.phase {
            FleetFundingPolicyRotationPhaseRecord::Staging => Err(InternalError::conflict()),
            FleetFundingPolicyRotationPhaseRecord::PreparingRoots { prepared }
                if prepared.len() < rotation.roots.len() =>
            {
                let root = rotation.roots[prepared.len()].clone();
                Ok(FleetFundingPolicyRotationStep::PrepareRoot {
                    fleet_subnet_root: root.fleet_subnet_root,
                    request: FleetFundingPolicyRotationRootPrepareRequest {
                        operation_id: rotation.operation_id,
                        plan_digest: rotation.plan_digest,
                        predecessor_registry: rotation.header.predecessor_registry.clone(),
                        predecessor_generation: rotation.header.predecessor_generation,
                        successor_generation: rotation.header.successor_generation,
                        root,
                    },
                })
            }
            FleetFundingPolicyRotationPhaseRecord::PreparingRoots { .. } => {
                Ok(FleetFundingPolicyRotationStep::PublishRegistry)
            }
            FleetFundingPolicyRotationPhaseRecord::ActivatingRoots {
                successor_registry,
                activated,
                ..
            } if activated.len() < rotation.roots.len() => {
                let root = &rotation.roots[activated.len()];
                Ok(FleetFundingPolicyRotationStep::ActivateRoot {
                    fleet_subnet_root: root.fleet_subnet_root,
                    request: FleetFundingPolicyRotationRootActivateRequest {
                        operation_id: rotation.operation_id,
                        plan_digest: rotation.plan_digest,
                        predecessor_registry: rotation.header.predecessor_registry.clone(),
                        successor_registry: successor_registry.as_ref().clone(),
                        predecessor_generation: rotation.header.predecessor_generation,
                        successor_generation: rotation.header.successor_generation,
                        fleet_subnet_root: root.fleet_subnet_root,
                    },
                })
            }
            FleetFundingPolicyRotationPhaseRecord::ActivatingRoots { .. } => {
                Ok(FleetFundingPolicyRotationStep::Complete)
            }
        }
    }

    pub(crate) fn record_funding_policy_rotation_root_prepared(
        receipt: FleetFundingPolicyRotationRootReceipt,
        now_ns: u64,
    ) -> Result<(), InternalError> {
        let current = raw_funding()?;
        let current_rotation = current
            .rotation_current
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        let prepared_count = match &current_rotation.phase {
            FleetFundingPolicyRotationPhaseRecord::PreparingRoots { prepared } => prepared.len(),
            _ => return Err(InternalError::conflict()),
        };
        validate_root_receipt(current_rotation, prepared_count, &receipt, false)?;
        let mut next = current.clone();
        let rotation = next
            .rotation_current
            .as_mut()
            .ok_or_else(InternalError::conflict)?;
        let FleetFundingPolicyRotationPhaseRecord::PreparingRoots { prepared } =
            &mut rotation.phase
        else {
            return Err(InternalError::conflict());
        };
        prepared.push(receipt);
        rotation.updated_at_ns = now_ns;
        commit_funding(&current, next).map(|_| ())
    }

    pub(crate) fn publish_funding_policy_rotation_registry(
        now_ns: u64,
    ) -> Result<FleetRegistryVersion, InternalError> {
        let current_funding = raw_funding()?;
        let rotation = current_funding
            .rotation_current
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        let FleetFundingPolicyRotationPhaseRecord::PreparingRoots { prepared } = &rotation.phase
        else {
            return Err(InternalError::conflict());
        };
        if prepared.len() != rotation.roots.len() {
            return Err(InternalError::conflict());
        }

        let current_registry = Self::current()?;
        let current_version = registry_version(&current_registry)?;
        let successor_registry = if current_version == rotation.header.predecessor_registry {
            let mut next = current_registry.clone();
            next.root_funding = Some(rotation.header.proposed_coordinator_policy.clone());
            next.registry.revision = next
                .registry
                .revision
                .checked_add(1)
                .ok_or_else(InternalError::resource_exhausted)?;
            for root in &rotation.roots {
                let entry = next
                    .registry
                    .fleet_subnet_roots
                    .iter_mut()
                    .find(|entry| entry.fleet_subnet_root == root.fleet_subnet_root)
                    .ok_or_else(InternalError::invariant)?;
                entry.funding.root_funding = root.proposed_policy.clone();
            }
            let next = Self::validate_current(next)?;
            let version = registry_version(&next)?;
            let mut prospective_funding = current_funding.clone();
            activate_successor_funding_generation(&mut prospective_funding, &version)?;
            super::root_funding::validate_funding_record(&next, prospective_funding)?;
            FleetCoordinatorRegistryStore::commit_transition(&current_registry, next)
                .map_err(map_commit_error)?;
            version
        } else {
            validate_published_registry(rotation, &current_registry)?;
            current_version
        };

        let current_registry = Self::current()?;
        let mut next_funding = current_funding.clone();
        activate_successor_funding_generation(&mut next_funding, &successor_registry)?;
        let active = next_funding
            .rotation_current
            .as_mut()
            .ok_or_else(InternalError::invariant)?;
        active.updated_at_ns = now_ns;
        let next_funding =
            super::root_funding::validate_funding_record(&current_registry, next_funding)?;
        commit_funding(&current_funding, next_funding)?;
        Ok(successor_registry)
    }

    pub(crate) fn record_funding_policy_rotation_root_activated(
        receipt: FleetFundingPolicyRotationRootReceipt,
        now_ns: u64,
    ) -> Result<(), InternalError> {
        let current = raw_funding()?;
        let current_rotation = current
            .rotation_current
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        let activated_count = match &current_rotation.phase {
            FleetFundingPolicyRotationPhaseRecord::ActivatingRoots { activated, .. } => {
                activated.len()
            }
            _ => return Err(InternalError::conflict()),
        };
        validate_root_receipt(current_rotation, activated_count, &receipt, true)?;
        let mut next = current.clone();
        let rotation = next
            .rotation_current
            .as_mut()
            .ok_or_else(InternalError::conflict)?;
        let FleetFundingPolicyRotationPhaseRecord::ActivatingRoots { activated, .. } =
            &mut rotation.phase
        else {
            return Err(InternalError::conflict());
        };
        activated.push(receipt);
        rotation.updated_at_ns = now_ns;
        commit_funding(&current, next).map(|_| ())
    }

    pub(crate) fn complete_funding_policy_rotation(
        completed_at_ns: u64,
    ) -> Result<FleetFundingPolicyRotationReceipt, InternalError> {
        let current = raw_funding()?;
        let rotation = current
            .rotation_current
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        let FleetFundingPolicyRotationPhaseRecord::ActivatingRoots {
            successor_registry,
            activated,
            ..
        } = &rotation.phase
        else {
            return Err(InternalError::conflict());
        };
        if activated.len() != rotation.roots.len() {
            return Err(InternalError::conflict());
        }
        let registry = FleetCoordinatorRegistryStore::export()
            .current
            .ok_or_else(InternalError::unavailable)?;
        if &registry_version(&registry)? != successor_registry.as_ref() {
            return Err(InternalError::conflict());
        }
        let coordinator_policy = registry
            .root_funding
            .as_ref()
            .ok_or_else(InternalError::invariant)?;
        let successor_policy_set_hash = fleet_funding_policy_rotation_successor_policy_set_hash(
            coordinator_policy,
            registry
                .registry
                .fleet_subnet_roots
                .iter()
                .map(|root| (root.fleet_subnet_root, &root.funding)),
        );
        let receipt = FleetFundingPolicyRotationReceipt {
            operation_id: rotation.operation_id,
            plan_digest: rotation.plan_digest,
            predecessor_registry: rotation.header.predecessor_registry.clone(),
            successor_registry: successor_registry.as_ref().clone(),
            predecessor_generation: rotation.header.predecessor_generation,
            successor_generation: rotation.header.successor_generation,
            affected_root_count: rotation.header.affected_root_count,
            retained_historical_automatic_grants: current.historical_automatic_grants,
            retained_historical_automatic_cycles: current.historical_automatic_cycles.clone(),
            successor_policy_set_hash,
            maximum_new_automatic_cycles: rotation.header.maximum_new_automatic_cycles.clone(),
            apply_operator_debit: rotation.header.apply_operator_debit.clone(),
            completed_at_ns,
        };
        let checkpoint = FleetFundingPolicyRotationCheckpointRecord {
            receipt: receipt.clone(),
            plan: FleetFundingPolicyRotationPlan {
                header: rotation.header.clone(),
                roots: rotation.roots.clone(),
            },
            coordinator_policy: coordinator_policy.clone(),
            roots: registry
                .registry
                .fleet_subnet_roots
                .iter()
                .map(|root| FleetFundingPolicyRotationRootCheckpointRecord {
                    fleet_subnet_root: root.fleet_subnet_root,
                    funding: root.funding.clone(),
                })
                .collect(),
        };
        let mut next = current.clone();
        next.rotation_history.push(checkpoint);
        next.rotation_current = None;
        next.rotation_last = Some(receipt.clone());
        commit_funding(&current, next)?;
        Ok(receipt)
    }
}

fn component_operation_in_progress(record: Option<&FleetComponentProvisioningRecord>) -> bool {
    record.is_some_and(|record| {
        !matches!(
            &record.state,
            FleetComponentProvisioningStateRecord::RuntimesActivated { .. }
        )
    })
}

fn retained_window_spend_fits(
    window: Option<&FleetRootFundingWindowRecord>,
    successor_maximum_cycles: u128,
) -> bool {
    window.is_none_or(|window| window.spent_cycles.to_u128() <= successor_maximum_cycles)
}

fn completed_rotation_checkpoint(
    funding: &FleetCoordinatorFundingRecord,
    operation_id: [u8; 32],
    plan_digest: [u8; 32],
) -> Result<Option<&FleetFundingPolicyRotationCheckpointRecord>, InternalError> {
    let checkpoint = completed_rotation_checkpoint_by_operation(funding, operation_id)?;
    if checkpoint.is_some_and(|checkpoint| checkpoint.receipt.plan_digest != plan_digest) {
        return Err(InternalError::conflict());
    }
    Ok(checkpoint)
}

fn completed_rotation_checkpoint_by_operation(
    funding: &FleetCoordinatorFundingRecord,
    operation_id: [u8; 32],
) -> Result<Option<&FleetFundingPolicyRotationCheckpointRecord>, InternalError> {
    let mut checkpoints = funding
        .rotation_history
        .iter()
        .filter(|checkpoint| checkpoint.receipt.operation_id == operation_id);
    let checkpoint = checkpoints.next();
    if checkpoints.next().is_some() {
        return Err(InternalError::invariant());
    }
    Ok(checkpoint)
}

fn activate_successor_funding_generation(
    funding: &mut FleetCoordinatorFundingRecord,
    successor_registry: &FleetRegistryVersion,
) -> Result<(), InternalError> {
    let rotation = funding
        .rotation_current
        .as_ref()
        .ok_or_else(InternalError::invariant)?;
    let FleetFundingPolicyRotationPhaseRecord::PreparingRoots { prepared } = &rotation.phase else {
        return Err(InternalError::conflict());
    };
    funding.policy_generation = rotation.header.successor_generation;
    funding.historical_automatic_grants = funding
        .historical_automatic_grants
        .checked_add(u64::from(funding.automatic_grants))
        .ok_or_else(InternalError::invariant)?;
    funding.historical_automatic_cycles = funding
        .historical_automatic_cycles
        .to_u128()
        .checked_add(funding.automatic_cycles.to_u128())
        .ok_or_else(InternalError::invariant)?
        .into();
    funding.automatic_grants = 0;
    funding.automatic_cycles = 0_u128.into();
    for root in &mut funding.roots {
        root.historical_automatic_grants = root
            .historical_automatic_grants
            .checked_add(u64::from(root.automatic_grants))
            .ok_or_else(InternalError::invariant)?;
        root.historical_automatic_cycles = root
            .historical_automatic_cycles
            .to_u128()
            .checked_add(root.automatic_cycles.to_u128())
            .ok_or_else(InternalError::invariant)?
            .into();
        root.automatic_grants = 0;
        root.automatic_cycles = 0_u128.into();
    }
    funding.rotation_current = Some(FleetFundingPolicyRotationRecord {
        operation_id: rotation.operation_id,
        plan_digest: rotation.plan_digest,
        header: rotation.header.clone(),
        roots: rotation.roots.clone(),
        phase: FleetFundingPolicyRotationPhaseRecord::ActivatingRoots {
            successor_registry: Box::new(successor_registry.clone()),
            prepared: prepared.clone(),
            activated: Vec::new(),
        },
        opened_at_ns: rotation.opened_at_ns,
        updated_at_ns: rotation.updated_at_ns,
    });
    Ok(())
}

fn validate_published_registry(
    rotation: &FleetFundingPolicyRotationRecord,
    registry: &crate::storage::stable::fleet_coordinator::FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    if registry.root_funding.as_ref() != Some(&rotation.header.proposed_coordinator_policy)
        || registry.registry.revision
            != rotation
                .header
                .predecessor_registry
                .revision
                .checked_add(1)
                .ok_or_else(InternalError::resource_exhausted)?
    {
        return Err(InternalError::conflict());
    }
    for root in &rotation.roots {
        let entry = registry
            .registry
            .fleet_subnet_roots
            .iter()
            .find(|entry| entry.fleet_subnet_root == root.fleet_subnet_root)
            .ok_or_else(InternalError::conflict)?;
        if entry.funding.root_funding != root.proposed_policy {
            return Err(InternalError::conflict());
        }
    }
    Ok(())
}

fn validate_root_receipt(
    rotation: &FleetFundingPolicyRotationRecord,
    receipt_index: usize,
    receipt: &FleetFundingPolicyRotationRootReceipt,
    activated: bool,
) -> Result<(), InternalError> {
    let expected = rotation
        .roots
        .get(receipt_index)
        .ok_or_else(InternalError::conflict)?;
    if receipt.operation_id != rotation.operation_id
        || receipt.plan_digest != rotation.plan_digest
        || receipt.fleet_subnet_root != expected.fleet_subnet_root
        || receipt.predecessor_generation != rotation.header.predecessor_generation
        || receipt.successor_generation != rotation.header.successor_generation
        || !receipt.prepared
        || receipt.activated != activated
    {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn funding_usage(
    historical_grants: u64,
    historical_cycles: u128,
    generation_grants: u32,
    generation_cycles: u128,
) -> FleetFundingPolicyUsage {
    FleetFundingPolicyUsage {
        historical_automatic_grants: historical_grants,
        historical_automatic_cycles: historical_cycles.into(),
        generation_automatic_grants: generation_grants,
        generation_automatic_cycles: generation_cycles.into(),
    }
}

fn rotation_root_count(length: usize) -> Result<u32, InternalError> {
    u32::try_from(length).map_err(|_| InternalError::invariant())
}

fn rotation_history_root_count(
    funding: &FleetCoordinatorFundingRecord,
) -> Result<usize, InternalError> {
    funding
        .rotation_history
        .iter()
        .try_fold(0_usize, |total, checkpoint| {
            total
                .checked_add(checkpoint.roots.len())
                .ok_or_else(InternalError::resource_exhausted)
        })
}

fn registry_version(
    registry: &crate::storage::stable::fleet_coordinator::FleetCoordinatorRegistryRecord,
) -> Result<FleetRegistryVersion, InternalError> {
    FleetRegistryOps::version(
        &registry.authority,
        &registry
            .component_deployment_configuration
            .component_topology,
        &registry.registry,
    )
}

fn raw_funding() -> Result<FleetCoordinatorFundingRecord, InternalError> {
    FleetCoordinatorFundingStore::export()
        .current
        .ok_or_else(InternalError::unavailable)
}

fn commit_funding(
    current: &FleetCoordinatorFundingRecord,
    next: FleetCoordinatorFundingRecord,
) -> Result<crate::storage::stable::fleet_coordinator::FleetCoordinatorCommitOutcome, InternalError>
{
    let registry = FleetCoordinatorOps::current()?;
    let next = super::root_funding::validate_funding_record(&registry, next)?;
    FleetCoordinatorFundingStore::commit_transition(current, next).map_err(map_commit_error)
}

const fn map_commit_error(error: FleetCoordinatorCommitError) -> InternalError {
    match error {
        FleetCoordinatorCommitError::ConflictingState => InternalError::conflict(),
        FleetCoordinatorCommitError::Uninitialized => InternalError::unavailable(),
    }
}

#[cfg(test)]
mod tests {
    use super::retained_window_spend_fits;
    use crate::storage::stable::fleet_coordinator::FleetRootFundingWindowRecord;
    use canic_core::cdk::types::Cycles;

    #[test]
    fn policy_rotation_successor_window_must_preserve_the_exact_retained_spend() {
        let window = FleetRootFundingWindowRecord {
            window_start_secs: 90,
            spent_cycles: Cycles::new(30),
        };

        assert!(retained_window_spend_fits(None, 0));
        assert!(retained_window_spend_fits(Some(&window), 30));
        assert!(!retained_window_spend_fits(Some(&window), 29));
    }
}
