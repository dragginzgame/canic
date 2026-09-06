//! Module: workflow::root_ledger_retirement
//!
//! Responsibility: evacuate the Root Ledger account within its existing retirement operation.
//! Boundary: exact transfer intent precedes dispatch; deletion requires its receipt.

use crate::{
    ops::component_registry::ComponentRegistryOps,
    view::component_registry::RootFleetSubnetDeletionPreparationIntentView,
    workflow::fleet_subnet_root::{
        reserve_root_deletion_cycle_reclamation, settle_root_deletion_cycle_reclamation,
    },
};
use canic_core::control_plane_support::{
    error::InternalError,
    ops::ic::{
        IcOps,
        cycles_ledger::{CyclesLedgerOps, CyclesLedgerTransferError},
    },
};

pub(super) async fn evacuate(
    intent: RootFleetSubnetDeletionPreparationIntentView,
) -> Result<RootFleetSubnetDeletionPreparationIntentView, InternalError> {
    if intent.ledger_receipt.is_some() {
        require_empty_source().await?;
        return Ok(intent);
    }
    let transfer = if let Some(transfer) = intent.ledger_transfer {
        transfer
    } else {
        let balance = CyclesLedgerOps::balance_of(IcOps::canister_self())
            .await?
            .to_u128();
        let fee = if balance == 0 {
            0
        } else {
            CyclesLedgerOps::fee().await?.to_u128()
        };
        ComponentRegistryOps::begin_root_ledger_transfer(
            intent.operation_id,
            balance,
            fee,
            IcOps::now_nanos(),
        )?
    };
    let block_index = if transfer.balance_before == 0 {
        None
    } else {
        let permit = reserve_root_deletion_cycle_reclamation(
            intent.coordinator,
            0,
            intent.retained_cycles_target,
            IcOps::canister_cycle_balance().to_u128(),
        )?;
        let response = CyclesLedgerOps::transfer(&permit, &transfer).await;
        let response = match response {
            Ok(response) => {
                settle_root_deletion_cycle_reclamation(&permit, Ok(()))?;
                response
            }
            Err(error) => {
                settle_root_deletion_cycle_reclamation(&permit, Err(error))?;
                return Err(InternalError::invariant());
            }
        };
        let block = match response {
            Ok(block)
            | Err(CyclesLedgerTransferError::Duplicate {
                duplicate_of: block,
            }) => block,
            Err(CyclesLedgerTransferError::TemporarilyUnavailable) => {
                return Err(InternalError::unavailable());
            }
            Err(_) => return Err(InternalError::conflict()),
        };
        Some(u128::try_from(block.0).map_err(|_| InternalError::invariant())?)
    };
    require_empty_source().await?;
    ComponentRegistryOps::complete_root_ledger_transfer(intent.operation_id, block_index)
}

async fn require_empty_source() -> Result<(), InternalError> {
    if CyclesLedgerOps::balance_of(IcOps::canister_self())
        .await?
        .to_u128()
        != 0
    {
        return Err(InternalError::conflict());
    }
    Ok(())
}
