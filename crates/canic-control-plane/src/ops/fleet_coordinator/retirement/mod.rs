//! Module: ops::fleet_coordinator::retirement
//!
//! Responsibility: retain final Coordinator Ledger evacuation under exact removed-Fleet authority.
//! Does not own: authentication, Ledger calls, canister deletion or replacement deployment.
//! Boundary: every transfer identity is durable before dispatch and immutable on retry.

use super::FleetCoordinatorOps;
use crate::storage::stable::fleet_coordinator::{
    FleetCoordinatorRegistryRecord, FleetRetirementRecord,
};
use candid::Principal;
use canic_core::{
    control_plane_support::{error::InternalError, ops::fleet_registry::FleetRegistryOps},
    dto::fleet_registry::{
        FleetLedgerTransferIntent, FleetLedgerTransferReceipt, FleetRetirementRequest,
        FleetRetirementStatus, FleetSubnetRootStatus,
    },
};

impl FleetCoordinatorOps {
    pub(crate) fn retirement_preflight(
        caller: Principal,
        request: FleetRetirementRequest,
        now_ns: u64,
    ) -> Result<FleetRetirementStatus, InternalError> {
        if request.destination != caller
            || caller == Principal::anonymous()
            || caller == Principal::management_canister()
            || request.operation_id == [0; 32]
        {
            return Err(InternalError::invalid_input());
        }
        let current = Self::current()?;
        if let Some(existing) = retirement_status(&current.retirement) {
            return if existing.request == request {
                Ok(existing)
            } else {
                Err(InternalError::conflict())
            };
        }
        Self::require_root_funding_snapshot_resumable()?;
        if Self::retains_operation_id(request.operation_id)? {
            return Err(InternalError::conflict());
        }
        validate_retirement_authority(&current, &request)?;
        Ok(FleetRetirementStatus {
            request,
            prepared_at_ns: now_ns,
            ledger_transfer: None,
            ledger_receipt: None,
        })
    }

    pub(crate) fn begin_retirement_transfer(
        caller: Principal,
        request: FleetRetirementRequest,
        balance_before: u128,
        fee: u128,
        created_at_time: u64,
    ) -> Result<FleetLedgerTransferIntent, InternalError> {
        let status = Self::retirement_preflight(caller, request, created_at_time)?;
        if let Some(existing) = status.ledger_transfer {
            return Ok(existing);
        }
        let current = Self::current()?;
        let transfer = FleetLedgerTransferIntent {
            source: current.authority.binding.coordinator,
            destination: status.request.destination,
            balance_before,
            fee,
            created_at_time,
            memo: status.request.operation_id,
        };
        let mut next = current.clone();
        next.retirement = FleetRetirementRecord::Transferring {
            request: status.request,
            prepared_at_ns: status.prepared_at_ns,
            transfer: transfer.clone(),
        };
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(transfer)
    }

    pub(crate) fn complete_retirement_transfer(
        operation_id: [u8; 32],
        block_index: Option<u128>,
    ) -> Result<FleetRetirementStatus, InternalError> {
        let current = Self::current()?;
        let status =
            retirement_status(&current.retirement).ok_or_else(InternalError::unavailable)?;
        if status.request.operation_id != operation_id {
            return Err(InternalError::conflict());
        }
        if let Some(receipt) = &status.ledger_receipt {
            return if receipt.block_index == block_index {
                Ok(status)
            } else {
                Err(InternalError::conflict())
            };
        }
        let transfer = status
            .ledger_transfer
            .ok_or_else(InternalError::unavailable)?;
        let mut next = current.clone();
        next.retirement = FleetRetirementRecord::Complete {
            request: status.request,
            prepared_at_ns: status.prepared_at_ns,
            receipt: FleetLedgerTransferReceipt {
                intent: transfer,
                block_index,
            },
        };
        let next = Self::validate_current(next)?;
        let status = retirement_status(&next.retirement).ok_or_else(InternalError::invariant)?;
        Self::commit_transition(&current, next)?;
        Ok(status)
    }
}

pub(super) fn retirement_status(record: &FleetRetirementRecord) -> Option<FleetRetirementStatus> {
    let (request, prepared_at_ns, transfer, receipt) = match record {
        FleetRetirementRecord::NotStarted => return None,
        FleetRetirementRecord::Transferring {
            request,
            prepared_at_ns,
            transfer,
        } => (request, prepared_at_ns, Some(transfer.clone()), None),
        FleetRetirementRecord::Complete {
            request,
            prepared_at_ns,
            receipt,
        } => (
            request,
            prepared_at_ns,
            Some(receipt.intent.clone()),
            Some(receipt.clone()),
        ),
    };
    Some(FleetRetirementStatus {
        request: request.clone(),
        prepared_at_ns: *prepared_at_ns,
        ledger_transfer: transfer,
        ledger_receipt: receipt,
    })
}

pub(super) fn validate_retirement(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    let Some(status) = retirement_status(&current.retirement) else {
        return Ok(());
    };
    validate_retirement_authority(current, &status.request)?;
    let transfer_is_valid = status.ledger_transfer.as_ref().is_none_or(|transfer| {
        let amount_is_valid = if transfer.balance_before == 0 {
            transfer.fee == 0
        } else {
            transfer.balance_before > transfer.fee && transfer.fee > 0
        };
        [
            transfer.source == current.authority.binding.coordinator,
            transfer.destination == status.request.destination,
            transfer.memo == status.request.operation_id,
            transfer.created_at_time >= status.prepared_at_ns,
            transfer.fee <= status.request.maximum_ledger_fee,
            amount_is_valid,
        ]
        .into_iter()
        .all(|valid| valid)
    });
    let receipt_is_valid = status
        .ledger_receipt
        .as_ref()
        .is_none_or(|receipt| receipt.block_index.is_some() == (receipt.intent.balance_before > 0));
    if !transfer_is_valid || !receipt_is_valid {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn validate_retirement_authority(
    current: &FleetCoordinatorRegistryRecord,
    request: &FleetRetirementRequest,
) -> Result<(), InternalError> {
    let version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &current.registry,
    )?;
    let roots_are_deleted = current.registry.fleet_subnet_roots.iter().all(|root| {
        root.status == FleetSubnetRootStatus::Removed
            && current
                .root_deletion_receipts
                .iter()
                .any(|receipt| receipt.fleet_subnet_root == root.fleet_subnet_root)
    });
    let assets_have_exact_owner = current
        .root_draining_reservations
        .iter()
        .all(|reservation| reservation.response.request.asset_recipient == request.destination);
    let authority_is_exact = [
        request.operation_id != [0; 32],
        request.expected_registry == version,
        request.destination != current.authority.binding.coordinator,
        request.destination != Principal::anonymous(),
        request.destination != Principal::management_canister(),
        current.registry_activation_receipt.is_some(),
        roots_are_deleted,
        assets_have_exact_owner,
    ]
    .into_iter()
    .all(|valid| valid);
    if !authority_is_exact {
        return Err(InternalError::conflict());
    }
    Ok(())
}
