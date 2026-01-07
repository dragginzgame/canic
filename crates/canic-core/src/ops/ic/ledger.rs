//! ICRC ledger helpers (ops / approved IC surface).
//!
//! This module provides the approved, metrics-ready façade for interacting with
//! ICRC ledgers. It deliberately contains **no policy** (e.g. allowance
//! sufficiency checks). Policy belongs in access/rules or workflow.

use crate::{
    Error, ThisError,
    cdk::spec::icrc::icrc2::{Allowance, TransferFromArgs, TransferFromResult},
    infra::{InfraError, ic::ledger::LedgerInfra},
    ops::{ic::IcOpsError, prelude::*},
};

///
/// LedgerOpsError
///

#[derive(Debug, ThisError)]
pub enum LedgerOpsError {
    /// Any infra failure (IC call failed, candid errors, ledger rejection mapped in infra, etc.)
    #[error(transparent)]
    Infra(#[from] InfraError),
}

impl From<LedgerOpsError> for Error {
    fn from(err: LedgerOpsError) -> Self {
        IcOpsError::from(err).into()
    }
}

///
/// LedgerOps
///

pub struct LedgerOps;

impl LedgerOps {
    /// Query ICRC-2 allowance (raw ledger response).
    pub async fn allowance(
        ledger_id: Principal,
        payer: Account,
        spender: Account,
    ) -> Result<Allowance, Error> {
        let allowance = LedgerInfra::icrc2_allowance(ledger_id, payer, spender)
            .await
            .map_err(LedgerOpsError::from)?;

        Ok(allowance)
    }

    /// Execute an ICRC-2 transfer_from and return the block index on success.
    pub async fn transfer_from(
        ledger_id: Principal,
        from: Principal,
        to: Account,
        amount: u64,
        memo: Option<Vec<u8>>,
    ) -> Result<u64, Error> {
        // Note: created_at_time is set at the call site here because ops owns
        // execution conventions; infra owns mechanics.
        let from_account = Account {
            owner: from,
            subaccount: None,
        };

        let args = TransferFromArgs {
            from: from_account,
            to,
            amount,
            memo,
            created_at_time: Some(crate::cdk::api::time()),
        };

        let result: TransferFromResult = LedgerInfra::icrc2_transfer_from(ledger_id, args)
            .await
            .map_err(LedgerOpsError::from)?;

        match result {
            TransferFromResult::Ok(block_index) => Ok(block_index),

            // By construction, infra::ic::ledger::icrc2_transfer_from already maps Err(...)
            // into InfraError (lossless), so this branch should be unreachable. Keep it anyway
            // to be robust to future infra changes.
            TransferFromResult::Err(_) => {
                unreachable!()
                /*
                Err(LedgerOpsError::Infra(InfraError::from(
                LedgerInfraError::TransferFromRejected {
                    symbol: LedgerInfra::ledger_meta(ledger_id).symbol,
                    // We can't recover the error here without matching; infra should not return Err(...)
                    // if it wants ops to handle it. Prefer keeping infra mapping.
                    error: unreachable!(
                        "infra::ic::ledger maps TransferFromResult::Err into InfraError"
                    ),
                },*/
            }
        }
    }
}
