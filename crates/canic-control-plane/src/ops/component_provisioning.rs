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
        RootComponentProvisioningClaimCursorRecord, RootComponentProvisioningCommitError,
        RootComponentProvisioningInstallCursorRecord, RootComponentProvisioningPlacementKey,
        RootComponentProvisioningPlacementRecord, RootComponentProvisioningRecord,
        RootComponentProvisioningRegistryCursorRecord,
        RootComponentProvisioningReservationCursorRecord,
        RootComponentProvisioningStateRecordPhase, RootComponentProvisioningStore,
    },
    view::{
        component_provisioning::{
            RootComponentProvisioningAdvanceDisposition, RootComponentProvisioningClaimCursorView,
            RootComponentProvisioningInstallCursorView, RootComponentProvisioningMemberView,
            RootComponentProvisioningRegistryCursorView,
            RootComponentProvisioningReservationCursorView, RootComponentProvisioningView,
        },
        component_registry::{
            ComponentRegistryPartitionView, RootComponentAllocationProgressView,
            RootComponentAllocationView,
        },
    },
};
use candid::{CandidType, Principal};
use canic_core::{
    control_plane_support::{
        error::{InternalError, InternalErrorOrigin},
        ops::component_provisioning_plan::RootComponentProvisioningBatchValidation,
    },
    dto::{
        component_deployment::ProtectedComponentDeployment,
        component_provisioning::{
            RootComponentProvisioningAcceptanceRequest, RootComponentProvisioningAdvanceRequest,
            RootComponentProvisioningPhase, RootComponentProvisioningStatusRequest,
            RootComponentProvisioningStatusResponse,
        },
        component_registry::{
            ComponentLifecycleStatus, ComponentProvisioningOrigin, ComponentRegistryHead,
        },
        fleet_registry::FleetRegistryVersion,
    },
    ids::{
        ComponentBinding, ComponentDeploymentConfigurationDigest, ComponentGroupMemberPath,
        ComponentGroupPlacementId, ComponentSpecId,
    },
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const ACCEPTANCE_RECEIPT_DOMAIN: &[u8] = b"canic/root-component-provisioning-acceptance-receipt/v1";
const MEMBER_OPERATION_DOMAIN: &[u8] = b"canic/root-component-provisioning-member-operation/v1";
const RESERVATION_CURSOR_DOMAIN: &[u8] = b"canic/root-component-provisioning-reservation-cursor/v1";
const CLAIM_CURSOR_DOMAIN: &[u8] = b"canic/root-component-provisioning-claim-cursor/v1";
const INSTALL_CURSOR_DOMAIN: &[u8] = b"canic/root-component-provisioning-install-cursor/v1";
const REGISTRY_CURSOR_DOMAIN: &[u8] = b"canic/root-component-provisioning-registry-cursor/v1";

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

#[derive(CandidType)]
struct RootComponentProvisioningClaimCursorAuthority {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    placement_index: u32,
    member_index: u32,
    claimed_component_count: u32,
}

#[derive(CandidType)]
struct RootComponentProvisioningInstallCursorAuthority {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    placement_index: u32,
    member_index: u32,
    installed_component_count: u32,
}

#[derive(CandidType)]
struct RootComponentProvisioningRegistryCursorAuthority {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    placement_index: u32,
    member_index: u32,
    registry_committed_component_count: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProvisioningProgress {
    reserved: u32,
    claimed: u32,
    installed: u32,
    registry_committed: u32,
}

#[derive(Clone, Copy)]
struct ProvisioningCursorRecords {
    reservation: RootComponentProvisioningReservationCursorRecord,
    claim: RootComponentProvisioningClaimCursorRecord,
    install: RootComponentProvisioningInstallCursorRecord,
    registry: RootComponentProvisioningRegistryCursorRecord,
}

impl ProvisioningProgress {
    const fn from_request(request: RootComponentProvisioningAdvanceRequest) -> Self {
        Self {
            reserved: request.expected_reserved_component_count,
            claimed: request.expected_claimed_component_count,
            installed: request.expected_installed_component_count,
            registry_committed: request.expected_registry_committed_component_count,
        }
    }

    const fn from_view(view: &RootComponentProvisioningView) -> Self {
        Self {
            reserved: view.reservation_cursor.reserved_component_count,
            claimed: view.claim_cursor.claimed_component_count,
            installed: view.install_cursor.installed_component_count,
            registry_committed: view.registry_cursor.registry_committed_component_count,
        }
    }

    fn replays_one_step_before(self, current: Self, component_count: u32) -> bool {
        let reservation = self.claimed == 0
            && self.installed == 0
            && self.registry_committed == 0
            && current.claimed == 0
            && current.installed == 0
            && current.registry_committed == 0
            && self.reserved.checked_add(1) == Some(current.reserved);
        let claim = self.reserved == current.reserved
            && self.installed == 0
            && self.registry_committed == 0
            && current.installed == 0
            && current.registry_committed == 0
            && self.claimed.checked_add(1) == Some(current.claimed);
        let install = self.reserved == current.reserved
            && self.claimed == current.claimed
            && self.registry_committed == 0
            && current.registry_committed == 0
            && self.installed.checked_add(1) == Some(current.installed);
        let registry_commit = self.reserved == current.reserved
            && self.claimed == current.claimed
            && self.installed == current.installed
            && self.registry_committed.checked_add(1) == Some(current.registry_committed);
        let prerequisites_are_canonical = (current.claimed == 0
            || current.reserved == component_count)
            && (current.installed == 0 || current.claimed == component_count)
            && (current.registry_committed == 0 || current.installed == component_count);
        prerequisites_are_canonical && (reservation || claim || install || registry_commit)
    }
}

#[derive(Eq, PartialEq)]
struct ReservedMemberAuthority<'a> {
    member_operation_id: [u8; 32],
    component_spec: &'a ComponentSpecId,
    spec_hash: [u8; 32],
    provisioning_origin: &'a ComponentProvisioningOrigin,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
}

#[derive(Eq, PartialEq)]
struct RegistryCommittedMemberAuthority<'a> {
    binding: &'a ComponentBinding,
    provisioning_origin: &'a ComponentProvisioningOrigin,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
    status: ComponentLifecycleStatus,
    registry: ComponentRegistryHead,
    registry_encoded_bytes: u64,
    directory_synchronized_at_ns: u64,
    reserved_descendants: u32,
    committed_descendants: u32,
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
        let claim_cursor = claim_cursor_record(request.operation_id, request.plan_hash, 0, 0, 0)?;
        let install_cursor =
            install_cursor_record(request.operation_id, request.plan_hash, 0, 0, 0)?;
        let registry_cursor =
            registry_cursor_record(request.operation_id, request.plan_hash, 0, 0, 0)?;
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
                claim_cursor,
                install_cursor,
                registry_cursor,
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

    /// Interpret every expected cursor without allowing a retry to skip work.
    pub(crate) fn advance_disposition(
        request: RootComponentProvisioningAdvanceRequest,
        view: &RootComponentProvisioningView,
    ) -> Result<RootComponentProvisioningAdvanceDisposition, InternalError> {
        validate_operation_and_plan_hash(request.operation_id, request.plan_hash)?;
        if request.operation_id != view.operation_id || request.plan_hash != view.plan_hash {
            return Err(InternalError::conflict(
                "root Component provisioning advance request names different authority",
            ));
        }
        let expected = ProvisioningProgress::from_request(request);
        let current = ProvisioningProgress::from_view(view);
        if expected == current {
            return if current.registry_committed == view.component_count {
                Ok(RootComponentProvisioningAdvanceDisposition::Complete)
            } else {
                Ok(RootComponentProvisioningAdvanceDisposition::Advance)
            };
        }
        if expected.replays_one_step_before(current, view.component_count) {
            return Ok(RootComponentProvisioningAdvanceDisposition::Replay);
        }
        Err(InternalError::conflict(
            "root Component provisioning cursors differ from expected progress",
        ))
    }

    /// Select the next member in O(1) from the hash-bound canonical cursor.
    pub(crate) fn next_member_reservation(
        view: &RootComponentProvisioningView,
    ) -> Result<RootComponentProvisioningMemberView, InternalError> {
        if view.reservation_cursor.reserved_component_count >= view.component_count {
            return Err(InternalError::conflict(
                "root Component provisioning has no unreserved member",
            ));
        }
        member_at_cursor(
            view,
            view.reservation_cursor.placement_index,
            view.reservation_cursor.member_index,
        )
    }

    /// Select the next prepaid-Canister claim in O(1) canonical member order.
    pub(crate) fn next_member_claim(
        view: &RootComponentProvisioningView,
    ) -> Result<RootComponentProvisioningMemberView, InternalError> {
        if view.reservation_cursor.reserved_component_count != view.component_count {
            return Err(InternalError::conflict(
                "root Component provisioning cannot claim assets before all identities are reserved",
            ));
        }
        if view.claim_cursor.claimed_component_count >= view.component_count {
            return Err(InternalError::conflict(
                "root Component provisioning has no unclaimed member",
            ));
        }
        member_at_cursor(
            view,
            view.claim_cursor.placement_index,
            view.claim_cursor.member_index,
        )
    }

    /// Select the next Store-backed install in O(1) canonical member order.
    pub(crate) fn next_member_install(
        view: &RootComponentProvisioningView,
    ) -> Result<RootComponentProvisioningMemberView, InternalError> {
        if view.claim_cursor.claimed_component_count != view.component_count {
            return Err(InternalError::conflict(
                "root Component provisioning cannot install members before every Canister is claimed",
            ));
        }
        if view.install_cursor.installed_component_count >= view.component_count {
            return Err(InternalError::conflict(
                "root Component provisioning has no uninstalled member",
            ));
        }
        member_at_cursor(
            view,
            view.install_cursor.placement_index,
            view.install_cursor.member_index,
        )
    }

    /// Select the next verified member for one O(1) Component Registry commitment.
    pub(crate) fn next_member_registry_commit(
        view: &RootComponentProvisioningView,
    ) -> Result<RootComponentProvisioningMemberView, InternalError> {
        if view.install_cursor.installed_component_count != view.component_count {
            return Err(InternalError::conflict(
                "root Component provisioning cannot commit Registry partitions before every member is installed",
            ));
        }
        if view.registry_cursor.registry_committed_component_count >= view.component_count {
            return Err(InternalError::conflict(
                "root Component provisioning has no Registry-uncommitted member",
            ));
        }
        member_at_cursor(
            view,
            view.registry_cursor.placement_index,
            view.registry_cursor.member_index,
        )
    }

    /// Derive one exact protected runtime context from accepted plan and claimed allocation.
    pub(crate) fn member_deployment_context(
        view: &RootComponentProvisioningView,
        member: &RootComponentProvisioningMemberView,
        allocation: &RootComponentAllocationView,
    ) -> Result<ProtectedComponentDeployment, InternalError> {
        validate_member_authority(view, member, allocation)?;
        let canister_id = claimed_allocation_canister(&allocation.progress)?;
        Ok(ProtectedComponentDeployment::GroupMember {
            binding: ComponentBinding {
                authority: view.batch.root.authority.clone(),
                component: allocation.component,
                component_spec: allocation.component_spec.clone(),
                spec_hash: allocation.spec_hash,
                role: allocation.role.clone(),
                placement_subnet: view.batch.root.placement_subnet,
                fleet_subnet_root: view.batch.root.fleet_subnet_root,
                canister_id,
            },
            configuration_digest: view.configuration_digest,
            group_placement: member.group_placement.clone(),
            component_group: member.component_group.clone(),
            member_path: member.member_path.clone(),
            purpose: member.purpose.clone(),
            labels: member.labels.clone(),
            limits: member.limits.clone(),
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
        if Self::advance_disposition(request, &current)?
            != RootComponentProvisioningAdvanceDisposition::Advance
            || current.reservation_cursor.reserved_component_count == current.component_count
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
            claim_cursor,
            install_cursor,
            registry_cursor,
            accepted_at_ns,
            receipt_content_hash,
            ..
        } = next_record.state;
        next_record.state = RootComponentProvisioningStateRecordPhase::Accepted {
            placement_count,
            component_count,
            reservation_cursor: next_cursor,
            claim_cursor,
            install_cursor,
            registry_cursor,
            accepted_at_ns,
            receipt_content_hash,
        };
        RootComponentProvisioningStore::replace_operation(&current_record, next_record.clone())
            .map_err(map_commit_error)?;
        validated_record(next_record)
    }

    /// Commit one exact reconciled prepaid-Canister claim to the aggregate cursor.
    pub(crate) fn mark_member_claimed(
        request: RootComponentProvisioningAdvanceRequest,
        allocation: &RootComponentAllocationView,
    ) -> Result<RootComponentProvisioningView, InternalError> {
        let current_record = RootComponentProvisioningStore::operation(request.operation_id)
            .ok_or_else(|| {
                InternalError::unavailable("root Component provisioning operation is not accepted")
            })?;
        let current = validated_record(current_record.clone())?;
        if Self::advance_disposition(request, &current)?
            != RootComponentProvisioningAdvanceDisposition::Advance
            || current.reservation_cursor.reserved_component_count != current.component_count
        {
            return Err(InternalError::conflict(
                "root Component provisioning claim step is already committed or not ready",
            ));
        }
        let member = Self::next_member_claim(&current)?;
        validate_member_authority(&current, &member, allocation)?;
        claimed_allocation_canister(&allocation.progress)?;
        let next_cursor = advance_claim_cursor(&current)?;
        let mut next_record = current_record.clone();
        let RootComponentProvisioningStateRecordPhase::Accepted {
            placement_count,
            component_count,
            reservation_cursor,
            install_cursor,
            registry_cursor,
            accepted_at_ns,
            receipt_content_hash,
            ..
        } = next_record.state;
        next_record.state = RootComponentProvisioningStateRecordPhase::Accepted {
            placement_count,
            component_count,
            reservation_cursor,
            claim_cursor: next_cursor,
            install_cursor,
            registry_cursor,
            accepted_at_ns,
            receipt_content_hash,
        };
        RootComponentProvisioningStore::replace_operation(&current_record, next_record.clone())
            .map_err(map_commit_error)?;
        validated_record(next_record)
    }

    /// Commit one exact verified Store-backed install to the aggregate cursor.
    pub(crate) fn mark_member_installed(
        request: RootComponentProvisioningAdvanceRequest,
        allocation: &RootComponentAllocationView,
    ) -> Result<RootComponentProvisioningView, InternalError> {
        let current_record = RootComponentProvisioningStore::operation(request.operation_id)
            .ok_or_else(|| {
                InternalError::unavailable("root Component provisioning operation is not accepted")
            })?;
        let current = validated_record(current_record.clone())?;
        if Self::advance_disposition(request, &current)?
            != RootComponentProvisioningAdvanceDisposition::Advance
            || current.claim_cursor.claimed_component_count != current.component_count
        {
            return Err(InternalError::conflict(
                "root Component provisioning install step is already committed or not ready",
            ));
        }
        let member = Self::next_member_install(&current)?;
        validate_installed_member(&current, &member, allocation)?;
        let next_cursor = advance_install_cursor(&current)?;
        let mut next_record = current_record.clone();
        let RootComponentProvisioningStateRecordPhase::Accepted {
            placement_count,
            component_count,
            reservation_cursor,
            claim_cursor,
            accepted_at_ns,
            receipt_content_hash,
            registry_cursor,
            ..
        } = next_record.state;
        next_record.state = RootComponentProvisioningStateRecordPhase::Accepted {
            placement_count,
            component_count,
            reservation_cursor,
            claim_cursor,
            install_cursor: next_cursor,
            registry_cursor,
            accepted_at_ns,
            receipt_content_hash,
        };
        RootComponentProvisioningStore::replace_operation(&current_record, next_record.clone())
            .map_err(map_commit_error)?;
        validated_record(next_record)
    }

    /// Commit one exact reconciled Component Registry partition to the aggregate cursor.
    pub(crate) fn mark_member_registry_committed(
        request: RootComponentProvisioningAdvanceRequest,
        allocation: &RootComponentAllocationView,
        partition: &crate::view::component_registry::ComponentRegistryPartitionView,
    ) -> Result<RootComponentProvisioningView, InternalError> {
        let current_record = RootComponentProvisioningStore::operation(request.operation_id)
            .ok_or_else(|| {
                InternalError::unavailable("root Component provisioning operation is not accepted")
            })?;
        let current = validated_record(current_record.clone())?;
        if Self::advance_disposition(request, &current)?
            != RootComponentProvisioningAdvanceDisposition::Advance
            || current.install_cursor.installed_component_count != current.component_count
        {
            return Err(InternalError::conflict(
                "root Component provisioning Registry step is already committed or not ready",
            ));
        }
        let member = Self::next_member_registry_commit(&current)?;
        validate_registry_committed_member(&current, &member, allocation, partition)?;
        let next_cursor = advance_registry_cursor(&current)?;
        let mut next_record = current_record.clone();
        let RootComponentProvisioningStateRecordPhase::Accepted {
            placement_count,
            component_count,
            reservation_cursor,
            claim_cursor,
            install_cursor,
            accepted_at_ns,
            receipt_content_hash,
            ..
        } = next_record.state;
        next_record.state = RootComponentProvisioningStateRecordPhase::Accepted {
            placement_count,
            component_count,
            reservation_cursor,
            claim_cursor,
            install_cursor,
            registry_cursor: next_cursor,
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
        claim_cursor,
        install_cursor,
        registry_cursor,
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
    validate_record_cursors(
        record.operation_id,
        record.plan_hash,
        &record.batch,
        component_count,
        ProvisioningCursorRecords {
            reservation: reservation_cursor,
            claim: claim_cursor,
            install: install_cursor,
            registry: registry_cursor,
        },
    )?;
    validate_record_placement_index(record.operation_id, record.plan_hash, &record.batch)?;
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
        claim_cursor: RootComponentProvisioningClaimCursorView {
            placement_index: claim_cursor.placement_index,
            member_index: claim_cursor.member_index,
            claimed_component_count: claim_cursor.claimed_component_count,
        },
        install_cursor: RootComponentProvisioningInstallCursorView {
            placement_index: install_cursor.placement_index,
            member_index: install_cursor.member_index,
            installed_component_count: install_cursor.installed_component_count,
        },
        registry_cursor: RootComponentProvisioningRegistryCursorView {
            placement_index: registry_cursor.placement_index,
            member_index: registry_cursor.member_index,
            registry_committed_component_count: registry_cursor.registry_committed_component_count,
        },
        accepted_at_ns,
        receipt_content_hash,
    })
}

fn validate_record_cursors(
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    batch: &canic_core::dto::component_provisioning::FleetSubnetRootProvisioningBatch,
    component_count: u32,
    cursors: ProvisioningCursorRecords,
) -> Result<(), InternalError> {
    validate_reservation_cursor(
        operation_id,
        plan_hash,
        batch,
        component_count,
        cursors.reservation,
    )?;
    validate_claim_cursor(
        operation_id,
        plan_hash,
        batch,
        component_count,
        cursors.reservation.reserved_component_count,
        cursors.claim,
    )?;
    validate_install_cursor(
        operation_id,
        plan_hash,
        batch,
        component_count,
        cursors.claim.claimed_component_count,
        cursors.install,
    )?;
    validate_registry_cursor(
        operation_id,
        plan_hash,
        batch,
        component_count,
        cursors.install.installed_component_count,
        cursors.registry,
    )
}

fn validate_record_placement_index(
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    batch: &canic_core::dto::component_provisioning::FleetSubnetRootProvisioningBatch,
) -> Result<(), InternalError> {
    let expected_placement = RootComponentProvisioningPlacementRecord {
        operation_id,
        plan_hash,
    };
    for placement in &batch.placements {
        let key = RootComponentProvisioningPlacementKey::from(&placement.group_placement);
        if RootComponentProvisioningStore::placement(&key) != Some(expected_placement) {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "accepted root Component provisioning placement index is inconsistent",
            ));
        }
    }
    Ok(())
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
    if cursor.content_hash != expected.content_hash {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root Component provisioning reservation cursor hash is invalid",
        ));
    }
    validate_member_cursor(
        batch,
        component_count,
        cursor.placement_index,
        cursor.member_index,
        cursor.reserved_component_count,
        "reservation",
    )
}

fn validate_claim_cursor(
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    batch: &canic_core::dto::component_provisioning::FleetSubnetRootProvisioningBatch,
    component_count: u32,
    reserved_component_count: u32,
    cursor: RootComponentProvisioningClaimCursorRecord,
) -> Result<(), InternalError> {
    let expected = claim_cursor_record(
        operation_id,
        plan_hash,
        cursor.placement_index,
        cursor.member_index,
        cursor.claimed_component_count,
    )?;
    if cursor.content_hash != expected.content_hash {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root Component provisioning claim cursor hash is invalid",
        ));
    }
    if cursor.claimed_component_count > 0 && reserved_component_count != component_count {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root Component provisioning claimed an asset before reserving every identity",
        ));
    }
    validate_member_cursor(
        batch,
        component_count,
        cursor.placement_index,
        cursor.member_index,
        cursor.claimed_component_count,
        "claim",
    )
}

fn validate_install_cursor(
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    batch: &canic_core::dto::component_provisioning::FleetSubnetRootProvisioningBatch,
    component_count: u32,
    claimed_component_count: u32,
    cursor: RootComponentProvisioningInstallCursorRecord,
) -> Result<(), InternalError> {
    let expected = install_cursor_record(
        operation_id,
        plan_hash,
        cursor.placement_index,
        cursor.member_index,
        cursor.installed_component_count,
    )?;
    if cursor.content_hash != expected.content_hash {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root Component provisioning install cursor hash is invalid",
        ));
    }
    if cursor.installed_component_count > 0 && claimed_component_count != component_count {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root Component provisioning installed a member before claiming every Canister",
        ));
    }
    validate_member_cursor(
        batch,
        component_count,
        cursor.placement_index,
        cursor.member_index,
        cursor.installed_component_count,
        "install",
    )
}

fn validate_registry_cursor(
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    batch: &canic_core::dto::component_provisioning::FleetSubnetRootProvisioningBatch,
    component_count: u32,
    installed_component_count: u32,
    cursor: RootComponentProvisioningRegistryCursorRecord,
) -> Result<(), InternalError> {
    let expected = registry_cursor_record(
        operation_id,
        plan_hash,
        cursor.placement_index,
        cursor.member_index,
        cursor.registry_committed_component_count,
    )?;
    if cursor.content_hash != expected.content_hash {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root Component provisioning Registry cursor hash is invalid",
        ));
    }
    if cursor.registry_committed_component_count > 0 && installed_component_count != component_count
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root Component provisioning committed a Registry partition before installing every member",
        ));
    }
    validate_member_cursor(
        batch,
        component_count,
        cursor.placement_index,
        cursor.member_index,
        cursor.registry_committed_component_count,
        "Registry",
    )
}

fn validate_member_cursor(
    batch: &canic_core::dto::component_provisioning::FleetSubnetRootProvisioningBatch,
    component_count: u32,
    placement_index: u32,
    member_index: u32,
    completed_count: u32,
    cursor_kind: &str,
) -> Result<(), InternalError> {
    if completed_count > component_count {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            format!("root Component provisioning {cursor_kind} cursor count is invalid"),
        ));
    }
    let placement_count = u32::try_from(batch.placements.len()).map_err(|_| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root Component provisioning placement count exceeds u32",
        )
    })?;
    if completed_count == component_count {
        if placement_index != placement_count || member_index != 0 {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                format!(
                    "terminal root Component provisioning {cursor_kind} cursor is not canonical"
                ),
            ));
        }
        return Ok(());
    }
    let placement = batch
        .placements
        .get(usize::try_from(placement_index).map_err(|_| {
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
    if usize::try_from(member_index)
        .ok()
        .is_none_or(|index| index >= placement.entries.len())
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            format!("root Component provisioning {cursor_kind} member cursor is out of bounds"),
        ));
    }
    Ok(())
}

fn advance_reservation_cursor(
    view: &RootComponentProvisioningView,
) -> Result<RootComponentProvisioningReservationCursorRecord, InternalError> {
    let (next_placement, next_member, next_reserved) = advance_member_cursor(
        view,
        view.reservation_cursor.placement_index,
        view.reservation_cursor.member_index,
        view.reservation_cursor.reserved_component_count,
    )?;
    reservation_cursor_record(
        view.operation_id,
        view.plan_hash,
        next_placement,
        next_member,
        next_reserved,
    )
}

fn advance_claim_cursor(
    view: &RootComponentProvisioningView,
) -> Result<RootComponentProvisioningClaimCursorRecord, InternalError> {
    let (next_placement, next_member, next_claimed) = advance_member_cursor(
        view,
        view.claim_cursor.placement_index,
        view.claim_cursor.member_index,
        view.claim_cursor.claimed_component_count,
    )?;
    claim_cursor_record(
        view.operation_id,
        view.plan_hash,
        next_placement,
        next_member,
        next_claimed,
    )
}

fn advance_install_cursor(
    view: &RootComponentProvisioningView,
) -> Result<RootComponentProvisioningInstallCursorRecord, InternalError> {
    let (next_placement, next_member, next_installed) = advance_member_cursor(
        view,
        view.install_cursor.placement_index,
        view.install_cursor.member_index,
        view.install_cursor.installed_component_count,
    )?;
    install_cursor_record(
        view.operation_id,
        view.plan_hash,
        next_placement,
        next_member,
        next_installed,
    )
}

fn advance_registry_cursor(
    view: &RootComponentProvisioningView,
) -> Result<RootComponentProvisioningRegistryCursorRecord, InternalError> {
    let (next_placement, next_member, next_committed) = advance_member_cursor(
        view,
        view.registry_cursor.placement_index,
        view.registry_cursor.member_index,
        view.registry_cursor.registry_committed_component_count,
    )?;
    registry_cursor_record(
        view.operation_id,
        view.plan_hash,
        next_placement,
        next_member,
        next_committed,
    )
}

fn advance_member_cursor(
    view: &RootComponentProvisioningView,
    placement_index: u32,
    member_index: u32,
    completed_count: u32,
) -> Result<(u32, u32, u32), InternalError> {
    let placement = view
        .batch
        .placements
        .get(usize::try_from(placement_index).map_err(|_| {
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
    let next_completed = completed_count.checked_add(1).ok_or_else(|| {
        InternalError::resource_exhausted("root Component provisioning cursor count overflowed")
    })?;
    let next_member = member_index.checked_add(1).ok_or_else(|| {
        InternalError::resource_exhausted("root Component provisioning member cursor overflowed")
    })?;
    let entry_count = u32::try_from(placement.entries.len()).map_err(|_| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "root Component provisioning placement member count exceeds u32",
        )
    })?;
    if next_member == entry_count {
        let next_placement = placement_index.checked_add(1).ok_or_else(|| {
            InternalError::resource_exhausted(
                "root Component provisioning placement cursor overflowed",
            )
        })?;
        Ok((next_placement, 0, next_completed))
    } else {
        Ok((placement_index, next_member, next_completed))
    }
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

fn claim_cursor_record(
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    placement_index: u32,
    member_index: u32,
    claimed_component_count: u32,
) -> Result<RootComponentProvisioningClaimCursorRecord, InternalError> {
    let authority = RootComponentProvisioningClaimCursorAuthority {
        operation_id,
        plan_hash,
        placement_index,
        member_index,
        claimed_component_count,
    };
    Ok(RootComponentProvisioningClaimCursorRecord {
        placement_index,
        member_index,
        claimed_component_count,
        content_hash: domain_separated_candid_hash(CLAIM_CURSOR_DOMAIN, authority)?,
    })
}

fn install_cursor_record(
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    placement_index: u32,
    member_index: u32,
    installed_component_count: u32,
) -> Result<RootComponentProvisioningInstallCursorRecord, InternalError> {
    let authority = RootComponentProvisioningInstallCursorAuthority {
        operation_id,
        plan_hash,
        placement_index,
        member_index,
        installed_component_count,
    };
    Ok(RootComponentProvisioningInstallCursorRecord {
        placement_index,
        member_index,
        installed_component_count,
        content_hash: domain_separated_candid_hash(INSTALL_CURSOR_DOMAIN, authority)?,
    })
}

fn registry_cursor_record(
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    placement_index: u32,
    member_index: u32,
    registry_committed_component_count: u32,
) -> Result<RootComponentProvisioningRegistryCursorRecord, InternalError> {
    let authority = RootComponentProvisioningRegistryCursorAuthority {
        operation_id,
        plan_hash,
        placement_index,
        member_index,
        registry_committed_component_count,
    };
    Ok(RootComponentProvisioningRegistryCursorRecord {
        placement_index,
        member_index,
        registry_committed_component_count,
        content_hash: domain_separated_candid_hash(REGISTRY_CURSOR_DOMAIN, authority)?,
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

fn member_at_cursor(
    view: &RootComponentProvisioningView,
    placement_index: u32,
    member_index: u32,
) -> Result<RootComponentProvisioningMemberView, InternalError> {
    let placement = view
        .batch
        .placements
        .get(usize::try_from(placement_index).map_err(|_| {
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
    let entry = placement
        .entries
        .get(usize::try_from(member_index).map_err(|_| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "root Component provisioning member cursor exceeds usize",
            )
        })?)
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "root Component provisioning member cursor is out of bounds",
            )
        })?;
    Ok(RootComponentProvisioningMemberView {
        member_operation_id: member_operation_id(
            view.batch.root.fleet_subnet_root,
            view.operation_id,
            view.plan_hash,
            &placement.group_placement,
            &entry.member_path,
        )?,
        group_placement: placement.group_placement.clone(),
        component_group: placement.component_group.clone(),
        member_path: entry.member_path.clone(),
        component_spec: entry.component_spec.clone(),
        spec_hash: entry.spec_hash,
        purpose: entry.purpose.clone(),
        labels: entry.labels.clone(),
        limits: entry.limits.clone(),
    })
}

fn validate_reserved_member(
    view: &RootComponentProvisioningView,
    member: &RootComponentProvisioningMemberView,
    allocation: &RootComponentAllocationView,
) -> Result<(), InternalError> {
    validate_member_authority(view, member, allocation)?;
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

fn validate_installed_member(
    view: &RootComponentProvisioningView,
    member: &RootComponentProvisioningMemberView,
    allocation: &RootComponentAllocationView,
) -> Result<(), InternalError> {
    validate_member_authority(view, member, allocation)?;
    if !matches!(
        allocation.progress,
        RootComponentAllocationProgressView::Verified { .. }
    ) {
        return Err(InternalError::conflict(
            "Component Group member did not stop at the verified install boundary",
        ));
    }
    Ok(())
}

fn validate_registry_committed_member(
    view: &RootComponentProvisioningView,
    member: &RootComponentProvisioningMemberView,
    allocation: &RootComponentAllocationView,
    partition: &ComponentRegistryPartitionView,
) -> Result<(), InternalError> {
    validate_member_authority(view, member, allocation)?;
    let RootComponentAllocationProgressView::Committed {
        installation,
        commitment,
        ..
    } = &allocation.progress
    else {
        return Err(InternalError::conflict(
            "Component Group member did not reach its Registry commitment boundary",
        ));
    };
    let expected = RegistryCommittedMemberAuthority {
        binding: &installation.binding,
        provisioning_origin: &allocation.provisioning_origin,
        release_set: allocation.release_set,
        status: ComponentLifecycleStatus::Prepared,
        registry: commitment.registry.clone(),
        registry_encoded_bytes: commitment.prepared_registry_encoded_bytes,
        directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
        reserved_descendants: 0,
        committed_descendants: 0,
    };
    let actual = RegistryCommittedMemberAuthority {
        binding: &partition.binding,
        provisioning_origin: &partition.provisioning_origin,
        release_set: partition.release_set,
        status: partition.status,
        registry: ComponentRegistryHead {
            component: partition.binding.component,
            revision: partition.revision,
            content_hash: partition.content_hash,
        },
        registry_encoded_bytes: partition.encoded_bytes,
        directory_synchronized_at_ns: partition.directory_synchronized_at_ns,
        reserved_descendants: partition.reserved_descendants,
        committed_descendants: partition.committed_descendants,
    };
    if actual != expected || actual.registry_encoded_bytes > member.limits.maximum_registry_bytes {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Component Group member Registry partition differs from its accepted authority or durable receipt",
        ));
    }
    Ok(())
}

fn validate_member_authority(
    view: &RootComponentProvisioningView,
    member: &RootComponentProvisioningMemberView,
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
    Ok(())
}

fn claimed_allocation_canister(
    progress: &RootComponentAllocationProgressView,
) -> Result<Principal, InternalError> {
    match progress {
        RootComponentAllocationProgressView::Created { canister, .. }
        | RootComponentAllocationProgressView::InstallIntent { canister, .. }
        | RootComponentAllocationProgressView::Installed { canister, .. }
        | RootComponentAllocationProgressView::Verified { canister, .. }
        | RootComponentAllocationProgressView::Committed { canister, .. } => Ok(*canister),
        RootComponentAllocationProgressView::Reserved
        | RootComponentAllocationProgressView::CreationIntent(_) => Err(InternalError::conflict(
            "Component Group member has not completed its prepaid-Canister claim",
        )),
        RootComponentAllocationProgressView::Removed { .. } => Err(InternalError::conflict(
            "Component Group member was removed before aggregate provisioning completed",
        )),
    }
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
        claimed_component_count: view.claim_cursor.claimed_component_count,
        installed_component_count: view.install_cursor.installed_component_count,
        registry_committed_component_count: view.registry_cursor.registry_committed_component_count,
        accepted_at_ns: view.accepted_at_ns,
        receipt_content_hash: view.receipt_content_hash,
    }
}
