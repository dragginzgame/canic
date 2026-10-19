//! Module: fleet_ensure::ops::operator_mint::receipts::blocks
//!
//! Responsibility: bind bounded ICRC3 blocks to a certified tip and exact mint intent.
//! Boundary: upstream ICRC3 hashing; no certificate verification or local accounting.

use crate::fleet_ensure::{
    model::operator_mint::OperatorMintIntentRecord,
    ops::operator_mint::receipts::DepositVerificationError as Error,
};
use ic_platform::icrc_ledger_types::Value;
use std::collections::BTreeMap;

pub(super) fn deposit_amounts(
    block: &Value,
    intent: &OperatorMintIntentRecord,
    gross: u128,
) -> Result<(u128, u128), Error> {
    let fields = map(block)?;
    if fields
        .keys()
        .any(|key| !matches!(key.as_str(), "tx" | "ts" | "fee" | "phash"))
        || nat(fields.get("ts"))? > u128::from(u64::MAX)
        || nat(fields.get("fee"))? != 0
    {
        return Err(Error::UnsupportedDeposit);
    }
    let tx = map(fields.get("tx").ok_or(Error::UnsupportedDeposit)?)?;
    if tx.len() != 5
        || tx
            .keys()
            .any(|key| !matches!(key.as_str(), "op" | "to" | "memo" | "amt" | "fee"))
        || !matches!(tx.get("op"), Some(Value::Text(op)) if op == "mint")
    {
        return Err(Error::UnsupportedDeposit);
    }
    let Some(Value::Array(account)) = tx.get("to") else {
        return Err(Error::UnsupportedDeposit);
    };
    if account.len() != 1
        || blob(account.first()) != Some(intent.authority.operator.as_slice())
        || blob(tx.get("memo")) != Some(intent.deposit_memo.as_slice())
    {
        return Err(Error::DepositBindingMismatch);
    }
    let net = nat(tx.get("amt"))?;
    let fee = nat(tx.get("fee"))?;
    if net == 0 || net.checked_add(fee) != Some(gross) {
        return Err(Error::AmountMismatch);
    }
    Ok((net, fee))
}

const fn map(value: &Value) -> Result<&BTreeMap<String, Value>, Error> {
    match value {
        Value::Map(fields) => Ok(fields),
        _ => Err(Error::UnsupportedDeposit),
    }
}

fn nat(value: Option<&Value>) -> Result<u128, Error> {
    match value {
        Some(Value::Nat(value)) => (&value.0).try_into().map_err(|_| Error::AmountMismatch),
        _ => Err(Error::UnsupportedDeposit),
    }
}

fn blob(value: Option<&Value>) -> Option<&[u8]> {
    match value {
        Some(Value::Blob(value)) => Some(value.as_ref()),
        _ => None,
    }
}

pub(super) const fn evidence_error(error: ic_query::ledger_evidence::BlockEvidenceError) -> Error {
    match error {
        ic_query::ledger_evidence::BlockEvidenceError::BudgetExceeded => Error::BudgetExceeded,
        ic_query::ledger_evidence::BlockEvidenceError::ChainMismatch => Error::ChainMismatch,
        ic_query::ledger_evidence::BlockEvidenceError::UnsupportedValue => {
            Error::UnsupportedBlockValue
        }
    }
}
