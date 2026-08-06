//! Module: ops::component_provisioning
//!
//! Responsibility: commit and read exact root-local Component Group provisioning authority.
//! Does not own: caller authentication, Store observation, Component effects, or orchestration.
//! Boundary: workflow supplies a validated batch; ops derives immutable member context only from
//! that durable record.

#[cfg(test)]
mod tests;

use crate::{
    storage::stable::component_provisioning::{
        RootComponentProvisioningCommitError, RootComponentProvisioningPlacementKey,
        RootComponentProvisioningPlacementRecord, RootComponentProvisioningRecord,
        RootComponentProvisioningStateRecordPhase, RootComponentProvisioningStore,
    },
    view::component_provisioning::RootComponentProvisioningView,
};
use candid::CandidType;
use canic_core::{
    control_plane_support::{
        error::{InternalError, InternalErrorOrigin},
        ops::component_provisioning_plan::RootComponentProvisioningBatchValidation,
    },
    dto::{
        component_provisioning::{
            RootComponentProvisioningAcceptanceRequest, RootComponentProvisioningPhase,
            RootComponentProvisioningStatusRequest, RootComponentProvisioningStatusResponse,
        },
        component_registry::ComponentProvisioningOrigin,
        fleet_registry::FleetRegistryVersion,
    },
    ids::{
        ComponentDeploymentConfigurationDigest, ComponentGroupMemberPath,
        ComponentGroupPlacementId, ComponentSpecId,
    },
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const ACCEPTANCE_RECEIPT_DOMAIN: &[u8] = b"canic/root-component-provisioning-acceptance-receipt/v1";

#[derive(CandidType)]
struct RootComponentProvisioningAcceptanceReceiptAuthority<'a> {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    fleet_registry: &'a FleetRegistryVersion,
    configuration_digest: ComponentDeploymentConfigurationDigest,
    batch: &'a canic_core::dto::component_provisioning::FleetSubnetRootProvisioningBatch,
    placement_count: u32,
    component_count: u32,
    accepted_at_ns: u64,
}

/// Stable root-local Component Group provisioning operations.
pub struct RootComponentProvisioningOps;

impl RootComponentProvisioningOps {
    /// Return an exact durable acceptance replay before consulting mutable live prerequisites.
    pub(crate) fn acceptance_replay(
        request: &RootComponentProvisioningAcceptanceRequest,
    ) -> Result<Option<RootComponentProvisioningView>, InternalError> {
        validate_operation_and_plan_hash(request.operation_id, request.plan_hash)?;
        let Some(record) = RootComponentProvisioningStore::operation(request.operation_id) else {
            return Ok(None);
        };
        let view = validated_record(record)?;
        if !request_matches_view(request, &view) {
            return Err(InternalError::conflict(
                "root Component provisioning operation is already bound to different intent",
            ));
        }
        Ok(Some(view))
    }

    /// Commit one exact already-validated batch or replay its original receipt.
    pub(crate) fn accept(
        request: RootComponentProvisioningAcceptanceRequest,
        validation: &RootComponentProvisioningBatchValidation,
        accepted_at_ns: u64,
    ) -> Result<RootComponentProvisioningView, InternalError> {
        validate_acceptance_identity(&request, validation, accepted_at_ns)?;
        if let Some(existing) = RootComponentProvisioningStore::operation(request.operation_id) {
            let view = validated_record(existing)?;
            return if request_matches_view(&request, &view) {
                Ok(view)
            } else {
                Err(InternalError::conflict(
                    "root Component provisioning operation is already bound to different intent",
                ))
            };
        }

        let current = validated_aggregate_state()?;
        let next_placements = current
            .tracked_group_placements
            .checked_add(validation.placement_count)
            .ok_or_else(|| {
                InternalError::resource_exhausted(
                    "root Component Group placement accounting overflowed",
                )
            })?;
        if next_placements > request.batch.root.limits.maximum_group_placements {
            return Err(InternalError::resource_exhausted(format!(
                "root Component Group placement reservation requires {next_placements}, exceeding protected limit {}",
                request.batch.root.limits.maximum_group_placements
            )));
        }
        for placement in &request.batch.placements {
            let key = RootComponentProvisioningPlacementKey::from(&placement.group_placement);
            if RootComponentProvisioningStore::placement(&key).is_some() {
                return Err(InternalError::conflict(format!(
                    "Component Group placement '{:?}' is already reserved",
                    placement.group_placement
                )));
            }
        }

        let receipt_content_hash = acceptance_receipt_hash(
            &request,
            validation.placement_count,
            validation.component_count,
            accepted_at_ns,
        )?;
        let record = RootComponentProvisioningRecord {
            operation_id: request.operation_id,
            plan_hash: request.plan_hash,
            fleet_registry: request.fleet_registry,
            configuration_digest: request.configuration_digest,
            batch: request.batch,
            state: RootComponentProvisioningStateRecordPhase::Accepted {
                placement_count: validation.placement_count,
                component_count: validation.component_count,
                accepted_at_ns,
                receipt_content_hash,
            },
        };
        RootComponentProvisioningStore::accept(record.clone()).map_err(map_commit_error)?;
        validated_record(record)
    }

    /// Read one exact accepted operation without mutation.
    pub(crate) fn status(
        request: RootComponentProvisioningStatusRequest,
    ) -> Result<RootComponentProvisioningView, InternalError> {
        validate_operation_and_plan_hash(request.operation_id, request.plan_hash)?;
        let record =
            RootComponentProvisioningStore::operation(request.operation_id).ok_or_else(|| {
                InternalError::unavailable("root Component provisioning operation is not accepted")
            })?;
        let view = validated_record(record)?;
        if view.plan_hash != request.plan_hash {
            return Err(InternalError::conflict(
                "root Component provisioning status names a different plan",
            ));
        }
        Ok(view)
    }

    /// Number of distinct accepted or committed placements occupying the root ceiling.
    pub(crate) fn tracked_group_placements() -> Result<u32, InternalError> {
        Ok(validated_aggregate_state()?.tracked_group_placements)
    }

    /// Fence unrelated top-level allocations while one aggregate batch owns root capacity.
    pub(crate) fn require_ordinary_allocation_open() -> Result<(), InternalError> {
        let state = validated_aggregate_state()?;
        if state.active_operation_id.is_some() {
            return Err(InternalError::conflict(
                "root Component provisioning batch owns top-level allocation capacity",
            ));
        }
        Ok(())
    }

    /// Reject a different active aggregate operation before any fresh acceptance observation.
    pub(crate) fn require_acceptance_open(operation_id: [u8; 32]) -> Result<(), InternalError> {
        let state = validated_aggregate_state()?;
        match state.active_operation_id {
            None => Ok(()),
            Some(active) if active != operation_id => Err(InternalError::conflict(
                "root already has a different active Component provisioning operation",
            )),
            Some(_) => Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "active root Component provisioning operation has no replayable record",
            )),
        }
    }

    /// Keep a root with retained group placements out of ordinary root draining.
    pub(crate) fn require_root_draining_open() -> Result<(), InternalError> {
        let state = validated_aggregate_state()?;
        if state.active_operation_id.is_some() || state.tracked_group_placements != 0 {
            return Err(InternalError::conflict(
                "root retains Component Group provisioning authority",
            ));
        }
        Ok(())
    }

    /// Revalidate one retained group origin against its immutable accepted member authority.
    pub(crate) fn validate_member_provisioning_origin(
        origin: &ComponentProvisioningOrigin,
        component_spec: &ComponentSpecId,
        spec_hash: [u8; 32],
    ) -> Result<(), InternalError> {
        let ComponentProvisioningOrigin::ComponentGroup {
            operation_id,
            plan_hash,
            group_placement,
            member_path,
        } = origin
        else {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Ops,
                "group provisioning validation received a non-group origin",
            ));
        };
        let view = Self::status(RootComponentProvisioningStatusRequest {
            operation_id: *operation_id,
            plan_hash: *plan_hash,
        })?;
        let (_placement, entry) = accepted_member(&view, group_placement, member_path)?;
        if &entry.component_spec != component_spec || entry.spec_hash != spec_hash {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "stored Component Group origin differs from its accepted member authority",
            ));
        }
        Ok(())
    }
}

fn validate_acceptance_identity(
    request: &RootComponentProvisioningAcceptanceRequest,
    validation: &RootComponentProvisioningBatchValidation,
    accepted_at_ns: u64,
) -> Result<(), InternalError> {
    validate_operation_and_plan_hash(request.operation_id, request.plan_hash)?;
    if accepted_at_ns == 0 {
        return Err(InternalError::invalid_input(
            "root Component provisioning acceptance time must be positive",
        ));
    }
    let placement_count = u32::try_from(request.batch.placements.len()).map_err(|_| {
        InternalError::resource_exhausted("root Component provisioning placement count exceeds u32")
    })?;
    let component_count = request
        .batch
        .placements
        .iter()
        .try_fold(0_u32, |total, placement| {
            total
                .checked_add(u32::try_from(placement.entries.len()).map_err(|_| {
                    InternalError::resource_exhausted(
                        "root Component provisioning member count exceeds u32",
                    )
                })?)
                .ok_or_else(|| {
                    InternalError::resource_exhausted(
                        "root Component provisioning member count overflowed",
                    )
                })
        })?;
    if placement_count != validation.placement_count
        || component_count != validation.component_count
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Ops,
            "root Component provisioning validation facts differ from the accepted batch",
        ));
    }
    Ok(())
}

fn validate_operation_and_plan_hash(
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
) -> Result<(), InternalError> {
    if operation_id == [0; 32] {
        return Err(InternalError::invalid_input(
            "root Component provisioning operation ID must be nonzero",
        ));
    }
    if plan_hash == [0; 32] {
        return Err(InternalError::invalid_input(
            "root Component provisioning plan hash must be nonzero",
        ));
    }
    Ok(())
}

fn validated_aggregate_state() -> Result<
    crate::storage::stable::component_provisioning::RootComponentProvisioningStateRecord,
    InternalError,
> {
    let state = RootComponentProvisioningStore::state();
    if u64::from(state.tracked_group_placements)
        != RootComponentProvisioningStore::placement_count()
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root Component provisioning placement capacity is inconsistent",
        ));
    }
    Ok(state)
}

fn validated_record(
    record: RootComponentProvisioningRecord,
) -> Result<RootComponentProvisioningView, InternalError> {
    validate_operation_and_plan_hash(record.operation_id, record.plan_hash)?;
    let RootComponentProvisioningStateRecordPhase::Accepted {
        placement_count,
        component_count,
        accepted_at_ns,
        receipt_content_hash,
    } = record.state;
    let validation = RootComponentProvisioningBatchValidation {
        placement_count,
        component_count,
        component_spec_counts: BTreeMap::default(),
        component_roles: BTreeSet::default(),
    };
    let request = RootComponentProvisioningAcceptanceRequest {
        fleet_registry: record.fleet_registry.clone(),
        configuration_digest: record.configuration_digest,
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        batch: record.batch.clone(),
    };
    validate_acceptance_identity(&request, &validation, accepted_at_ns)?;
    let expected_hash =
        acceptance_receipt_hash(&request, placement_count, component_count, accepted_at_ns)?;
    if receipt_content_hash != expected_hash {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root Component provisioning acceptance receipt hash is invalid",
        ));
    }
    let expected_placement = RootComponentProvisioningPlacementRecord {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
    };
    for placement in &record.batch.placements {
        let key = RootComponentProvisioningPlacementKey::from(&placement.group_placement);
        if RootComponentProvisioningStore::placement(&key) != Some(expected_placement) {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "accepted root Component provisioning placement index is inconsistent",
            ));
        }
    }
    let state = validated_aggregate_state()?;
    if state.active_operation_id != Some(record.operation_id) {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "accepted root Component provisioning aggregate state is inconsistent",
        ));
    }
    Ok(RootComponentProvisioningView {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        fleet_registry: record.fleet_registry,
        configuration_digest: record.configuration_digest,
        batch: record.batch,
        placement_count,
        component_count,
        accepted_at_ns,
        receipt_content_hash,
    })
}

fn request_matches_view(
    request: &RootComponentProvisioningAcceptanceRequest,
    view: &RootComponentProvisioningView,
) -> bool {
    request.operation_id == view.operation_id
        && request.plan_hash == view.plan_hash
        && request.fleet_registry == view.fleet_registry
        && request.configuration_digest == view.configuration_digest
        && request.batch == view.batch
}

fn accepted_member<'a>(
    view: &'a RootComponentProvisioningView,
    group_placement: &ComponentGroupPlacementId,
    member_path: &ComponentGroupMemberPath,
) -> Result<
    (
        &'a canic_core::dto::component_provisioning::ComponentGroupPlacementPlan,
        &'a canic_core::dto::component_provisioning::ComponentGroupPlanEntry,
    ),
    InternalError,
> {
    let placement = view
        .batch
        .placements
        .binary_search_by(|candidate| candidate.group_placement.cmp(group_placement))
        .ok()
        .map(|index| &view.batch.placements[index])
        .ok_or_else(|| {
            InternalError::conflict("Component Group placement is absent from accepted root batch")
        })?;
    let entry = placement
        .entries
        .binary_search_by(|candidate| candidate.member_path.cmp(member_path))
        .ok()
        .map(|index| &placement.entries[index])
        .ok_or_else(|| {
            InternalError::conflict("Component Group member is absent from accepted root batch")
        })?;
    Ok((placement, entry))
}

fn acceptance_receipt_hash(
    request: &RootComponentProvisioningAcceptanceRequest,
    placement_count: u32,
    component_count: u32,
    accepted_at_ns: u64,
) -> Result<[u8; 32], InternalError> {
    let authority = RootComponentProvisioningAcceptanceReceiptAuthority {
        operation_id: request.operation_id,
        plan_hash: request.plan_hash,
        fleet_registry: &request.fleet_registry,
        configuration_digest: request.configuration_digest,
        batch: &request.batch,
        placement_count,
        component_count,
        accepted_at_ns,
    };
    let bytes = candid::encode_one(authority).map_err(|error| {
        InternalError::invariant(
            InternalErrorOrigin::Ops,
            format!("could not encode root provisioning acceptance receipt: {error}"),
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(ACCEPTANCE_RECEIPT_DOMAIN);
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    Ok(hasher.finalize().into())
}

fn map_commit_error(error: RootComponentProvisioningCommitError) -> InternalError {
    match error {
        RootComponentProvisioningCommitError::ActiveOperationConflict => InternalError::conflict(
            "root already has a different active Component provisioning operation",
        ),
        RootComponentProvisioningCommitError::ConflictingOperation => InternalError::conflict(
            "root Component provisioning operation changed before acceptance committed",
        ),
        RootComponentProvisioningCommitError::PlacementConflict => {
            InternalError::conflict("root Component provisioning placement is already reserved")
        }
        RootComponentProvisioningCommitError::PlacementCountOverflow => {
            InternalError::resource_exhausted(
                "root Component Group placement accounting overflowed",
            )
        }
    }
}

/// Convert one validated durable view to its compact boundary receipt.
pub fn status_response(
    view: RootComponentProvisioningView,
) -> RootComponentProvisioningStatusResponse {
    RootComponentProvisioningStatusResponse {
        operation_id: view.operation_id,
        plan_hash: view.plan_hash,
        fleet_registry: view.fleet_registry,
        configuration_digest: view.configuration_digest,
        fleet_subnet_root: view.batch.root.fleet_subnet_root,
        phase: RootComponentProvisioningPhase::Accepted,
        placement_count: view.placement_count,
        component_count: view.component_count,
        accepted_at_ns: view.accepted_at_ns,
        receipt_content_hash: view.receipt_content_hash,
    }
}
