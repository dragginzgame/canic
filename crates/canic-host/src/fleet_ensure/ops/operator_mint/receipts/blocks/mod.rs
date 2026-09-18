//! Module: fleet_ensure::ops::operator_mint::receipts::blocks
//!
//! Responsibility: bind bounded ICRC3 blocks to a certified tip and exact mint intent.
//! Boundary: upstream ICRC3 hashing; no certificate verification or local accounting.

use crate::fleet_ensure::{
    model::operator_mint::OperatorMintIntentRecord,
    ops::operator_mint::receipts::{
        CyclesLedgerBlock, DepositVerificationError as Error, ReceiptVerificationLimits,
        certificate::CertifiedTip,
    },
};
use icrc_ledger_types::icrc::generic_value::Value;
use std::collections::BTreeMap;

pub(super) fn check_budget(
    chain: &[CyclesLedgerBlock],
    limits: ReceiptVerificationLimits,
) -> Result<(), Error> {
    if chain.len() > limits.blocks {
        return Err(Error::BudgetExceeded);
    }
    let mut nodes = limits.value_nodes;
    let mut bytes = limits.value_bytes;
    for block in chain {
        if block.id.0.bits() > 128 {
            return Err(Error::ChainMismatch);
        }
        check_value(&block.block, limits.depth, &mut nodes, &mut bytes)?;
    }
    Ok(())
}

fn check_value(
    value: &Value,
    depth: u8,
    nodes: &mut usize,
    bytes: &mut usize,
) -> Result<(), Error> {
    *nodes = nodes.checked_sub(1).ok_or(Error::BudgetExceeded)?;
    let depth = depth.checked_sub(1).ok_or(Error::BudgetExceeded)?;
    let scalar_bytes = match value {
        Value::Blob(value) => value.len(),
        Value::Text(value) => value.len(),
        Value::Nat64(_) => size_of::<u64>(),
        Value::Nat(value) => {
            usize::try_from(value.0.bits().div_ceil(8)).map_err(|_| Error::BudgetExceeded)?
        }
        Value::Int(value) => {
            // The upstream hasher panics for Int values outside signed 128 bits.
            i128::try_from(&value.0).map_err(|_| Error::UnsupportedBlockValue)?;
            usize::try_from(value.0.bits().div_ceil(8)).map_err(|_| Error::BudgetExceeded)?
        }
        Value::Array(values) => {
            for value in values {
                check_value(value, depth, nodes, bytes)?;
            }
            0
        }
        Value::Map(values) => {
            for (key, value) in values {
                *bytes = bytes.checked_sub(key.len()).ok_or(Error::BudgetExceeded)?;
                check_value(value, depth, nodes, bytes)?;
            }
            0
        }
    };
    *bytes = bytes
        .checked_sub(scalar_bytes)
        .ok_or(Error::BudgetExceeded)?;
    Ok(())
}

pub(super) fn verify_chain<'a>(
    chain: &'a [CyclesLedgerBlock],
    tip: &CertifiedTip,
    deposit_index: u128,
) -> Result<&'a Value, Error> {
    let expected_len = tip
        .index
        .checked_sub(deposit_index)
        .and_then(|n| n.checked_add(1));
    if chain.is_empty() || expected_len != Some(chain.len() as u128) {
        return Err(Error::ChainMismatch);
    }
    let mut expected_hash = tip.hash;
    for (offset, block) in chain.iter().enumerate() {
        let expected_index = tip.index - offset as u128;
        if block.id != expected_index || block.block.hash() != expected_hash {
            return Err(Error::ChainMismatch);
        }
        let fields = map(&block.block).map_err(|_| Error::ChainMismatch)?;
        if expected_index == 0 {
            if fields.contains_key("phash") {
                return Err(Error::ChainMismatch);
            }
        } else {
            expected_hash = blob(fields.get("phash"))
                .and_then(|bytes| bytes.try_into().ok())
                .ok_or(Error::ChainMismatch)?;
        }
    }
    chain
        .last()
        .map(|block| &block.block)
        .ok_or(Error::ChainMismatch)
}

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
