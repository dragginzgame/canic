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
        RootComponentProvisioningReservationCursorRecord,
        RootComponentProvisioningStateRecordPhase, RootComponentProvisioningStore,
    },
    view::{
        component_provisioning::{
            RootComponentMemberReservationView, RootComponentProvisioningReservationCursorView,
            RootComponentProvisioningReservationDisposition, RootComponentProvisioningView,
        },
        component_registry::{RootComponentAllocationProgressView, RootComponentAllocationView},
    },
};
use candid::{CandidType, Principal};
use canic_core::{
    control_plane_support::{
        error::{InternalError, InternalErrorOrigin},
        ops::component_provisioning_plan::RootComponentProvisioningBatchValidation,
    },
    dto::{
        component_provisioning::{
            RootComponentProvisioningAcceptanceRequest, RootComponentProvisioningAdvanceRequest,
            RootComponentProvisioningPhase, RootComponentProvisioningStatusRequest,
            RootComponentProvisioningStatusResponse,
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
const MEMBER_OPERATION_DOMAIN: &[u8] = b"canic/root-component-provisioning-member-operation/v1";
const RESERVATION_CURSOR_DOMAIN: &[u8] = b"canic/root-component-provisioning-reservation-cursor/v1";

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

#[derive(CandidType)]
struct RootComponentProvisioningMemberOperationAuthority<'a> {
    fleet_subnet_root: Principal,
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    group_placement: &'a ComponentGroupPlacementId,
    member_path: &'a ComponentGroupMemberPath,
}

#[derive(CandidType)]
struct RootComponentProvisioningReservationCursorAuthority {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    placement_index: u32,
    member_index: u32,
    reserved_component_count: u32,
}

#[derive(Eq, PartialEq)]
struct ReservedMemberAuthority<'a> {
    member_operation_id: [u8; 32],
    component_spec: &'a ComponentSpecId,
    spec_hash: [u8; 32],
    provisioning_origin: &'a ComponentProvisioningOrigin,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
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
        let reservation_cursor =
            reservation_cursor_record(request.operation_id, request.plan_hash, 0, 0, 0)?;
        let record = RootComponentProvisioningRecord {
            operation_id: request.operation_id,
            plan_hash: request.plan_hash,
            fleet_registry: request.fleet_registry,
            configuration_digest: request.configuration_digest,
            batch: request.batch,
            state: RootComponentProvisioningStateRecordPhase::Accepted {
                placement_count: validation.placement_count,
                component_count: validation.component_count,
                reservation_cursor,
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

    /// Interpret an expected reservation cursor without allowing a retry to skip work.
    pub(crate) fn reservation_disposition(
        request: RootComponentProvisioningAdvanceRequest,
        view: &RootComponentProvisioningView,
    ) -> Result<RootComponentProvisioningReservationDisposition, InternalError> {
        validate_operation_and_plan_hash(request.operation_id, request.plan_hash)?;
        if request.operation_id != view.operation_id || request.plan_hash != view.plan_hash {
            return Err(InternalError::conflict(
                "root Component provisioning advance request names different authority",
            ));
        }
        let current = view.reservation_cursor.reserved_component_count;
        if request.expected_reserved_component_count == current {
            return if current == view.component_count {
                Ok(RootComponentProvisioningReservationDisposition::Complete)
            } else {
                Ok(RootComponentProvisioningReservationDisposition::Advance)
            };
        }
        if request.expected_reserved_component_count.checked_add(1) == Some(current) {
            return Ok(RootComponentProvisioningReservationDisposition::Replay);
        }
        Err(InternalError::conflict(
            "root Component provisioning reservation cursor differs from expected progress",
        ))
    }

    /// Select the next member in O(1) from the hash-bound canonical cursor.
    pub(crate) fn next_member_reservation(
        view: &RootComponentProvisioningView,
    ) -> Result<RootComponentMemberReservationView, InternalError> {
        if view.reservation_cursor.reserved_component_count >= view.component_count {
            return Err(InternalError::conflict(
                "root Component provisioning has no unreserved member",
            ));
        }
        let placement = view
            .batch
            .placements
            .get(
                usize::try_from(view.reservation_cursor.placement_index).map_err(|_| {
                    InternalError::invariant(
                        InternalErrorOrigin::Storage,
                        "root Component provisioning placement cursor exceeds usize",
                    )
                })?,
            )
            .ok_or_else(|| {
                InternalError::invariant(
                    InternalErrorOrigin::Storage,
                    "root Component provisioning placement cursor is out of bounds",
                )
            })?;
        let entry = placement
            .entries
            .get(
                usize::try_from(view.reservation_cursor.member_index).map_err(|_| {
                    InternalError::invariant(
                        InternalErrorOrigin::Storage,
                        "root Component provisioning member cursor exceeds usize",
                    )
                })?,
            )
            .ok_or_else(|| {
                InternalError::invariant(
                    InternalErrorOrigin::Storage,
                    "root Component provisioning member cursor is out of bounds",
                )
            })?;
        Ok(RootComponentMemberReservationView {
            member_operation_id: member_operation_id(
                view.batch.root.fleet_subnet_root,
                view.operation_id,
                view.plan_hash,
                &placement.group_placement,
                &entry.member_path,
            )?,
            group_placement: placement.group_placement.clone(),
            member_path: entry.member_path.clone(),
            component_spec: entry.component_spec.clone(),
            spec_hash: entry.spec_hash,
        })
    }

    /// Commit one exact reconciled Component identity reservation to the aggregate cursor.
    pub(crate) fn mark_member_reserved(
        request: RootComponentProvisioningAdvanceRequest,
        allocation: &RootComponentAllocationView,
    ) -> Result<RootComponentProvisioningView, InternalError> {
        let current_record = RootComponentProvisioningStore::operation(request.operation_id)
            .ok_or_else(|| {
                InternalError::unavailable("root Component provisioning operation is not accepted")
            })?;
        let current = validated_record(current_record.clone())?;
        if Self::reservation_disposition(request, &current)?
            != RootComponentProvisioningReservationDisposition::Advance
        {
            return Err(InternalError::conflict(
                "root Component provisioning reservation step is already committed",
            ));
        }
        let member = Self::next_member_reservation(&current)?;
        validate_reserved_member(&current, &member, allocation)?;
        let next_cursor = advance_reservation_cursor(&current)?;
        let mut next_record = current_record.clone();
        let RootComponentProvisioningStateRecordPhase::Accepted {
            placement_count,
            component_count,
            accepted_at_ns,
            receipt_content_hash,
            ..
        } = next_record.state;
        next_record.state = RootComponentProvisioningStateRecordPhase::Accepted {
            placement_count,
            component_count,
            reservation_cursor: next_cursor,
            accepted_at_ns,
            receipt_content_hash,
        };
        RootComponentProvisioningStore::replace_operation(&current_record, next_record.clone())
            .map_err(map_commit_error)?;
        validated_record(next_record)
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
        reservation_cursor,
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
    validate_reservation_cursor(
        record.operation_id,
        record.plan_hash,
        &record.batch,
        component_count,
        reservation_cursor,
    )?;
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
        reservation_cursor: RootComponentProvisioningReservationCursorView {
            placement_index: reservation_cursor.placement_index,
            member_index: reservation_cursor.member_index,
            reserved_component_count: reservation_cursor.reserved_component_count,
        },
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

fn validate_reservation_cursor(
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    batch: &canic_core::dto::component_provisioning::FleetSubnetRootProvisioningBatch,
    component_count: u32,
    cursor: RootComponentProvisioningReservationCursorRecord,
) -> Result<(), InternalError> {
    let expected = reservation_cursor_record(
        operation_id,
        plan_hash,
        cursor.placement_index,
        cursor.member_index,
        cursor.reserved_component_count,
    )?;
    if cursor.content_hash != expected.content_hash
        || cursor.reserved_component_count > component_count
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root Component provisioning reservation cursor is invalid",
        ));
    }
    let placement_count = u32::try_from(batch.placements.len()).map_err(|_| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root Component provisioning placement count exceeds u32",
        )
    })?;
    if cursor.reserved_component_count == component_count {
        if cursor.placement_index != placement_count || cursor.member_index != 0 {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "terminal root Component provisioning cursor is not canonical",
            ));
        }
        return Ok(());
    }
    let placement = batch
        .placements
        .get(usize::try_from(cursor.placement_index).map_err(|_| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "root Component provisioning placement cursor exceeds usize",
            )
        })?)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "root Component provisioning placement cursor is out of bounds",
            )
        })?;
    if usize::try_from(cursor.member_index)
        .ok()
        .is_none_or(|index| index >= placement.entries.len())
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root Component provisioning member cursor is out of bounds",
        ));
    }
    Ok(())
}

fn advance_reservation_cursor(
    view: &RootComponentProvisioningView,
) -> Result<RootComponentProvisioningReservationCursorRecord, InternalError> {
    let placement_index =
        usize::try_from(view.reservation_cursor.placement_index).map_err(|_| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "root Component provisioning placement cursor exceeds usize",
            )
        })?;
    let placement = view.batch.placements.get(placement_index).ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root Component provisioning placement cursor is out of bounds",
        )
    })?;
    let next_reserved = view
        .reservation_cursor
        .reserved_component_count
        .checked_add(1)
        .ok_or_else(|| {
            InternalError::resource_exhausted(
                "root Component provisioning reservation count overflowed",
            )
        })?;
    let next_member = view
        .reservation_cursor
        .member_index
        .checked_add(1)
        .ok_or_else(|| {
            InternalError::resource_exhausted(
                "root Component provisioning member cursor overflowed",
            )
        })?;
    let entry_count = u32::try_from(placement.entries.len()).map_err(|_| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root Component provisioning placement member count exceeds u32",
        )
    })?;
    let (next_placement, next_member) = if next_member == entry_count {
        (
            view.reservation_cursor
                .placement_index
                .checked_add(1)
                .ok_or_else(|| {
                    InternalError::resource_exhausted(
                        "root Component provisioning placement cursor overflowed",
                    )
                })?,
            0,
        )
    } else {
        (view.reservation_cursor.placement_index, next_member)
    };
    reservation_cursor_record(
        view.operation_id,
        view.plan_hash,
        next_placement,
        next_member,
        next_reserved,
    )
}

fn reservation_cursor_record(
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    placement_index: u32,
    member_index: u32,
    reserved_component_count: u32,
) -> Result<RootComponentProvisioningReservationCursorRecord, InternalError> {
    let authority = RootComponentProvisioningReservationCursorAuthority {
        operation_id,
        plan_hash,
        placement_index,
        member_index,
        reserved_component_count,
    };
    Ok(RootComponentProvisioningReservationCursorRecord {
        placement_index,
        member_index,
        reserved_component_count,
        content_hash: domain_separated_candid_hash(RESERVATION_CURSOR_DOMAIN, authority)?,
    })
}

fn member_operation_id(
    fleet_subnet_root: Principal,
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    group_placement: &ComponentGroupPlacementId,
    member_path: &ComponentGroupMemberPath,
) -> Result<[u8; 32], InternalError> {
    domain_separated_candid_hash(
        MEMBER_OPERATION_DOMAIN,
        RootComponentProvisioningMemberOperationAuthority {
            fleet_subnet_root,
            operation_id,
            plan_hash,
            group_placement,
            member_path,
        },
    )
}

fn validate_reserved_member(
    view: &RootComponentProvisioningView,
    member: &RootComponentMemberReservationView,
    allocation: &RootComponentAllocationView,
) -> Result<(), InternalError> {
    let expected_origin = ComponentProvisioningOrigin::ComponentGroup {
        operation_id: view.operation_id,
        plan_hash: view.plan_hash,
        group_placement: member.group_placement.clone(),
        member_path: member.member_path.clone(),
    };
    let expected = ReservedMemberAuthority {
        member_operation_id: member.member_operation_id,
        component_spec: &member.component_spec,
        spec_hash: member.spec_hash,
        provisioning_origin: &expected_origin,
        release_set: view.batch.active_release_set,
    };
    let actual = ReservedMemberAuthority {
        member_operation_id: allocation.operation_id,
        component_spec: &allocation.component_spec,
        spec_hash: allocation.spec_hash,
        provisioning_origin: &allocation.provisioning_origin,
        release_set: allocation.release_set,
    };
    if actual != expected {
        return Err(InternalError::conflict(
            "Component Group member reservation differs from accepted authority",
        ));
    }
    if !matches!(
        allocation.progress,
        RootComponentAllocationProgressView::Reserved
    ) {
        return Err(InternalError::conflict(
            "Component Group member crossed its reservation boundary outside the aggregate workflow",
        ));
    }
    Ok(())
}

fn domain_separated_candid_hash<T: CandidType>(
    domain: &[u8],
    value: T,
) -> Result<[u8; 32], InternalError> {
    let bytes = candid::encode_one(value).map_err(|error| {
        InternalError::invariant(
            InternalErrorOrigin::Ops,
            format!("could not encode root Component provisioning authority: {error}"),
        )
    })?;
    let byte_count = u64::try_from(bytes.len()).map_err(|_| {
        InternalError::resource_exhausted(
            "root Component provisioning authority exceeds the canonical byte-count range",
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(byte_count.to_be_bytes());
    hasher.update(bytes);
    Ok(hasher.finalize().into())
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
        RootComponentProvisioningCommitError::OperationChanged => InternalError::conflict(
            "root Component provisioning operation changed before progress committed",
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
        reserved_component_count: view.reservation_cursor.reserved_component_count,
        accepted_at_ns: view.accepted_at_ns,
        receipt_content_hash: view.receipt_content_hash,
    }
}
