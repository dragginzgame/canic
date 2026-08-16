//! Module: ops::component_provisioning
//!
//! Responsibility: commit and read exact root-local Component Group provisioning authority.
//! Does not own: caller authentication, Store observation, Component effects, or orchestration.
//! Boundary: workflow supplies a validated batch; ops derives immutable member context only from
//! that durable record.

#[cfg(test)]
mod tests;

use crate::ops::component_registry::ComponentRegistryOps;
use crate::{
    storage::stable::component_provisioning::{
        RootComponentProvisioningClaimCursorRecord, RootComponentProvisioningCommitError,
        RootComponentProvisioningInstallCursorRecord, RootComponentProvisioningPlacementKey,
        RootComponentProvisioningPlacementRecord, RootComponentProvisioningRecord,
        RootComponentProvisioningRegistryCursorRecord,
        RootComponentProvisioningReservationCursorRecord, RootComponentProvisioningResultRecord,
        RootComponentProvisioningRuntimeModeRecord, RootComponentProvisioningStateRecordPhase,
        RootComponentProvisioningStore, RootComponentPublicationIntentRecord,
        RootProvisionedGroupMemberRecord, RootProvisionedGroupPlacementRecord,
    },
    view::{
        component_provisioning::{
            RootComponentDeploymentAuthorityView, RootComponentGroupRuntimeAuthorityView,
            RootComponentProvisioningAdvanceDisposition, RootComponentProvisioningClaimCursorView,
            RootComponentProvisioningInstallCursorView, RootComponentProvisioningMemberView,
            RootComponentProvisioningRegistryCursorView,
            RootComponentProvisioningReservationCursorView, RootComponentProvisioningRuntimeMode,
            RootComponentProvisioningView, RootComponentPublicationIntentView,
            RootComponentPublicationMemberView,
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
        error::InternalError,
        ops::component_provisioning_plan::RootComponentProvisioningBatchValidation,
        ops::component_provisioning_receipt::{
            RootComponentProvisioningAcceptanceReceiptAuthority,
            RootComponentProvisioningProvisionedReceiptAuthority,
            RootComponentProvisioningPublishedReceiptAuthority,
            RootComponentProvisioningReceiptOps,
            RootComponentProvisioningRuntimesActiveReceiptAuthority,
        },
    },
    dto::{
        component_deployment::{
            ComponentDeploymentLimits, ComponentDeploymentPurpose, ProtectedComponentDeployment,
        },
        component_provisioning::{
            ComponentDirectoryPublicationEvidence, ComponentGroupDirectory,
            ComponentGroupDirectoryMember, ComponentGroupDirectoryProvenance,
            ComponentGroupDirectoryPublicationEvidence, RootComponentActivationEvidence,
            RootComponentActivationRequest, RootComponentProvisioningAcceptanceRequest,
            RootComponentProvisioningAdvanceRequest, RootComponentProvisioningPhase,
            RootComponentProvisioningResult, RootComponentProvisioningStatusRequest,
            RootComponentProvisioningStatusResponse, RootComponentPublicationEvidence,
            RootComponentPublicationRequest, RootProvisionedGroupMember,
            RootProvisionedGroupPlacement,
        },
        component_registry::{
            ComponentLifecycleStatus, ComponentProvisioningOrigin, ComponentRegistryHead,
        },
        fleet_registry::FleetDirectorySnapshot,
    },
    ids::{ComponentBinding, ComponentGroupMemberPath, ComponentGroupPlacementId, ComponentSpecId},
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const MEMBER_OPERATION_DOMAIN: &[u8] = b"canic/root-component-provisioning-member-operation/v1";
const RESERVATION_CURSOR_DOMAIN: &[u8] = b"canic/root-component-provisioning-reservation-cursor/v1";
const CLAIM_CURSOR_DOMAIN: &[u8] = b"canic/root-component-provisioning-claim-cursor/v1";
const INSTALL_CURSOR_DOMAIN: &[u8] = b"canic/root-component-provisioning-install-cursor/v1";
const REGISTRY_CURSOR_DOMAIN: &[u8] = b"canic/root-component-provisioning-registry-cursor/v1";

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

struct ProvisionedMemberEvidence {
    member: RootComponentProvisioningMemberView,
    allocation: RootComponentAllocationView,
    partition: ComponentRegistryPartitionView,
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

#[derive(Eq, PartialEq)]
struct ProvisionedPlacementAuthority<'a> {
    group_placement: &'a ComponentGroupPlacementId,
    component_group: &'a canic_core::ids::ComponentGroupSpecId,
    member_count: usize,
}

#[derive(Eq, PartialEq)]
struct ProvisionedResultMemberAuthority<'a> {
    member_path: &'a ComponentGroupMemberPath,
    component_spec: &'a ComponentSpecId,
    purpose: &'a ComponentDeploymentPurpose,
    limits: &'a ComponentDeploymentLimits,
    binding_authority: &'a canic_core::ids::FleetRegistryAuthority,
    binding_component_spec: &'a ComponentSpecId,
    binding_spec_hash: [u8; 32],
    binding_placement_subnet: canic_core::ids::SubnetId,
    binding_root: Principal,
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
            return Err(InternalError::conflict());
        }
        Ok(Some(view))
    }

    /// Commit one exact already-validated batch or replay its original receipt.
    pub(crate) fn accept(
        request: RootComponentProvisioningAcceptanceRequest,
        validation: &RootComponentProvisioningBatchValidation,
        runtime_mode: RootComponentProvisioningRuntimeMode,
        accepted_at_ns: u64,
    ) -> Result<RootComponentProvisioningView, InternalError> {
        validate_acceptance_identity(&request, validation, accepted_at_ns)?;
        if let Some(existing) = RootComponentProvisioningStore::operation(request.operation_id) {
            let view = validated_record(existing)?;
            return if request_matches_view(&request, &view) && view.runtime_mode == runtime_mode {
                Ok(view)
            } else {
                Err(InternalError::conflict())
            };
        }

        let current = validated_aggregate_state()?;
        let next_placements = current
            .tracked_group_placements
            .checked_add(validation.placement_count)
            .ok_or_else(|| InternalError::resource_exhausted())?;
        if next_placements > request.batch.root.limits.maximum_group_placements {
            return Err(InternalError::resource_exhausted());
        }
        for placement in &request.batch.placements {
            let key = RootComponentProvisioningPlacementKey::from(&placement.group_placement);
            if RootComponentProvisioningStore::placement(&key).is_some() {
                return Err(InternalError::conflict());
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
            runtime_mode: runtime_mode.into(),
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
        let record = RootComponentProvisioningStore::operation(request.operation_id)
            .ok_or_else(|| InternalError::unavailable())?;
        let view = validated_record(record)?;
        if view.plan_hash != request.plan_hash {
            return Err(InternalError::conflict());
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
            return Err(InternalError::conflict());
        }
        let expected = ProvisioningProgress::from_request(request);
        let current = ProvisioningProgress::from_view(view);
        if expected == current {
            return match view.phase {
                RootComponentProvisioningPhase::Accepted => {
                    Ok(RootComponentProvisioningAdvanceDisposition::Advance)
                }
                RootComponentProvisioningPhase::Provisioned
                | RootComponentProvisioningPhase::Published
                | RootComponentProvisioningPhase::RuntimesActive => {
                    Ok(RootComponentProvisioningAdvanceDisposition::Complete)
                }
            };
        }
        if expected.replays_one_step_before(current, view.component_count) {
            return Ok(RootComponentProvisioningAdvanceDisposition::Replay);
        }
        Err(InternalError::conflict())
    }

    /// Select the next member in O(1) from the hash-bound canonical cursor.
    pub(crate) fn next_member_reservation(
        view: &RootComponentProvisioningView,
    ) -> Result<RootComponentProvisioningMemberView, InternalError> {
        if view.reservation_cursor.reserved_component_count >= view.component_count {
            return Err(InternalError::conflict());
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
            return Err(InternalError::conflict());
        }
        if view.claim_cursor.claimed_component_count >= view.component_count {
            return Err(InternalError::conflict());
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
            return Err(InternalError::conflict());
        }
        if view.install_cursor.installed_component_count >= view.component_count {
            return Err(InternalError::conflict());
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
            return Err(InternalError::conflict());
        }
        if view.registry_cursor.registry_committed_component_count >= view.component_count {
            return Err(InternalError::conflict());
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

    /// Reconstruct one top-level Component's protected deployment and Directory authority.
    pub(crate) fn component_deployment_authority(
        origin: &ComponentProvisioningOrigin,
        binding: &ComponentBinding,
    ) -> Result<RootComponentDeploymentAuthorityView, InternalError> {
        let ComponentProvisioningOrigin::ComponentGroup {
            operation_id,
            plan_hash,
            group_placement,
            member_path,
        } = origin
        else {
            return Ok(RootComponentDeploymentAuthorityView {
                deployment: ProtectedComponentDeployment::UngroupedOrdinary {
                    binding: binding.clone(),
                },
                component_group: None,
            });
        };
        let record = RootComponentProvisioningStore::operation(*operation_id)
            .ok_or_else(|| InternalError::invariant())?;
        let view = validated_record(record)?;
        if view.plan_hash != *plan_hash {
            return Err(InternalError::invariant());
        }
        let placement_index = view
            .batch
            .placements
            .iter()
            .position(|placement| &placement.group_placement == group_placement)
            .ok_or_else(|| InternalError::invariant())?;
        let member = member_by_path(&view, group_placement, member_path)?;
        let component_authority_is_exact = [
            view.batch.root.authority == binding.authority,
            view.batch.root.placement_subnet == binding.placement_subnet,
            view.batch.root.fleet_subnet_root == binding.fleet_subnet_root,
            member.component_spec == binding.component_spec,
            member.spec_hash == binding.spec_hash,
        ]
        .into_iter()
        .all(|valid| valid);
        if !component_authority_is_exact {
            return Err(InternalError::invariant());
        }
        let result = view
            .result
            .as_ref()
            .ok_or_else(|| InternalError::invariant())?;
        let component_group =
            derive_component_group_directory_from_view(&view, result, placement_index)?;
        let retained_binding = component_group
            .members
            .iter()
            .find(|candidate| candidate.member_path == member.member_path)
            .map(|candidate| &candidate.binding)
            .ok_or_else(|| InternalError::invariant())?;
        if retained_binding != binding {
            return Err(InternalError::invariant());
        }
        Ok(RootComponentDeploymentAuthorityView {
            deployment: ProtectedComponentDeployment::GroupMember {
                binding: binding.clone(),
                configuration_digest: view.configuration_digest,
                group_placement: member.group_placement,
                component_group: member.component_group,
                member_path: member.member_path,
                purpose: member.purpose,
                labels: member.labels,
                limits: member.limits,
            },
            component_group: Some(component_group),
        })
    }

    /// Commit one exact reconciled Component identity reservation to the aggregate cursor.
    pub(crate) fn mark_member_reserved(
        request: RootComponentProvisioningAdvanceRequest,
        allocation: &RootComponentAllocationView,
    ) -> Result<RootComponentProvisioningView, InternalError> {
        let current_record = RootComponentProvisioningStore::operation(request.operation_id)
            .ok_or_else(|| InternalError::unavailable())?;
        let current = validated_record(current_record.clone())?;
        if Self::advance_disposition(request, &current)?
            != RootComponentProvisioningAdvanceDisposition::Advance
            || current.reservation_cursor.reserved_component_count == current.component_count
        {
            return Err(InternalError::conflict());
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
        } = next_record.state
        else {
            return Err(InternalError::invariant());
        };
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
            .ok_or_else(|| InternalError::unavailable())?;
        let current = validated_record(current_record.clone())?;
        if Self::advance_disposition(request, &current)?
            != RootComponentProvisioningAdvanceDisposition::Advance
            || current.reservation_cursor.reserved_component_count != current.component_count
        {
            return Err(InternalError::conflict());
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
        } = next_record.state
        else {
            return Err(InternalError::invariant());
        };
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
            .ok_or_else(|| InternalError::unavailable())?;
        let current = validated_record(current_record.clone())?;
        if Self::advance_disposition(request, &current)?
            != RootComponentProvisioningAdvanceDisposition::Advance
            || current.claim_cursor.claimed_component_count != current.component_count
        {
            return Err(InternalError::conflict());
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
        } = next_record.state
        else {
            return Err(InternalError::invariant());
        };
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
            .ok_or_else(|| InternalError::unavailable())?;
        let current = validated_record(current_record.clone())?;
        if Self::advance_disposition(request, &current)?
            != RootComponentProvisioningAdvanceDisposition::Advance
            || current.install_cursor.installed_component_count != current.component_count
        {
            return Err(InternalError::conflict());
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
        } = next_record.state
        else {
            return Err(InternalError::invariant());
        };
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

    /// Freeze one complete group-partitioned result after every Registry commit.
    pub(crate) fn finalize_provisioned(
        request: RootComponentProvisioningAdvanceRequest,
        provisioned_at_ns: u64,
    ) -> Result<RootComponentProvisioningView, InternalError> {
        let current = Self::status(RootComponentProvisioningStatusRequest {
            operation_id: request.operation_id,
            plan_hash: request.plan_hash,
        })?;
        let evidence = provisioned_member_evidence(&current)?;
        let result = provisioned_result_record(&current, &evidence)?;
        commit_provisioned_result(request, provisioned_at_ns, result)
    }

    /// Begin or replay publication against one exact newer Fleet Directory authority.
    pub(crate) fn begin_publication(
        request: &RootComponentPublicationRequest,
        fleet_directory: &FleetDirectorySnapshot,
        started_at_ns: u64,
    ) -> Result<RootComponentProvisioningView, InternalError> {
        let current_record = RootComponentProvisioningStore::operation(request.operation_id)
            .ok_or_else(|| InternalError::unavailable())?;
        let current = validated_record(current_record.clone())?;
        validate_publication_request(request, &current)?;
        if current.phase == RootComponentProvisioningPhase::Published
            || current.publication_started_at_ns.is_some()
        {
            return Ok(current);
        }
        if started_at_ns < current.provisioned_at_ns.unwrap_or(u64::MAX) {
            return Err(InternalError::invalid_input());
        }
        if fleet_directory.provenance.registry != request.published_fleet_registry
            || fleet_directory.provenance.source_fleet_subnet_root
                != current.batch.root.fleet_subnet_root
        {
            return Err(InternalError::conflict());
        }
        let RootComponentProvisioningStateRecordPhase::Provisioned {
            placement_count,
            component_count,
            result,
            accepted_at_ns,
            provisioned_at_ns,
            receipt_content_hash,
        } = current_record.state.clone()
        else {
            return Err(InternalError::conflict());
        };
        let result_view = provisioning_result_from_record(&result);
        let component_group_directories =
            result_view
                .placements
                .iter()
                .enumerate()
                .map(|(index, placement)| {
                    let directory =
                        derive_component_group_directory(&current_record, &result_view, index)?;
                    Ok(ComponentGroupDirectoryPublicationEvidence {
                    group_placement: placement.group_placement.clone(),
                    content_hash: RootComponentProvisioningReceiptOps::
                        component_group_directory_content_hash(&directory)?,
                })
                })
                .collect::<Result<Vec<_>, InternalError>>()?;
        let publication = RootComponentPublicationEvidence {
            fleet_registry: request.published_fleet_registry.clone(),
            fleet_directory_content_hash:
                RootComponentProvisioningReceiptOps::fleet_directory_content_hash(fleet_directory)?,
            component_directories: vec![],
            component_group_directories,
        };
        let mut next = current_record.clone();
        next.state = RootComponentProvisioningStateRecordPhase::Publishing {
            placement_count,
            component_count,
            result,
            publication,
            published_component_count: 0,
            in_flight: None,
            accepted_at_ns,
            provisioned_at_ns,
            publication_started_at_ns: started_at_ns,
            provisioned_receipt_content_hash: receipt_content_hash,
        };
        RootComponentProvisioningStore::replace_operation(&current_record, next.clone())
            .map_err(map_commit_error)?;
        validated_record(next)
    }

    /// Select the next exact prepared Component and its root-derived group projection.
    pub(crate) fn next_publication_member(
        view: &RootComponentProvisioningView,
    ) -> Result<Option<RootComponentPublicationMemberView>, InternalError> {
        if view.published_component_count == view.component_count {
            return Ok(None);
        }
        member_at_index(view, view.published_component_count).map(Some)
    }

    /// Persist exact pre-call intent before one Component Directory delivery.
    pub(crate) fn begin_publication_delivery(
        request: &RootComponentPublicationRequest,
        member: &RootComponentPublicationMemberView,
        directory_authority_hash: [u8; 32],
        started_at_ns: u64,
    ) -> Result<RootComponentProvisioningView, InternalError> {
        let current_record = required_publishing_record(request)?;
        let current = validated_record(current_record.clone())?;
        validate_publication_request(request, &current)?;
        if let Some(intent) = &current.publication_in_flight {
            return if intent.component_index == member.component_index
                && intent.canister_id == member.binding.canister_id
                && intent.directory_authority_hash == directory_authority_hash
            {
                Ok(current)
            } else {
                Err(InternalError::conflict())
            };
        }
        if directory_authority_hash == [0; 32]
            || started_at_ns < current.publication_started_at_ns.unwrap_or(u64::MAX)
        {
            return Err(InternalError::invalid_input());
        }
        let mut next = current_record.clone();
        let RootComponentProvisioningStateRecordPhase::Publishing { in_flight, .. } =
            &mut next.state
        else {
            unreachable!("required publishing record changed before local mutation");
        };
        *in_flight = Some(RootComponentPublicationIntentRecord {
            component_index: member.component_index,
            canister_id: member.binding.canister_id,
            directory_authority_hash,
            started_at_ns,
        });
        RootComponentProvisioningStore::replace_operation(&current_record, next.clone())
            .map_err(map_commit_error)?;
        validated_record(next)
    }

    /// Commit one independently observed exact Component Directory delivery.
    pub(crate) fn record_publication_delivery(
        request: &RootComponentPublicationRequest,
        member: &RootComponentPublicationMemberView,
        directory_authority_hash: [u8; 32],
    ) -> Result<RootComponentProvisioningView, InternalError> {
        let current_record = required_publishing_record(request)?;
        let current = validated_record(current_record.clone())?;
        validate_publication_request(request, &current)?;
        let intent = current
            .publication_in_flight
            .as_ref()
            .ok_or_else(|| InternalError::conflict())?;
        if intent.component_index != member.component_index
            || intent.canister_id != member.binding.canister_id
            || intent.directory_authority_hash != directory_authority_hash
        {
            return Err(InternalError::conflict());
        }
        let mut next = current_record.clone();
        let RootComponentProvisioningStateRecordPhase::Publishing {
            publication,
            published_component_count,
            in_flight,
            ..
        } = &mut next.state
        else {
            unreachable!("required publishing record changed before local mutation");
        };
        publication
            .component_directories
            .push(ComponentDirectoryPublicationEvidence {
                component: member.binding.component,
                content_hash: member.component_registry_content_hash,
            });
        *published_component_count = published_component_count
            .checked_add(1)
            .ok_or_else(|| InternalError::resource_exhausted())?;
        *in_flight = None;
        RootComponentProvisioningStore::replace_operation(&current_record, next.clone())
            .map_err(map_commit_error)?;
        validated_record(next)
    }

    /// Freeze one complete response-idempotent root publication receipt.
    pub(crate) fn finalize_published(
        request: &RootComponentPublicationRequest,
        published_at_ns: u64,
    ) -> Result<RootComponentProvisioningView, InternalError> {
        let current_record = required_publishing_record(request)?;
        let current = validated_record(current_record.clone())?;
        validate_publication_request(request, &current)?;
        if current.published_component_count != current.component_count
            || current.publication_in_flight.is_some()
        {
            return Err(InternalError::conflict());
        }
        let RootComponentProvisioningStateRecordPhase::Publishing {
            placement_count,
            component_count,
            result,
            publication,
            accepted_at_ns,
            provisioned_at_ns,
            ..
        } = current_record.state.clone()
        else {
            unreachable!("required publishing record changed before finalization");
        };
        if published_at_ns < provisioned_at_ns {
            return Err(InternalError::invalid_input());
        }
        let result_view = provisioning_result_from_record(&result);
        let receipt_content_hash = RootComponentProvisioningReceiptOps::published_content_hash(
            RootComponentProvisioningPublishedReceiptAuthority {
                operation_id: current_record.operation_id,
                plan_hash: current_record.plan_hash,
                configuration_digest: current_record.configuration_digest,
                root: &current_record.batch.root,
                result: &result_view,
                publication: &publication,
                accepted_at_ns,
                provisioned_at_ns,
                published_at_ns,
            },
        )?;
        let mut next = current_record.clone();
        next.state = RootComponentProvisioningStateRecordPhase::Published {
            placement_count,
            component_count,
            result,
            publication,
            accepted_at_ns,
            provisioned_at_ns,
            published_at_ns,
            receipt_content_hash,
        };
        RootComponentProvisioningStore::replace_operation(&current_record, next.clone())
            .map_err(map_commit_error)?;
        validated_record(next)
    }

    /// Begin or replay activation after exact Directory publication is terminal.
    pub(crate) fn begin_activation(
        request: &RootComponentActivationRequest,
        started_at_ns: u64,
    ) -> Result<RootComponentProvisioningView, InternalError> {
        let current_record = RootComponentProvisioningStore::operation(request.operation_id)
            .ok_or_else(|| InternalError::unavailable())?;
        let current = validated_record(current_record.clone())?;
        validate_activation_request(request, &current)?;
        if current.phase == RootComponentProvisioningPhase::RuntimesActive
            || current.activation_started_at_ns.is_some()
        {
            return Ok(current);
        }
        let RootComponentProvisioningStateRecordPhase::Published {
            placement_count,
            component_count,
            result,
            publication,
            accepted_at_ns,
            provisioned_at_ns,
            published_at_ns,
            receipt_content_hash,
        } = current_record.state.clone()
        else {
            return Err(InternalError::conflict());
        };
        if started_at_ns < published_at_ns {
            return Err(InternalError::invalid_input());
        }
        let mut next = current_record.clone();
        next.state = RootComponentProvisioningStateRecordPhase::Activating {
            placement_count,
            component_count,
            result,
            publication,
            activated_component_count: 0,
            accepted_at_ns,
            provisioned_at_ns,
            published_at_ns,
            activation_started_at_ns: started_at_ns,
            published_receipt_content_hash: receipt_content_hash,
        };
        RootComponentProvisioningStore::replace_operation(&current_record, next.clone())
            .map_err(map_commit_error)?;
        validated_record(next)
    }

    /// Select the next exact Component whose runtime and membership must become active.
    pub(crate) fn next_activation_member(
        view: &RootComponentProvisioningView,
    ) -> Result<Option<RootComponentPublicationMemberView>, InternalError> {
        if view.activated_component_count == view.component_count {
            return Ok(None);
        }
        member_at_index(view, view.activated_component_count).map(Some)
    }

    /// Commit one exact Component only after its runtime, membership and current Directory agree.
    pub(crate) fn mark_member_activated(
        request: &RootComponentActivationRequest,
        member: &RootComponentPublicationMemberView,
    ) -> Result<RootComponentProvisioningView, InternalError> {
        let current_record = required_activating_record(request)?;
        let current = validated_record(current_record.clone())?;
        validate_activation_request(request, &current)?;
        if member.component_index != current.activated_component_count {
            return Err(InternalError::conflict());
        }
        let allocation = ComponentRegistryOps::allocation(member.member_operation_id)
            .ok_or_else(|| InternalError::unavailable())?;
        validate_activation_member_authority(&current, member, &allocation)?;
        validate_terminal_component_activation(member, &allocation)?;
        let mut next = current_record.clone();
        let RootComponentProvisioningStateRecordPhase::Activating {
            activated_component_count,
            ..
        } = &mut next.state
        else {
            unreachable!("required activating record changed before local mutation");
        };
        *activated_component_count = activated_component_count
            .checked_add(1)
            .ok_or_else(|| InternalError::resource_exhausted())?;
        RootComponentProvisioningStore::replace_operation(&current_record, next.clone())
            .map_err(map_commit_error)?;
        validated_record(next)
    }

    /// Freeze one complete response-idempotent root runtime-activation receipt.
    pub(crate) fn finalize_runtimes_active(
        request: &RootComponentActivationRequest,
        activation: RootComponentActivationEvidence,
        runtimes_activated_at_ns: u64,
    ) -> Result<RootComponentProvisioningView, InternalError> {
        let current_record = required_activating_record(request)?;
        let current = validated_record(current_record.clone())?;
        validate_activation_request(request, &current)?;
        if current.activated_component_count != current.component_count {
            return Err(InternalError::conflict());
        }
        let RootComponentProvisioningStateRecordPhase::Activating {
            placement_count,
            component_count,
            result,
            publication,
            accepted_at_ns,
            provisioned_at_ns,
            published_at_ns,
            activation_started_at_ns,
            published_receipt_content_hash,
            ..
        } = current_record.state.clone()
        else {
            unreachable!("required activating record changed before finalization");
        };
        let receipt_content_hash =
            RootComponentProvisioningReceiptOps::runtimes_active_content_hash(
                RootComponentProvisioningRuntimesActiveReceiptAuthority {
                    operation_id: current_record.operation_id,
                    plan_hash: current_record.plan_hash,
                    configuration_digest: current_record.configuration_digest,
                    root: &current_record.batch.root,
                    published_receipt_content_hash,
                    activation,
                    activation_started_at_ns,
                    runtimes_activated_at_ns,
                },
            )?;
        let mut next = current_record.clone();
        next.state = RootComponentProvisioningStateRecordPhase::RuntimesActive {
            placement_count,
            component_count,
            result,
            publication,
            activation,
            accepted_at_ns,
            provisioned_at_ns,
            published_at_ns,
            activation_started_at_ns,
            runtimes_activated_at_ns,
            published_receipt_content_hash,
            receipt_content_hash,
        };
        let _validated_terminal_state = validated_record_state(&next)?;
        RootComponentProvisioningStore::complete_operation(&current_record, next.clone())
            .map_err(map_commit_error)?;
        validated_record(next)
    }

    /// Number of distinct accepted or committed placements occupying the root ceiling.
    pub(crate) fn tracked_group_placements() -> Result<u32, InternalError> {
        Ok(validated_aggregate_state()?.tracked_group_placements)
    }

    /// Fence unrelated top-level allocations while one aggregate batch owns root capacity.
    pub(crate) fn require_ordinary_allocation_open() -> Result<(), InternalError> {
        let state = validated_aggregate_state()?;
        if state.active_operation_id.is_some() {
            return Err(InternalError::conflict());
        }
        Ok(())
    }

    /// Reject a different active aggregate operation before any fresh acceptance observation.
    pub(crate) fn require_acceptance_open(operation_id: [u8; 32]) -> Result<(), InternalError> {
        let state = validated_aggregate_state()?;
        match state.active_operation_id {
            None => Ok(()),
            Some(active) if active != operation_id => Err(InternalError::conflict()),
            Some(_) => Err(InternalError::invariant()),
        }
    }

    /// Keep a root with retained group placements out of ordinary root draining.
    pub(crate) fn require_root_draining_open() -> Result<(), InternalError> {
        let state = validated_aggregate_state()?;
        if state.active_operation_id.is_some() || state.tracked_group_placements != 0 {
            return Err(InternalError::conflict());
        }
        Ok(())
    }

    /// Revalidate one retained group origin against its immutable accepted member authority.
    pub(crate) fn validate_member_provisioning_origin(
        origin: &ComponentProvisioningOrigin,
        component_spec: &ComponentSpecId,
        spec_hash: [u8; 32],
    ) -> Result<(), InternalError> {
        Self::validated_member_origin(origin, component_spec, spec_hash).map(|_| ())
    }

    /// Reconstruct one retained group member's exact deployment and Directory authority.
    pub(crate) fn component_group_runtime_authority(
        allocation: &RootComponentAllocationView,
    ) -> Result<RootComponentGroupRuntimeAuthorityView, InternalError> {
        let (view, placement_index) = Self::validated_member_origin(
            &allocation.provisioning_origin,
            &allocation.component_spec,
            allocation.spec_hash,
        )?;
        let ComponentProvisioningOrigin::ComponentGroup { member_path, .. } =
            &allocation.provisioning_origin
        else {
            unreachable!("validated group origin changed before runtime reconstruction");
        };
        let member_index = view.batch.placements[placement_index]
            .entries
            .binary_search_by(|candidate| candidate.member_path.cmp(member_path))
            .map_err(|_| InternalError::invariant())?;
        let member = member_at_cursor(
            &view,
            u32::try_from(placement_index).map_err(|_| InternalError::resource_exhausted())?,
            u32::try_from(member_index).map_err(|_| InternalError::resource_exhausted())?,
        )?;
        let deployment = Self::member_deployment_context(&view, &member, allocation)?;
        let result = view
            .result
            .as_ref()
            .ok_or_else(|| InternalError::invariant())?;
        let component_group =
            derive_component_group_directory_from_view(&view, result, placement_index)?;
        Ok(RootComponentGroupRuntimeAuthorityView {
            deployment,
            component_group,
        })
    }

    fn validated_member_origin(
        origin: &ComponentProvisioningOrigin,
        component_spec: &ComponentSpecId,
        spec_hash: [u8; 32],
    ) -> Result<(RootComponentProvisioningView, usize), InternalError> {
        let ComponentProvisioningOrigin::ComponentGroup {
            operation_id,
            plan_hash,
            group_placement,
            member_path,
        } = origin
        else {
            return Err(InternalError::invariant());
        };
        let view = Self::status(RootComponentProvisioningStatusRequest {
            operation_id: *operation_id,
            plan_hash: *plan_hash,
        })?;
        let (_placement, entry) = accepted_member(&view, group_placement, member_path)?;
        if &entry.component_spec != component_spec || entry.spec_hash != spec_hash {
            return Err(InternalError::invariant());
        }
        let placement_index = view
            .batch
            .placements
            .binary_search_by(|candidate| candidate.group_placement.cmp(group_placement))
            .map_err(|_| InternalError::invariant())?;
        Ok((view, placement_index))
    }
}

fn required_publishing_record(
    request: &RootComponentPublicationRequest,
) -> Result<RootComponentProvisioningRecord, InternalError> {
    let record = RootComponentProvisioningStore::operation(request.operation_id)
        .ok_or_else(|| InternalError::unavailable())?;
    if !matches!(
        record.state,
        RootComponentProvisioningStateRecordPhase::Publishing { .. }
    ) {
        return Err(InternalError::conflict());
    }
    Ok(record)
}

fn required_activating_record(
    request: &RootComponentActivationRequest,
) -> Result<RootComponentProvisioningRecord, InternalError> {
    let record = RootComponentProvisioningStore::operation(request.operation_id)
        .ok_or_else(|| InternalError::unavailable())?;
    if !matches!(
        record.state,
        RootComponentProvisioningStateRecordPhase::Activating { .. }
    ) {
        return Err(InternalError::conflict());
    }
    Ok(record)
}

fn member_at_index(
    view: &RootComponentProvisioningView,
    target_index: u32,
) -> Result<RootComponentPublicationMemberView, InternalError> {
    let result = view
        .result
        .as_ref()
        .ok_or_else(|| InternalError::invariant())?;
    let mut flat_index = 0_u32;
    for (placement_index, (planned, provisioned)) in view
        .batch
        .placements
        .iter()
        .zip(&result.placements)
        .enumerate()
    {
        for (entry, member) in planned.entries.iter().zip(&provisioned.members) {
            if flat_index == target_index {
                return Ok(RootComponentPublicationMemberView {
                    component_index: flat_index,
                    member_operation_id: member_operation_id(
                        view.batch.root.fleet_subnet_root,
                        view.operation_id,
                        view.plan_hash,
                        &planned.group_placement,
                        &entry.member_path,
                    )?,
                    binding: member.binding.clone(),
                    component_registry_revision: member.component_registry_revision,
                    component_registry_content_hash: member.component_registry_content_hash,
                    deployment: ProtectedComponentDeployment::GroupMember {
                        binding: member.binding.clone(),
                        configuration_digest: view.configuration_digest,
                        group_placement: planned.group_placement.clone(),
                        component_group: planned.component_group.clone(),
                        member_path: entry.member_path.clone(),
                        purpose: entry.purpose.clone(),
                        labels: entry.labels.clone(),
                        limits: entry.limits.clone(),
                    },
                    component_group: derive_component_group_directory_from_view(
                        view,
                        result,
                        placement_index,
                    )?,
                });
            }
            flat_index = flat_index
                .checked_add(1)
                .ok_or_else(|| InternalError::resource_exhausted())?;
        }
    }
    Err(InternalError::invariant())
}

fn validate_publication_request(
    request: &RootComponentPublicationRequest,
    view: &RootComponentProvisioningView,
) -> Result<(), InternalError> {
    validate_operation_and_plan_hash(request.operation_id, request.plan_hash)?;
    let count_is_current =
        request.expected_published_component_count == view.published_component_count;
    let count_replays_last = request.expected_published_component_count.checked_add(1)
        == Some(view.published_component_count);
    let request_is_exact = [
        request.operation_id == view.operation_id,
        request.plan_hash == view.plan_hash,
        count_is_current || count_replays_last,
        request.published_fleet_registry.authority == view.fleet_registry.authority,
        request.published_fleet_registry.revision >= view.fleet_registry.revision,
        request.published_fleet_registry.content_hash != [0; 32],
    ]
    .into_iter()
    .all(|matches| matches);
    if !request_is_exact {
        return Err(InternalError::conflict());
    }
    if request.published_fleet_registry.revision == view.fleet_registry.revision
        && request.published_fleet_registry != view.fleet_registry
    {
        return Err(InternalError::conflict());
    }
    if let Some(publication) = &view.publication
        && publication.fleet_registry != request.published_fleet_registry
    {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn validate_activation_request(
    request: &RootComponentActivationRequest,
    view: &RootComponentProvisioningView,
) -> Result<(), InternalError> {
    validate_operation_and_plan_hash(request.operation_id, request.plan_hash)?;
    let cursor_is_current = request.expected_activated_component_count
        == view.activated_component_count
        && request.expected_root_runtime_active == view.root_runtime_active;
    let replays_component_activation = request.expected_activated_component_count.checked_add(1)
        == Some(view.activated_component_count)
        && !request.expected_root_runtime_active
        && !view.root_runtime_active;
    let replays_root_activation = request.expected_activated_component_count
        == view.activated_component_count
        && !request.expected_root_runtime_active
        && view.root_runtime_active;
    let progress_is_current_or_replayed = [
        cursor_is_current,
        replays_component_activation,
        replays_root_activation,
    ]
    .into_iter()
    .any(|matches| matches);
    let request_is_exact = [
        request.operation_id == view.operation_id,
        request.plan_hash == view.plan_hash,
        progress_is_current_or_replayed,
    ]
    .into_iter()
    .all(|matches| matches);
    if !request_is_exact {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn validate_terminal_component_activation(
    member: &RootComponentPublicationMemberView,
    allocation: &RootComponentAllocationView,
) -> Result<(), InternalError> {
    let RootComponentAllocationProgressView::Committed { commitment, .. } = &allocation.progress
    else {
        return Err(InternalError::conflict());
    };
    let membership = commitment
        .membership
        .as_ref()
        .ok_or_else(|| InternalError::conflict())?;
    if !commitment.runtime_activated || !membership.directory_synchronized {
        return Err(InternalError::conflict());
    }
    let partition = ComponentRegistryOps::active_membership_partition(member.member_operation_id)?;
    let active_authority_is_exact = [
        partition.binding == member.binding,
        partition.status == ComponentLifecycleStatus::Active,
        partition.directory_synchronized_at_ns == membership.directory_synchronized_at_ns,
    ]
    .into_iter()
    .all(|matches| matches);
    if !active_authority_is_exact {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn validate_activation_member_authority(
    view: &RootComponentProvisioningView,
    member: &RootComponentPublicationMemberView,
    allocation: &RootComponentAllocationView,
) -> Result<(), InternalError> {
    let ProtectedComponentDeployment::GroupMember {
        group_placement,
        member_path,
        ..
    } = &member.deployment
    else {
        return Err(InternalError::invariant());
    };
    let expected_origin = ComponentProvisioningOrigin::ComponentGroup {
        operation_id: view.operation_id,
        plan_hash: view.plan_hash,
        group_placement: group_placement.clone(),
        member_path: member_path.clone(),
    };
    let authority_is_exact = [
        allocation.operation_id == member.member_operation_id,
        allocation.component_spec == member.binding.component_spec,
        allocation.spec_hash == member.binding.spec_hash,
        allocation.provisioning_origin == expected_origin,
        allocation.release_set == view.batch.active_release_set,
    ]
    .into_iter()
    .all(|matches| matches);
    if !authority_is_exact {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn validate_acceptance_identity(
    request: &RootComponentProvisioningAcceptanceRequest,
    validation: &RootComponentProvisioningBatchValidation,
    accepted_at_ns: u64,
) -> Result<(), InternalError> {
    validate_operation_and_plan_hash(request.operation_id, request.plan_hash)?;
    if accepted_at_ns == 0 {
        return Err(InternalError::invalid_input());
    }
    let placement_count = u32::try_from(request.batch.placements.len())
        .map_err(|_| InternalError::resource_exhausted())?;
    let component_count = request
        .batch
        .placements
        .iter()
        .try_fold(0_u32, |total, placement| {
            total
                .checked_add(
                    u32::try_from(placement.entries.len())
                        .map_err(|_| InternalError::resource_exhausted())?,
                )
                .ok_or_else(|| InternalError::resource_exhausted())
        })?;
    if placement_count != validation.placement_count
        || component_count != validation.component_count
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_operation_and_plan_hash(
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
) -> Result<(), InternalError> {
    if operation_id == [0; 32] {
        return Err(InternalError::invalid_input());
    }
    if plan_hash == [0; 32] {
        return Err(InternalError::invalid_input());
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
        return Err(InternalError::invariant());
    }
    Ok(state)
}

struct ValidatedProvisioningState {
    phase: RootComponentProvisioningPhase,
    placement_count: u32,
    component_count: u32,
    cursors: ProvisioningCursorRecords,
    result: Option<RootComponentProvisioningResult>,
    publication: Option<RootComponentPublicationEvidence>,
    published_component_count: u32,
    activated_component_count: u32,
    root_runtime_active: bool,
    publication_in_flight: Option<RootComponentPublicationIntentView>,
    activation: Option<RootComponentActivationEvidence>,
    accepted_at_ns: u64,
    provisioned_at_ns: Option<u64>,
    publication_started_at_ns: Option<u64>,
    published_at_ns: Option<u64>,
    activation_started_at_ns: Option<u64>,
    runtimes_activated_at_ns: Option<u64>,
    receipt_content_hash: [u8; 32],
}

fn validated_record(
    record: RootComponentProvisioningRecord,
) -> Result<RootComponentProvisioningView, InternalError> {
    validate_operation_and_plan_hash(record.operation_id, record.plan_hash)?;
    let state = validated_record_state(&record)?;
    let validation = RootComponentProvisioningBatchValidation {
        placement_count: state.placement_count,
        component_count: state.component_count,
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
    validate_acceptance_identity(&request, &validation, state.accepted_at_ns)?;
    validate_record_cursors(
        record.operation_id,
        record.plan_hash,
        &record.batch,
        state.component_count,
        state.cursors,
    )?;
    validate_record_placement_index(record.operation_id, record.plan_hash, &record.batch)?;
    validate_aggregate_operation(&record, state.phase, validated_aggregate_state()?)?;
    Ok(RootComponentProvisioningView {
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        fleet_registry: record.fleet_registry,
        configuration_digest: record.configuration_digest,
        batch: record.batch,
        runtime_mode: record.runtime_mode.into(),
        placement_count: state.placement_count,
        component_count: state.component_count,
        reservation_cursor: RootComponentProvisioningReservationCursorView {
            placement_index: state.cursors.reservation.placement_index,
            member_index: state.cursors.reservation.member_index,
            reserved_component_count: state.cursors.reservation.reserved_component_count,
        },
        claim_cursor: RootComponentProvisioningClaimCursorView {
            placement_index: state.cursors.claim.placement_index,
            member_index: state.cursors.claim.member_index,
            claimed_component_count: state.cursors.claim.claimed_component_count,
        },
        install_cursor: RootComponentProvisioningInstallCursorView {
            placement_index: state.cursors.install.placement_index,
            member_index: state.cursors.install.member_index,
            installed_component_count: state.cursors.install.installed_component_count,
        },
        registry_cursor: RootComponentProvisioningRegistryCursorView {
            placement_index: state.cursors.registry.placement_index,
            member_index: state.cursors.registry.member_index,
            registry_committed_component_count: state
                .cursors
                .registry
                .registry_committed_component_count,
        },
        phase: state.phase,
        result: state.result,
        publication: state.publication,
        published_component_count: state.published_component_count,
        activated_component_count: state.activated_component_count,
        root_runtime_active: state.root_runtime_active,
        publication_in_flight: state.publication_in_flight,
        activation: state.activation,
        accepted_at_ns: state.accepted_at_ns,
        provisioned_at_ns: state.provisioned_at_ns,
        publication_started_at_ns: state.publication_started_at_ns,
        published_at_ns: state.published_at_ns,
        activation_started_at_ns: state.activation_started_at_ns,
        runtimes_activated_at_ns: state.runtimes_activated_at_ns,
        receipt_content_hash: state.receipt_content_hash,
    })
}

fn validate_aggregate_operation(
    record: &RootComponentProvisioningRecord,
    phase: RootComponentProvisioningPhase,
    aggregate: crate::storage::stable::component_provisioning::RootComponentProvisioningStateRecord,
) -> Result<(), InternalError> {
    let active_operation_is_exact = aggregate.active_operation_id == Some(record.operation_id);
    let aggregate_is_exact = match phase {
        RootComponentProvisioningPhase::Accepted
        | RootComponentProvisioningPhase::Provisioned
        | RootComponentProvisioningPhase::Published => active_operation_is_exact,
        RootComponentProvisioningPhase::RuntimesActive => !active_operation_is_exact,
    };
    if !aggregate_is_exact {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validated_record_state(
    record: &RootComponentProvisioningRecord,
) -> Result<ValidatedProvisioningState, InternalError> {
    match record.state.clone() {
        RootComponentProvisioningStateRecordPhase::Accepted {
            placement_count,
            component_count,
            reservation_cursor,
            claim_cursor,
            install_cursor,
            registry_cursor,
            accepted_at_ns,
            receipt_content_hash,
        } => validated_accepted_state(
            record,
            placement_count,
            component_count,
            ProvisioningCursorRecords {
                reservation: reservation_cursor,
                claim: claim_cursor,
                install: install_cursor,
                registry: registry_cursor,
            },
            accepted_at_ns,
            receipt_content_hash,
        ),
        RootComponentProvisioningStateRecordPhase::Provisioned {
            placement_count,
            component_count,
            result,
            accepted_at_ns,
            provisioned_at_ns,
            receipt_content_hash,
        } => validated_provisioned_state(
            record,
            placement_count,
            component_count,
            result,
            accepted_at_ns,
            provisioned_at_ns,
            receipt_content_hash,
        ),
        RootComponentProvisioningStateRecordPhase::Publishing {
            placement_count,
            component_count,
            result,
            publication,
            published_component_count,
            in_flight,
            accepted_at_ns,
            provisioned_at_ns,
            publication_started_at_ns,
            provisioned_receipt_content_hash,
        } => validated_publishing_state(
            record,
            placement_count,
            component_count,
            result,
            publication,
            published_component_count,
            in_flight,
            accepted_at_ns,
            provisioned_at_ns,
            publication_started_at_ns,
            provisioned_receipt_content_hash,
        ),
        RootComponentProvisioningStateRecordPhase::Published {
            placement_count,
            component_count,
            result,
            publication,
            accepted_at_ns,
            provisioned_at_ns,
            published_at_ns,
            receipt_content_hash,
        } => validated_published_state(
            record,
            placement_count,
            component_count,
            result,
            publication,
            accepted_at_ns,
            provisioned_at_ns,
            published_at_ns,
            receipt_content_hash,
        ),
        state @ (RootComponentProvisioningStateRecordPhase::Activating { .. }
        | RootComponentProvisioningStateRecordPhase::RuntimesActive { .. }) => {
            validated_runtime_record_state(record, state)
        }
    }
}

fn validated_runtime_record_state(
    record: &RootComponentProvisioningRecord,
    state: RootComponentProvisioningStateRecordPhase,
) -> Result<ValidatedProvisioningState, InternalError> {
    match state {
        RootComponentProvisioningStateRecordPhase::Activating {
            placement_count,
            component_count,
            result,
            publication,
            activated_component_count,
            accepted_at_ns,
            provisioned_at_ns,
            published_at_ns,
            activation_started_at_ns,
            published_receipt_content_hash,
        } => validated_activating_state(
            record,
            ActivatingStateFields {
                placement_count,
                component_count,
                result,
                publication,
                activated_component_count,
                accepted_at_ns,
                provisioned_at_ns,
                published_at_ns,
                activation_started_at_ns,
                published_receipt_content_hash,
            },
        ),
        RootComponentProvisioningStateRecordPhase::RuntimesActive {
            placement_count,
            component_count,
            result,
            publication,
            activation,
            accepted_at_ns,
            provisioned_at_ns,
            published_at_ns,
            activation_started_at_ns,
            runtimes_activated_at_ns,
            published_receipt_content_hash,
            receipt_content_hash,
        } => validated_runtimes_active_state(
            record,
            RuntimesActiveStateFields {
                placement_count,
                component_count,
                result,
                publication,
                activation,
                accepted_at_ns,
                provisioned_at_ns,
                published_at_ns,
                activation_started_at_ns,
                runtimes_activated_at_ns,
                published_receipt_content_hash,
                receipt_content_hash,
            },
        ),
        _ => unreachable!("only runtime activation states delegate here"),
    }
}

fn validated_accepted_state(
    record: &RootComponentProvisioningRecord,
    placement_count: u32,
    component_count: u32,
    cursors: ProvisioningCursorRecords,
    accepted_at_ns: u64,
    receipt_content_hash: [u8; 32],
) -> Result<ValidatedProvisioningState, InternalError> {
    let request = acceptance_request(record);
    let expected_hash =
        acceptance_receipt_hash(&request, placement_count, component_count, accepted_at_ns)?;
    if receipt_content_hash != expected_hash {
        return Err(InternalError::invariant());
    }
    Ok(ValidatedProvisioningState {
        phase: RootComponentProvisioningPhase::Accepted,
        placement_count,
        component_count,
        cursors,
        result: None,
        publication: None,
        published_component_count: 0,
        activated_component_count: 0,
        root_runtime_active: false,
        publication_in_flight: None,
        activation: None,
        accepted_at_ns,
        provisioned_at_ns: None,
        publication_started_at_ns: None,
        published_at_ns: None,
        activation_started_at_ns: None,
        runtimes_activated_at_ns: None,
        receipt_content_hash,
    })
}

fn acceptance_request(
    record: &RootComponentProvisioningRecord,
) -> RootComponentProvisioningAcceptanceRequest {
    RootComponentProvisioningAcceptanceRequest {
        fleet_registry: record.fleet_registry.clone(),
        configuration_digest: record.configuration_digest,
        operation_id: record.operation_id,
        plan_hash: record.plan_hash,
        batch: record.batch.clone(),
    }
}

fn validated_provisioned_state(
    record: &RootComponentProvisioningRecord,
    placement_count: u32,
    component_count: u32,
    result_record: RootComponentProvisioningResultRecord,
    accepted_at_ns: u64,
    provisioned_at_ns: u64,
    receipt_content_hash: [u8; 32],
) -> Result<ValidatedProvisioningState, InternalError> {
    if provisioned_at_ns == 0 || provisioned_at_ns < accepted_at_ns {
        return Err(InternalError::invariant());
    }
    let result = provisioning_result_from_record(&result_record);
    validate_provisioned_result(&record.batch, component_count, &result)?;
    let expected_hash =
        provisioned_receipt_hash(record, &result, accepted_at_ns, provisioned_at_ns)?;
    if receipt_content_hash != expected_hash {
        return Err(InternalError::invariant());
    }
    let cursors = terminal_cursor_records(record, placement_count, component_count)?;
    Ok(ValidatedProvisioningState {
        phase: RootComponentProvisioningPhase::Provisioned,
        placement_count,
        component_count,
        cursors,
        result: Some(result),
        publication: None,
        published_component_count: 0,
        activated_component_count: 0,
        root_runtime_active: false,
        publication_in_flight: None,
        activation: None,
        accepted_at_ns,
        provisioned_at_ns: Some(provisioned_at_ns),
        publication_started_at_ns: None,
        published_at_ns: None,
        activation_started_at_ns: None,
        runtimes_activated_at_ns: None,
        receipt_content_hash,
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "the stable phase validator names every independently persisted publication field"
)]
fn validated_publishing_state(
    record: &RootComponentProvisioningRecord,
    placement_count: u32,
    component_count: u32,
    result_record: RootComponentProvisioningResultRecord,
    publication: RootComponentPublicationEvidence,
    published_component_count: u32,
    in_flight: Option<RootComponentPublicationIntentRecord>,
    accepted_at_ns: u64,
    provisioned_at_ns: u64,
    publication_started_at_ns: u64,
    provisioned_receipt_content_hash: [u8; 32],
) -> Result<ValidatedProvisioningState, InternalError> {
    let provisioned = validated_provisioned_state(
        record,
        placement_count,
        component_count,
        result_record,
        accepted_at_ns,
        provisioned_at_ns,
        provisioned_receipt_content_hash,
    )?;
    let result = provisioned
        .result
        .as_ref()
        .ok_or_else(|| InternalError::invariant())?;
    if publication_started_at_ns < provisioned_at_ns {
        return Err(InternalError::invariant());
    }
    validate_partial_publication(
        record,
        result,
        &publication,
        published_component_count,
        in_flight.as_ref(),
        publication_started_at_ns,
    )?;
    Ok(ValidatedProvisioningState {
        phase: RootComponentProvisioningPhase::Provisioned,
        placement_count,
        component_count,
        cursors: provisioned.cursors,
        result: Some(result.clone()),
        publication: Some(publication),
        published_component_count,
        activated_component_count: 0,
        root_runtime_active: false,
        publication_in_flight: in_flight.map(publication_intent_to_view),
        activation: None,
        accepted_at_ns,
        provisioned_at_ns: Some(provisioned_at_ns),
        publication_started_at_ns: Some(publication_started_at_ns),
        published_at_ns: None,
        activation_started_at_ns: None,
        runtimes_activated_at_ns: None,
        receipt_content_hash: provisioned_receipt_content_hash,
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "the stable phase validator names every independently persisted terminal field"
)]
fn validated_published_state(
    record: &RootComponentProvisioningRecord,
    placement_count: u32,
    component_count: u32,
    result_record: RootComponentProvisioningResultRecord,
    publication: RootComponentPublicationEvidence,
    accepted_at_ns: u64,
    provisioned_at_ns: u64,
    published_at_ns: u64,
    receipt_content_hash: [u8; 32],
) -> Result<ValidatedProvisioningState, InternalError> {
    let result = provisioning_result_from_record(&result_record);
    validate_provisioned_result(&record.batch, component_count, &result)?;
    if published_at_ns < provisioned_at_ns {
        return Err(InternalError::invariant());
    }
    validate_partial_publication(
        record,
        &result,
        &publication,
        component_count,
        None,
        provisioned_at_ns,
    )?;
    let expected = RootComponentProvisioningReceiptOps::published_content_hash(
        RootComponentProvisioningPublishedReceiptAuthority {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            configuration_digest: record.configuration_digest,
            root: &record.batch.root,
            result: &result,
            publication: &publication,
            accepted_at_ns,
            provisioned_at_ns,
            published_at_ns,
        },
    )?;
    if receipt_content_hash != expected {
        return Err(InternalError::invariant());
    }
    Ok(ValidatedProvisioningState {
        phase: RootComponentProvisioningPhase::Published,
        placement_count,
        component_count,
        cursors: terminal_cursor_records(record, placement_count, component_count)?,
        result: Some(result),
        publication: Some(publication),
        published_component_count: component_count,
        activated_component_count: 0,
        root_runtime_active: false,
        publication_in_flight: None,
        activation: None,
        accepted_at_ns,
        provisioned_at_ns: Some(provisioned_at_ns),
        publication_started_at_ns: Some(provisioned_at_ns),
        published_at_ns: Some(published_at_ns),
        activation_started_at_ns: None,
        runtimes_activated_at_ns: None,
        receipt_content_hash,
    })
}

struct ActivatingStateFields {
    placement_count: u32,
    component_count: u32,
    result: RootComponentProvisioningResultRecord,
    publication: RootComponentPublicationEvidence,
    activated_component_count: u32,
    accepted_at_ns: u64,
    provisioned_at_ns: u64,
    published_at_ns: u64,
    activation_started_at_ns: u64,
    published_receipt_content_hash: [u8; 32],
}

fn validated_activating_state(
    record: &RootComponentProvisioningRecord,
    fields: ActivatingStateFields,
) -> Result<ValidatedProvisioningState, InternalError> {
    let published = validated_published_state(
        record,
        fields.placement_count,
        fields.component_count,
        fields.result,
        fields.publication,
        fields.accepted_at_ns,
        fields.provisioned_at_ns,
        fields.published_at_ns,
        fields.published_receipt_content_hash,
    )?;
    if fields.activation_started_at_ns < fields.published_at_ns
        || fields.activated_component_count > fields.component_count
    {
        return Err(InternalError::invariant());
    }
    Ok(ValidatedProvisioningState {
        phase: RootComponentProvisioningPhase::Published,
        placement_count: published.placement_count,
        component_count: published.component_count,
        cursors: published.cursors,
        result: published.result,
        publication: published.publication,
        published_component_count: published.published_component_count,
        activated_component_count: fields.activated_component_count,
        root_runtime_active: false,
        publication_in_flight: None,
        activation: None,
        accepted_at_ns: published.accepted_at_ns,
        provisioned_at_ns: published.provisioned_at_ns,
        publication_started_at_ns: published.publication_started_at_ns,
        published_at_ns: published.published_at_ns,
        activation_started_at_ns: Some(fields.activation_started_at_ns),
        runtimes_activated_at_ns: None,
        receipt_content_hash: fields.published_receipt_content_hash,
    })
}

struct RuntimesActiveStateFields {
    placement_count: u32,
    component_count: u32,
    result: RootComponentProvisioningResultRecord,
    publication: RootComponentPublicationEvidence,
    activation: RootComponentActivationEvidence,
    accepted_at_ns: u64,
    provisioned_at_ns: u64,
    published_at_ns: u64,
    activation_started_at_ns: u64,
    runtimes_activated_at_ns: u64,
    published_receipt_content_hash: [u8; 32],
    receipt_content_hash: [u8; 32],
}

fn validated_runtimes_active_state(
    record: &RootComponentProvisioningRecord,
    fields: RuntimesActiveStateFields,
) -> Result<ValidatedProvisioningState, InternalError> {
    let published = validated_published_state(
        record,
        fields.placement_count,
        fields.component_count,
        fields.result,
        fields.publication,
        fields.accepted_at_ns,
        fields.provisioned_at_ns,
        fields.published_at_ns,
        fields.published_receipt_content_hash,
    )?;
    let activation_order_is_valid = fields.activation_started_at_ns >= fields.published_at_ns
        && fields.runtimes_activated_at_ns >= fields.activation_started_at_ns;
    let activation_identity_is_valid = fields.activation.component_count == fields.component_count
        && fields.activation.fleet_activation_operation_id != [0; 32]
        && fields.activation.initial_inventory_hash != [0; 32];
    let runtime_mode_is_valid = match record.runtime_mode {
        RootComponentProvisioningRuntimeModeRecord::FreshRoot => {
            fields.activation.root_activated_at_ns == fields.runtimes_activated_at_ns
        }
        RootComponentProvisioningRuntimeModeRecord::ActiveRoot => {
            fields.activation.root_activated_at_ns > 0
                && fields.activation.root_activated_at_ns <= fields.accepted_at_ns
        }
    };
    let activation_is_exact =
        activation_order_is_valid && activation_identity_is_valid && runtime_mode_is_valid;
    if !activation_is_exact {
        return Err(InternalError::invariant());
    }
    let expected = RootComponentProvisioningReceiptOps::runtimes_active_content_hash(
        RootComponentProvisioningRuntimesActiveReceiptAuthority {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            configuration_digest: record.configuration_digest,
            root: &record.batch.root,
            published_receipt_content_hash: fields.published_receipt_content_hash,
            activation: fields.activation,
            activation_started_at_ns: fields.activation_started_at_ns,
            runtimes_activated_at_ns: fields.runtimes_activated_at_ns,
        },
    )?;
    if fields.receipt_content_hash != expected {
        return Err(InternalError::invariant());
    }
    Ok(ValidatedProvisioningState {
        phase: RootComponentProvisioningPhase::RuntimesActive,
        placement_count: published.placement_count,
        component_count: published.component_count,
        cursors: published.cursors,
        result: published.result,
        publication: published.publication,
        published_component_count: published.published_component_count,
        activated_component_count: published.component_count,
        root_runtime_active: true,
        publication_in_flight: None,
        activation: Some(fields.activation),
        accepted_at_ns: published.accepted_at_ns,
        provisioned_at_ns: published.provisioned_at_ns,
        publication_started_at_ns: published.publication_started_at_ns,
        published_at_ns: published.published_at_ns,
        activation_started_at_ns: Some(fields.activation_started_at_ns),
        runtimes_activated_at_ns: Some(fields.runtimes_activated_at_ns),
        receipt_content_hash: fields.receipt_content_hash,
    })
}

impl From<RootComponentProvisioningRuntimeMode> for RootComponentProvisioningRuntimeModeRecord {
    fn from(mode: RootComponentProvisioningRuntimeMode) -> Self {
        match mode {
            RootComponentProvisioningRuntimeMode::FreshRoot => Self::FreshRoot,
            RootComponentProvisioningRuntimeMode::ActiveRoot => Self::ActiveRoot,
        }
    }
}

impl From<RootComponentProvisioningRuntimeModeRecord> for RootComponentProvisioningRuntimeMode {
    fn from(mode: RootComponentProvisioningRuntimeModeRecord) -> Self {
        match mode {
            RootComponentProvisioningRuntimeModeRecord::FreshRoot => Self::FreshRoot,
            RootComponentProvisioningRuntimeModeRecord::ActiveRoot => Self::ActiveRoot,
        }
    }
}

fn validate_partial_publication(
    record: &RootComponentProvisioningRecord,
    result: &RootComponentProvisioningResult,
    publication: &RootComponentPublicationEvidence,
    published_component_count: u32,
    in_flight: Option<&RootComponentPublicationIntentRecord>,
    publication_started_at_ns: u64,
) -> Result<(), InternalError> {
    if publication.fleet_registry.authority != record.fleet_registry.authority
        || publication.fleet_registry.revision < record.fleet_registry.revision
        || publication.fleet_registry.content_hash == [0; 32]
        || publication.fleet_directory_content_hash == [0; 32]
    {
        return Err(InternalError::invariant());
    }
    if publication.fleet_registry.revision == record.fleet_registry.revision
        && publication.fleet_registry != record.fleet_registry
    {
        return Err(InternalError::invariant());
    }
    let published_count = usize::try_from(published_component_count)
        .map_err(|_| InternalError::resource_exhausted())?;
    let component_count = result
        .placements
        .iter()
        .map(|placement| placement.members.len())
        .sum::<usize>();
    if published_count > component_count {
        return Err(InternalError::invariant());
    }
    let expected_members = result
        .placements
        .iter()
        .flat_map(|placement| &placement.members)
        .take(published_count);
    if publication.component_directories.len() != published_count {
        return Err(InternalError::invariant());
    }
    for (member, evidence) in expected_members.zip(&publication.component_directories) {
        if evidence.component != member.binding.component
            || evidence.content_hash != member.component_registry_content_hash
        {
            return Err(InternalError::invariant());
        }
    }
    if publication.component_group_directories.len() != result.placements.len() {
        return Err(InternalError::invariant());
    }
    for (index, (placement, evidence)) in result
        .placements
        .iter()
        .zip(&publication.component_group_directories)
        .enumerate()
    {
        let directory = derive_component_group_directory(record, result, index)?;
        let expected_hash =
            RootComponentProvisioningReceiptOps::component_group_directory_content_hash(
                &directory,
            )?;
        if evidence.group_placement != placement.group_placement
            || evidence.content_hash != expected_hash
        {
            return Err(InternalError::invariant());
        }
    }
    if let Some(intent) = in_flight {
        let member = result_member_at(result, intent.component_index)?;
        if intent.component_index != published_component_count
            || intent.canister_id != member.binding.canister_id
            || intent.directory_authority_hash == [0; 32]
            || intent.started_at_ns < publication_started_at_ns
        {
            return Err(InternalError::invariant());
        }
    }
    Ok(())
}

const fn publication_intent_to_view(
    intent: RootComponentPublicationIntentRecord,
) -> RootComponentPublicationIntentView {
    RootComponentPublicationIntentView {
        component_index: intent.component_index,
        canister_id: intent.canister_id,
        directory_authority_hash: intent.directory_authority_hash,
        started_at_ns: intent.started_at_ns,
    }
}

fn terminal_cursor_records(
    record: &RootComponentProvisioningRecord,
    placement_count: u32,
    component_count: u32,
) -> Result<ProvisioningCursorRecords, InternalError> {
    Ok(ProvisioningCursorRecords {
        reservation: reservation_cursor_record(
            record.operation_id,
            record.plan_hash,
            placement_count,
            0,
            component_count,
        )?,
        claim: claim_cursor_record(
            record.operation_id,
            record.plan_hash,
            placement_count,
            0,
            component_count,
        )?,
        install: install_cursor_record(
            record.operation_id,
            record.plan_hash,
            placement_count,
            0,
            component_count,
        )?,
        registry: registry_cursor_record(
            record.operation_id,
            record.plan_hash,
            placement_count,
            0,
            component_count,
        )?,
    })
}

fn provisioned_member_evidence(
    view: &RootComponentProvisioningView,
) -> Result<Vec<ProvisionedMemberEvidence>, InternalError> {
    if view.phase != RootComponentProvisioningPhase::Accepted
        || view.registry_cursor.registry_committed_component_count != view.component_count
    {
        return Err(InternalError::conflict());
    }
    let capacity =
        usize::try_from(view.component_count).map_err(|_| InternalError::resource_exhausted())?;
    let mut evidence = Vec::with_capacity(capacity);
    for (placement_index, placement) in view.batch.placements.iter().enumerate() {
        for member_index in 0..placement.entries.len() {
            let member = member_at_cursor(
                view,
                u32::try_from(placement_index).map_err(|_| InternalError::resource_exhausted())?,
                u32::try_from(member_index).map_err(|_| InternalError::resource_exhausted())?,
            )?;
            let allocation = ComponentRegistryOps::allocation(member.member_operation_id)
                .ok_or_else(|| InternalError::invariant())?;
            let partition = ComponentRegistryOps::partition(allocation.component)?
                .ok_or_else(|| InternalError::invariant())?;
            validate_registry_committed_member(view, &member, &allocation, &partition)?;
            evidence.push(ProvisionedMemberEvidence {
                member,
                allocation,
                partition,
            });
        }
    }
    Ok(evidence)
}

fn provisioned_result_record(
    view: &RootComponentProvisioningView,
    evidence: &[ProvisionedMemberEvidence],
) -> Result<RootComponentProvisioningResultRecord, InternalError> {
    if evidence.len()
        != usize::try_from(view.component_count).map_err(|_| InternalError::resource_exhausted())?
    {
        return Err(InternalError::invariant());
    }
    let mut evidence = evidence.iter();
    let mut placements = Vec::with_capacity(view.batch.placements.len());
    for placement in &view.batch.placements {
        let mut members = Vec::with_capacity(placement.entries.len());
        for entry in &placement.entries {
            let observed = evidence.next().ok_or_else(|| InternalError::invariant())?;
            let expected_member = (&placement.group_placement, &entry.member_path);
            let observed_member = (
                &observed.member.group_placement,
                &observed.member.member_path,
            );
            if observed_member != expected_member {
                return Err(InternalError::conflict());
            }
            validate_registry_committed_member(
                view,
                &observed.member,
                &observed.allocation,
                &observed.partition,
            )?;
            members.push(RootProvisionedGroupMemberRecord {
                member_path: entry.member_path.clone(),
                component_spec: entry.component_spec.clone(),
                purpose: entry.purpose.clone(),
                limits: entry.limits.clone(),
                binding: observed.partition.binding.clone(),
                component_registry_revision: observed.partition.revision,
                component_registry_content_hash: observed.partition.content_hash,
            });
        }
        placements.push(RootProvisionedGroupPlacementRecord {
            group_placement: placement.group_placement.clone(),
            component_group: placement.component_group.clone(),
            members,
        });
    }
    if evidence.next().is_some() {
        return Err(InternalError::invariant());
    }
    let result = RootComponentProvisioningResultRecord { placements };
    validate_provisioned_result(
        &view.batch,
        view.component_count,
        &provisioning_result_from_record(&result),
    )?;
    Ok(result)
}

fn commit_provisioned_result(
    request: RootComponentProvisioningAdvanceRequest,
    provisioned_at_ns: u64,
    result: RootComponentProvisioningResultRecord,
) -> Result<RootComponentProvisioningView, InternalError> {
    let current_record = RootComponentProvisioningStore::operation(request.operation_id)
        .ok_or_else(|| InternalError::unavailable())?;
    let current = validated_record(current_record.clone())?;
    if RootComponentProvisioningOps::advance_disposition(request, &current)?
        != RootComponentProvisioningAdvanceDisposition::Advance
        || current.phase != RootComponentProvisioningPhase::Accepted
        || current.registry_cursor.registry_committed_component_count != current.component_count
    {
        return Err(InternalError::conflict());
    }
    if provisioned_at_ns == 0 || provisioned_at_ns < current.accepted_at_ns {
        return Err(InternalError::invalid_input());
    }
    let result_view = provisioning_result_from_record(&result);
    validate_provisioned_result(&current.batch, current.component_count, &result_view)?;
    let receipt_content_hash = provisioned_receipt_hash(
        &current_record,
        &result_view,
        current.accepted_at_ns,
        provisioned_at_ns,
    )?;
    let next = RootComponentProvisioningRecord {
        state: RootComponentProvisioningStateRecordPhase::Provisioned {
            placement_count: current.placement_count,
            component_count: current.component_count,
            result,
            accepted_at_ns: current.accepted_at_ns,
            provisioned_at_ns,
            receipt_content_hash,
        },
        ..current_record.clone()
    };
    RootComponentProvisioningStore::replace_operation(&current_record, next.clone())
        .map_err(map_commit_error)?;
    validated_record(next)
}

fn provisioning_result_from_record(
    result: &RootComponentProvisioningResultRecord,
) -> RootComponentProvisioningResult {
    RootComponentProvisioningResult {
        placements: result
            .placements
            .iter()
            .map(|placement| RootProvisionedGroupPlacement {
                group_placement: placement.group_placement.clone(),
                component_group: placement.component_group.clone(),
                members: placement
                    .members
                    .iter()
                    .map(|member| RootProvisionedGroupMember {
                        member_path: member.member_path.clone(),
                        component_spec: member.component_spec.clone(),
                        purpose: member.purpose.clone(),
                        limits: member.limits.clone(),
                        binding: member.binding.clone(),
                        component_registry_revision: member.component_registry_revision,
                        component_registry_content_hash: member.component_registry_content_hash,
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn derive_component_group_directory(
    record: &RootComponentProvisioningRecord,
    result: &RootComponentProvisioningResult,
    placement_index: usize,
) -> Result<ComponentGroupDirectory, InternalError> {
    derive_component_group_directory_from_parts(
        record.operation_id,
        record.plan_hash,
        &record.batch,
        result,
        placement_index,
    )
}

fn derive_component_group_directory_from_view(
    view: &RootComponentProvisioningView,
    result: &RootComponentProvisioningResult,
    placement_index: usize,
) -> Result<ComponentGroupDirectory, InternalError> {
    derive_component_group_directory_from_parts(
        view.operation_id,
        view.plan_hash,
        &view.batch,
        result,
        placement_index,
    )
}

fn derive_component_group_directory_from_parts(
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    batch: &canic_core::dto::component_provisioning::FleetSubnetRootProvisioningBatch,
    result: &RootComponentProvisioningResult,
    placement_index: usize,
) -> Result<ComponentGroupDirectory, InternalError> {
    let planned = batch
        .placements
        .get(placement_index)
        .ok_or_else(|| InternalError::invariant())?;
    let provisioned = result
        .placements
        .get(placement_index)
        .ok_or_else(|| InternalError::invariant())?;
    let placement_matches = [
        planned.group_placement == provisioned.group_placement,
        planned.component_group == provisioned.component_group,
        planned.entries.len() == provisioned.members.len(),
    ]
    .into_iter()
    .all(|matches| matches);
    if !placement_matches {
        return Err(InternalError::invariant());
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
                return Err(InternalError::invariant());
            }
            Ok(ComponentGroupDirectoryMember {
                member_path: member.member_path.clone(),
                component_spec: member.component_spec.clone(),
                purpose: member.purpose.clone(),
                labels: entry.labels.clone(),
                binding: member.binding.clone(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ComponentGroupDirectory {
        provenance: ComponentGroupDirectoryProvenance {
            authority: batch.root.authority.clone(),
            fleet_subnet_root: batch.root.fleet_subnet_root,
            group_placement: provisioned.group_placement.clone(),
            component_group: provisioned.component_group.clone(),
            operation_id,
            plan_hash,
            placement_receipt_content_hash:
                RootComponentProvisioningReceiptOps::group_placement_content_hash(
                    operation_id,
                    plan_hash,
                    &batch.root,
                    provisioned,
                )?,
        },
        members,
    })
}

fn result_member_at(
    result: &RootComponentProvisioningResult,
    component_index: u32,
) -> Result<&RootProvisionedGroupMember, InternalError> {
    let index =
        usize::try_from(component_index).map_err(|_| InternalError::resource_exhausted())?;
    result
        .placements
        .iter()
        .flat_map(|placement| &placement.members)
        .nth(index)
        .ok_or_else(|| InternalError::invariant())
}

fn validate_provisioned_result(
    batch: &canic_core::dto::component_provisioning::FleetSubnetRootProvisioningBatch,
    component_count: u32,
    result: &RootComponentProvisioningResult,
) -> Result<(), InternalError> {
    if result.placements.len() != batch.placements.len() {
        return Err(InternalError::invariant());
    }
    let mut components = BTreeSet::new();
    let mut principals = BTreeSet::new();
    let mut observed_count = 0_u32;
    for (planned, provisioned) in batch.placements.iter().zip(&result.placements) {
        let expected = ProvisionedPlacementAuthority {
            group_placement: &planned.group_placement,
            component_group: &planned.component_group,
            member_count: planned.entries.len(),
        };
        let actual = ProvisionedPlacementAuthority {
            group_placement: &provisioned.group_placement,
            component_group: &provisioned.component_group,
            member_count: provisioned.members.len(),
        };
        if actual != expected {
            return Err(InternalError::invariant());
        }
        for (entry, member) in planned.entries.iter().zip(&provisioned.members) {
            validate_provisioned_result_member(batch, entry, member)?;
            if !components.insert(member.binding.component)
                || !principals.insert(member.binding.canister_id)
            {
                return Err(InternalError::invariant());
            }
            observed_count = observed_count
                .checked_add(1)
                .ok_or_else(|| InternalError::resource_exhausted())?;
        }
    }
    if observed_count != component_count {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn validate_provisioned_result_member(
    batch: &canic_core::dto::component_provisioning::FleetSubnetRootProvisioningBatch,
    entry: &canic_core::dto::component_provisioning::ComponentGroupPlanEntry,
    member: &RootProvisionedGroupMember,
) -> Result<(), InternalError> {
    let binding = &member.binding;
    let expected = ProvisionedResultMemberAuthority {
        member_path: &entry.member_path,
        component_spec: &entry.component_spec,
        purpose: &entry.purpose,
        limits: &entry.limits,
        binding_authority: &batch.root.authority,
        binding_component_spec: &entry.component_spec,
        binding_spec_hash: entry.spec_hash,
        binding_placement_subnet: batch.root.placement_subnet,
        binding_root: batch.root.fleet_subnet_root,
    };
    let actual = ProvisionedResultMemberAuthority {
        member_path: &member.member_path,
        component_spec: &member.component_spec,
        purpose: &member.purpose,
        limits: &member.limits,
        binding_authority: &binding.authority,
        binding_component_spec: &binding.component_spec,
        binding_spec_hash: binding.spec_hash,
        binding_placement_subnet: binding.placement_subnet,
        binding_root: binding.fleet_subnet_root,
    };
    let identity_is_qualified = provisioned_result_identity_is_qualified(member);
    if actual != expected || !identity_is_qualified {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn provisioned_result_identity_is_qualified(member: &RootProvisionedGroupMember) -> bool {
    if member.binding.component.as_bytes() == &[0; 32] {
        return false;
    }
    if member.binding.canister_id == Principal::anonymous() {
        return false;
    }
    if member.component_registry_revision == 0 {
        return false;
    }
    member.component_registry_content_hash != [0; 32]
}

fn provisioned_receipt_hash(
    record: &RootComponentProvisioningRecord,
    result: &RootComponentProvisioningResult,
    accepted_at_ns: u64,
    provisioned_at_ns: u64,
) -> Result<[u8; 32], InternalError> {
    RootComponentProvisioningReceiptOps::provisioned_content_hash(
        RootComponentProvisioningProvisionedReceiptAuthority {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            fleet_registry: &record.fleet_registry,
            configuration_digest: record.configuration_digest,
            root: &record.batch.root,
            result,
            accepted_at_ns,
            provisioned_at_ns,
        },
    )
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
            return Err(InternalError::invariant());
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
        return Err(InternalError::invariant());
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
        return Err(InternalError::invariant());
    }
    if cursor.claimed_component_count > 0 && reserved_component_count != component_count {
        return Err(InternalError::invariant());
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
        return Err(InternalError::invariant());
    }
    if cursor.installed_component_count > 0 && claimed_component_count != component_count {
        return Err(InternalError::invariant());
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
        return Err(InternalError::invariant());
    }
    if cursor.registry_committed_component_count > 0 && installed_component_count != component_count
    {
        return Err(InternalError::invariant());
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
    _cursor_kind: &str,
) -> Result<(), InternalError> {
    if completed_count > component_count {
        return Err(InternalError::invariant());
    }
    let placement_count =
        u32::try_from(batch.placements.len()).map_err(|_| InternalError::invariant())?;
    if completed_count == component_count {
        if placement_index != placement_count || member_index != 0 {
            return Err(InternalError::invariant());
        }
        return Ok(());
    }
    let placement = batch
        .placements
        .get(usize::try_from(placement_index).map_err(|_| InternalError::invariant())?)
        .ok_or_else(|| InternalError::invariant())?;
    if usize::try_from(member_index)
        .ok()
        .is_none_or(|index| index >= placement.entries.len())
    {
        return Err(InternalError::invariant());
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
        .get(usize::try_from(placement_index).map_err(|_| InternalError::invariant())?)
        .ok_or_else(|| InternalError::invariant())?;
    let next_completed = completed_count
        .checked_add(1)
        .ok_or_else(|| InternalError::resource_exhausted())?;
    let next_member = member_index
        .checked_add(1)
        .ok_or_else(|| InternalError::resource_exhausted())?;
    let entry_count =
        u32::try_from(placement.entries.len()).map_err(|_| InternalError::invariant())?;
    if next_member == entry_count {
        let next_placement = placement_index
            .checked_add(1)
            .ok_or_else(|| InternalError::resource_exhausted())?;
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
        .get(usize::try_from(placement_index).map_err(|_| InternalError::invariant())?)
        .ok_or_else(|| InternalError::invariant())?;
    let entry = placement
        .entries
        .get(usize::try_from(member_index).map_err(|_| InternalError::invariant())?)
        .ok_or_else(|| InternalError::invariant())?;
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

fn member_by_path(
    view: &RootComponentProvisioningView,
    group_placement: &ComponentGroupPlacementId,
    member_path: &ComponentGroupMemberPath,
) -> Result<RootComponentProvisioningMemberView, InternalError> {
    let placement = view
        .batch
        .placements
        .iter()
        .find(|placement| &placement.group_placement == group_placement)
        .ok_or_else(|| InternalError::invariant())?;
    let entry = placement
        .entries
        .iter()
        .find(|entry| &entry.member_path == member_path)
        .ok_or_else(|| InternalError::invariant())?;
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
        return Err(InternalError::conflict());
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
        return Err(InternalError::conflict());
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
        return Err(InternalError::conflict());
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
        return Err(InternalError::invariant());
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
        return Err(InternalError::conflict());
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
        | RootComponentAllocationProgressView::CreationIntent(_) => Err(InternalError::conflict()),
        RootComponentAllocationProgressView::Removed { .. } => Err(InternalError::conflict()),
    }
}

fn domain_separated_candid_hash<T: CandidType>(
    domain: &[u8],
    value: T,
) -> Result<[u8; 32], InternalError> {
    let bytes = candid::encode_one(value).map_err(|_error| InternalError::invariant())?;
    let byte_count = u64::try_from(bytes.len()).map_err(|_| InternalError::resource_exhausted())?;
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
        .ok_or_else(|| InternalError::conflict())?;
    let entry = placement
        .entries
        .binary_search_by(|candidate| candidate.member_path.cmp(member_path))
        .ok()
        .map(|index| &placement.entries[index])
        .ok_or_else(|| InternalError::conflict())?;
    Ok((placement, entry))
}

fn acceptance_receipt_hash(
    request: &RootComponentProvisioningAcceptanceRequest,
    placement_count: u32,
    component_count: u32,
    accepted_at_ns: u64,
) -> Result<[u8; 32], InternalError> {
    RootComponentProvisioningReceiptOps::acceptance_content_hash(
        RootComponentProvisioningAcceptanceReceiptAuthority {
            operation_id: request.operation_id,
            plan_hash: request.plan_hash,
            fleet_registry: &request.fleet_registry,
            configuration_digest: request.configuration_digest,
            batch: &request.batch,
            placement_count,
            component_count,
            accepted_at_ns,
        },
    )
}

fn map_commit_error(error: RootComponentProvisioningCommitError) -> InternalError {
    match error {
        RootComponentProvisioningCommitError::ActiveOperationConflict => {
            InternalError::public(canic_core::diagnostics::codes::REQUEST_UNEXPECTED_STATE)
        }
        RootComponentProvisioningCommitError::ConflictingOperation => {
            InternalError::public(canic_core::diagnostics::codes::REQUEST_CONFLICT)
        }
        RootComponentProvisioningCommitError::OperationChanged => {
            InternalError::public(canic_core::diagnostics::codes::AUTHORITY_CONFLICT)
        }
        RootComponentProvisioningCommitError::PlacementConflict => {
            InternalError::public(canic_core::diagnostics::codes::POSITION_CONFLICT)
        }
        RootComponentProvisioningCommitError::PlacementCountOverflow => {
            InternalError::public(canic_core::diagnostics::codes::CAPACITY_LIMIT)
        }
    }
}

/// Convert one validated durable view to its exact boundary receipt.
pub fn status_response(
    view: RootComponentProvisioningView,
) -> RootComponentProvisioningStatusResponse {
    RootComponentProvisioningStatusResponse {
        operation_id: view.operation_id,
        plan_hash: view.plan_hash,
        fleet_registry: view.fleet_registry,
        configuration_digest: view.configuration_digest,
        fleet_subnet_root: view.batch.root.fleet_subnet_root,
        phase: view.phase,
        placement_count: view.placement_count,
        component_count: view.component_count,
        reserved_component_count: view.reservation_cursor.reserved_component_count,
        claimed_component_count: view.claim_cursor.claimed_component_count,
        installed_component_count: view.install_cursor.installed_component_count,
        registry_committed_component_count: view.registry_cursor.registry_committed_component_count,
        published_component_count: view.published_component_count,
        activated_component_count: view.activated_component_count,
        root_runtime_active: view.root_runtime_active,
        result: view.result,
        publication: view.publication,
        activation: view.activation,
        accepted_at_ns: view.accepted_at_ns,
        provisioned_at_ns: view.provisioned_at_ns,
        published_at_ns: view.published_at_ns,
        activation_started_at_ns: view.activation_started_at_ns,
        runtimes_activated_at_ns: view.runtimes_activated_at_ns,
        receipt_content_hash: view.receipt_content_hash,
    }
}
