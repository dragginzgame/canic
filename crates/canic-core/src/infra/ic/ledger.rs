//! ICRC ledger helpers (IC edge).
//!
//! This module groups ICRC-2 allowance and transfer-from calls behind a
//! consistent Canic API surface.

use crate::{
    ThisError,
    cdk::{
        api,
        env::ck::{CKUSDC_LEDGER_CANISTER, CKUSDT_LEDGER_CANISTER},
        spec::icrc::icrc2::{
            Allowance, AllowanceArgs, TransferFromArgs, TransferFromError, TransferFromResult,
        },
        types::{Account, Principal},
    },
    infra::InfraError,
    infra::ic::IcInfraError,
    infra::ic::call::Call,
};

///
/// LedgerInfraError
///

#[derive(Debug, ThisError)]
pub enum LedgerInfraError {
    #[error("insufficient {symbol} allowance: has {allowance}, needs {required}")]
    InsufficientAllowance {
        symbol: &'static str,
        allowance: u64,
        required: u64,
    },

    #[error("{symbol} allowance expired at {expires_at_nanos} (now {now_nanos})")]
    AllowanceExpired {
        symbol: &'static str,
        expires_at_nanos: u64,
        now_nanos: u64,
    },

    #[error("{symbol} icrc2_transfer_from failed: {error:?}")]
    TransferFromRejected {
        symbol: &'static str,
        error: TransferFromError,
    },
}

impl From<LedgerInfraError> for InfraError {
    fn from(err: LedgerInfraError) -> Self {
        IcInfraError::from(err).into()
    }
}

///
/// LedgerMeta
/// Minimal metadata for known ledgers used in error messages and unit conversion.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LedgerMeta {
    pub symbol: &'static str,
    pub decimals: u8,
    pub is_known: bool,
}

///
/// ledger_meta
/// Returns best-effort metadata for a ledger.
///

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

///
/// icrc2_allowance
/// Calls `icrc2_allowance` on a ledger.
///

pub async fn icrc2_allowance(
    ledger_id: Principal,
    account: Account,
    spender: Account,
) -> Result<Allowance, InfraError> {
    let args = AllowanceArgs { account, spender };

    let allowance: Allowance = Call::unbounded_wait(ledger_id, "icrc2_allowance")
        .with_arg(args)
        .await?
        .candid()?;

    Ok(allowance)
}

///
/// validate_allowance
/// Ensures a payer has approved at least `required_amount` for the spender.
///

pub async fn validate_allowance(
    ledger_id: Principal,
    payer: Principal,
    spender: Account,
    required_amount: u64,
) -> Result<(), InfraError> {
    let meta = ledger_meta(ledger_id);

    let payer_account = Account {
        owner: payer,
        subaccount: None,
    };

    let allowance = icrc2_allowance(ledger_id, payer_account, spender).await?;

    if allowance.allowance < required_amount {
        return Err(LedgerInfraError::InsufficientAllowance {
            symbol: meta.symbol,
            allowance: allowance.allowance,
            required: required_amount,
        }
        .into());
    }

    if let Some(expires_at_nanos) = allowance.expires_at {
        let now_nanos = api::time();
        if expires_at_nanos <= now_nanos {
            return Err(LedgerInfraError::AllowanceExpired {
                symbol: meta.symbol,
                expires_at_nanos,
                now_nanos,
            }
            .into());
        }
    }

    Ok(())
}

///
/// icrc2_transfer_from
/// Executes an ICRC-2 `transfer_from` and returns the block index on success.
///

pub async fn icrc2_transfer_from(
    ledger_id: Principal,
    from: Principal,
    to: Account,
    amount: u64,
    memo: Option<Vec<u8>>,
) -> Result<u64, InfraError> {
    let meta = ledger_meta(ledger_id);

    let from_account = Account {
        owner: from,
        subaccount: None,
    };

    let args = TransferFromArgs {
        from: from_account,
        to,
        amount,
        memo,
        created_at_time: Some(api::time()),
    };

    let result: TransferFromResult = Call::unbounded_wait(ledger_id, "icrc2_transfer_from")
        .with_arg(args)
        .await?
        .candid()?;

    match result {
        TransferFromResult::Ok(block_index) => Ok(block_index),
        TransferFromResult::Err(err) => Err(LedgerInfraError::TransferFromRejected {
            symbol: meta.symbol,
            error: err,
        }
        .into()),
    }
}
