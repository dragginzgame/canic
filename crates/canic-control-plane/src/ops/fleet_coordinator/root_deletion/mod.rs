//! Module: ops::fleet_coordinator::root_deletion
//!
//! Responsibility: validate and locate the Coordinator's durable root-deletion authority.
//! Does not own: endpoint authorization, physical deletion effects, or Registry removal.
//! Boundary: the parent Coordinator ops owner supplies and commits canonical records.

use super::{
    FleetCoordinatorOps, FleetCoordinatorRegistryRecord,
    FleetSubnetRootRemovalPublicationReceiptRecord, receipt_invariant,
};
use candid::{CandidType, Principal};
use canic_core::{
    control_plane_support::error::{InternalError, InternalErrorOrigin},
    dto::{
        fleet_registry::{
            FleetSubnetRootDeletionCompletionRequest, FleetSubnetRootDeletionExecutionRequest,
            FleetSubnetRootDeletionExecutionResponse,
            FleetSubnetRootDeletionReadinessIntentRequest,
            FleetSubnetRootDeletionReadinessIntentResponse,
            FleetSubnetRootDeletionReadinessRequest, FleetSubnetRootDeletionReadinessResponse,
            FleetSubnetRootDeletionResponse, FleetSubnetRootDeletionStatusRequest,
            FleetSubnetRootStatus,
        },
        fleet_subnet_root::FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES,
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

impl FleetCoordinatorOps {
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

pub(super) fn validate_root_deletion_history(
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
