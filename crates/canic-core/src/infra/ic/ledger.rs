//! ICRC ledger helpers (infra / IC edge).
//!
//! This module provides **raw, mechanical access** to ICRC-2 ledger calls.
//! It performs no policy checks, no validation, and no orchestration.
//!
//! Responsibilities:
//! - Construct IC arguments
//! - Execute ledger calls
//! - Decode responses
//! - Surface lossless, mechanical failures
//!
//! Non-responsibilities:
//! - Allowance sufficiency checks
//! - Expiry validation
//! - Business or access rules
//! - Metrics or logging

#![allow(dead_code)]

use crate::{
    cdk::{
        env::ck::{CKUSDC_LEDGER_CANISTER, CKUSDT_LEDGER_CANISTER},
        spec::icrc::icrc2::{
            Allowance, AllowanceArgs, TransferFromArgs, TransferFromError, TransferFromResult,
        },
    },
    infra::{
        ic::{IcInfraError, call::Call},
        prelude::*,
    },
};

///
/// LedgerInfraError
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
/// Best-effort static metadata for known ledgers.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LedgerMeta {
    pub symbol: &'static str,
    pub decimals: u8,
    pub is_known: bool,
}

/// ledger_meta
/// Returns best-effort metadata for a ledger canister.
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

/// icrc2_allowance
/// Calls `icrc2_allowance` on the given ledger and returns the raw allowance.
pub async fn icrc2_allowance(
    ledger_id: Principal,
    account: Account,
    spender: Account,
) -> Result<Allowance, InfraError> {
    let args = AllowanceArgs { account, spender };

    let allowance: Allowance = Call::unbounded_wait(ledger_id, "icrc2_allowance")
        .try_with_arg(args)?
        .execute()
        .await?
        .candid()?;

    Ok(allowance)
}

/// icrc2_transfer_from
/// Executes `icrc2_transfer_from` and returns the raw result.
pub async fn icrc2_transfer_from(
    ledger_id: Principal,
    args: TransferFromArgs,
) -> Result<TransferFromResult, InfraError> {
    let result: TransferFromResult = Call::unbounded_wait(ledger_id, "icrc2_transfer_from")
        .try_with_arg(args)?
        .execute()
        .await?
        .candid()?;

    match result {
        TransferFromResult::Ok(_) => Ok(result),
        TransferFromResult::Err(err) => Err(LedgerInfraError::TransferFromRejected {
            symbol: ledger_meta(ledger_id).symbol,
            error: err,
        }
        .into()),
    }
}
