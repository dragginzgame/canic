//! Module: infra::ic::ledger
//!
//! Responsibility: execute raw ICRC ledger calls and decode responses.
//! Does not own: allowance policy, expiry validation, business rules, or metrics.
//! Boundary: ops calls this after workflow/policy approve ledger interactions.

use crate::{
    cdk::{
        spec::standards::icrc::icrc2::{
            Allowance, AllowanceArgs, TransferFromArgs, TransferFromError, TransferFromResult,
        },
        types::{Account, Principal},
    },
    infra::{
        InfraError,
        ic::{
            IcInfraError,
            call::Call,
            known::{CKUSDC_LEDGER_CANISTER, CKUSDT_LEDGER_CANISTER},
        },
    },
};
use thiserror::Error as ThisError;

///
/// LedgerInfraError
///
/// Mechanical failures returned by ICRC ledger calls.
///

#[derive(Debug, ThisError)]
pub enum LedgerInfraError {
    #[error("{symbol} icrc2_transfer_from rejected: {error:?}")]
    TransferFromRejected {
        symbol: &'static str,
        error: TransferFromError,
    },
}

impl From<LedgerInfraError> for InfraError {
    fn from(err: LedgerInfraError) -> Self {
        IcInfraError::LedgerInfra(err).into()
    }
}

///
/// LedgerMeta
///
/// Best-effort static metadata for known ledgers.
/// Owned by ledger infra and used for diagnostics around rejected calls.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LedgerMeta {
    pub symbol: &'static str,
    pub decimals: u8,
    pub is_known: bool,
}

///
/// LedgerInfra
///
/// Raw ICRC ledger adapter.
/// Owned by IC infra and consumed by ops ledger flows.
///

pub struct LedgerInfra;

impl LedgerInfra {
    /// Return best-effort metadata for a ledger canister.
    #[must_use]
    pub fn ledger_meta(ledger_id: Principal) -> LedgerMeta {
        if ledger_id == *CKUSDC_LEDGER_CANISTER {
            return LedgerMeta {
                symbol: "ckUSDC",
                decimals: 6,
                is_known: true,
            };
        }

        if ledger_id == *CKUSDT_LEDGER_CANISTER {
            return LedgerMeta {
                symbol: "ckUSDT",
                decimals: 6,
                is_known: true,
            };
        }

        LedgerMeta {
            symbol: "UNKNOWN",
            decimals: 6,
            is_known: false,
        }
    }

    /// Call `icrc2_allowance` on the given ledger and return the raw allowance.
    pub async fn icrc2_allowance(
        ledger_id: Principal,
        account: Account,
        spender: Account,
    ) -> Result<Allowance, InfraError> {
        let args = AllowanceArgs { account, spender };

        let allowance: Allowance = Call::unbounded_wait(ledger_id, "icrc2_allowance")
            .with_arg(args)?
            .execute()
            .await?
            .candid()?;

        Ok(allowance)
    }

    /// Execute `icrc2_transfer_from` and return the accepted block index.
    pub async fn icrc2_transfer_from(
        ledger_id: Principal,
        args: TransferFromArgs,
    ) -> Result<u64, InfraError> {
        let result: TransferFromResult = Call::unbounded_wait(ledger_id, "icrc2_transfer_from")
            .with_arg(args)?
            .execute()
            .await?
            .candid()?;

        match result {
            TransferFromResult::Ok(block_index) => Ok(block_index),
            TransferFromResult::Err(err) => Err(LedgerInfraError::TransferFromRejected {
                symbol: Self::ledger_meta(ledger_id).symbol,
                error: err,
            }
            .into()),
        }
    }
}
