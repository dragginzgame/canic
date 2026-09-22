//! Authenticate one external operator debit for a separately reviewed retirement.
//!
//! Reads use replicated queries and controller-only management status, never payments.
//! A burn receipt proves a debit, not withdrawal delivery or authority to retry it.

#[cfg(test)]
mod tests;

use crate::{
    fleet_ensure::{
        model::RetirementWithdrawalRecord,
        ops::operator_mint::receipts::{CyclesLedgerBlock, network_identity_sha256},
    },
    icp::{IcpCli, IcpManagementCallError},
};
use candid::{CandidType, Nat, Principal};
use canic_core::cdk::utils::hash::hex_bytes;
use ic_agent::{Agent, AgentError};
use icrc_ledger_types::{icrc::generic_value::Value, icrc3::blocks::GetBlocksRequest};
use serde::{Deserialize, de::DeserializeOwned};
use std::{collections::BTreeMap, time::Duration};
use thiserror::Error;

const RESPONSE_BYTES: usize = 64 * 1024;
const TIMEOUT: Duration = Duration::from_secs(90);

#[derive(Debug, Error)]
pub enum RetirementDebitError {
    #[error(
        "retirement debit does not match the selected Ledger, operator or controlled destination"
    )]
    Binding,
    #[error("retirement debit block is absent or has an unsupported withdrawal shape")]
    Block,
    #[error("retirement debit receipt exceeds its observation budget")]
    Budget,
    #[error(transparent)]
    Agent(#[from] AgentError),
    #[error(transparent)]
    Candid(#[from] candid::Error),
    #[error(transparent)]
    Identity(Box<IcpManagementCallError>),
    #[error(transparent)]
    Runtime(#[from] std::io::Error),
    #[error("retirement debit observation timed out; no payment was attempted")]
    Timeout,
}

impl From<IcpManagementCallError> for RetirementDebitError {
    fn from(error: IcpManagementCallError) -> Self {
        Self::Identity(Box::new(error))
    }
}

#[derive(CandidType, Deserialize)]
struct BlocksReply {
    log_length: Nat,
    blocks: Vec<CyclesLedgerBlock>,
}

#[derive(CandidType)]
struct StatusArgs {
    canister_id: Principal,
}

#[derive(CandidType, Deserialize)]
struct Status {
    settings: Settings,
}

#[derive(CandidType, Deserialize)]
struct Settings {
    controllers: Vec<Principal>,
}

/// Read and bind exactly one debit; callers still own policy and digest approval.
pub(in crate::fleet_ensure) fn observe(
    icp: &IcpCli,
    ledger: &str,
    operator: &str,
    block: u64,
) -> Result<RetirementWithdrawalRecord, RetirementDebitError> {
    let ledger = Principal::from_text(ledger).map_err(|_| RetirementDebitError::Binding)?;
    let operator = Principal::from_text(operator).map_err(|_| RetirementDebitError::Binding)?;
    let agent = icp.authenticated_agent_with_response_limit(RESPONSE_BYTES)?;
    if agent
        .get_principal()
        .map_err(|_| RetirementDebitError::Binding)?
        != operator
    {
        return Err(RetirementDebitError::Binding);
    }
    tokio::runtime::Runtime::new()?.block_on(async {
        let argument = candid::encode_one(vec![GetBlocksRequest {
            start: Nat::from(block),
            length: Nat::from(1_u8),
        }])?;
        let reply: BlocksReply =
            replicated_read(&agent, ledger, ledger, "icrc3_get_blocks", argument).await?;
        let network = hex_bytes(network_identity_sha256(&agent.read_root_key()));
        let receipt = decode_block(reply, ledger, operator, block, network)?;
        let destination = Principal::from_text(&receipt.destination)
            .map_err(|_| RetirementDebitError::Binding)?;
        let status: Status = replicated_read(
            &agent,
            Principal::management_canister(),
            destination,
            "canister_status",
            candid::encode_one(StatusArgs {
                canister_id: destination,
            })?,
        )
        .await?;
        if !status.settings.controllers.contains(&operator) {
            return Err(RetirementDebitError::Binding);
        }
        Ok(receipt)
    })
}

async fn replicated_read<T: CandidType + DeserializeOwned>(
    agent: &Agent,
    canister: Principal,
    effective: Principal,
    method: &str,
    argument: Vec<u8>,
) -> Result<T, RetirementDebitError> {
    let bytes = tokio::time::timeout(
        TIMEOUT,
        agent
            .update(&canister, method)
            .with_effective_canister_id(effective)
            .with_arg(argument)
            .call_and_wait(),
    )
    .await
    .map_err(|_| RetirementDebitError::Timeout)??;
    if bytes.len() > RESPONSE_BYTES {
        return Err(RetirementDebitError::Budget);
    }
    let mut limits = candid::de::DecoderConfig::new();
    limits
        .set_decoding_quota(100_000)
        .set_skipping_quota(100_000)
        .set_max_type_len(1_000);
    Ok(candid::utils::decode_one_with_config(&bytes, &limits)?)
}

fn decode_block(
    reply: BlocksReply,
    ledger: Principal,
    operator: Principal,
    index: u64,
    network: String,
) -> Result<RetirementWithdrawalRecord, RetirementDebitError> {
    if reply.log_length <= index || reply.blocks.len() != 1 {
        return Err(RetirementDebitError::Block);
    }
    let block = &reply.blocks[0];
    if block.id != index {
        return Err(RetirementDebitError::Block);
    }
    let fields = map(&block.block)?;
    if fields
        .keys()
        .any(|key| !matches!(key.as_str(), "tx" | "ts" | "fee" | "phash"))
    {
        return Err(RetirementDebitError::Block);
    }
    match fields.get("phash") {
        Some(Value::Blob(hash)) if hash.len() == 32 && index > 0 => {}
        None if index == 0 => {}
        _ => return Err(RetirementDebitError::Block),
    }
    let timestamp_ns =
        u64::try_from(nat(fields.get("ts"))?).map_err(|_| RetirementDebitError::Block)?;
    let fee_cycles = nat(fields.get("fee"))?;
    let tx = map(fields.get("tx").ok_or(RetirementDebitError::Block)?)?;
    if tx
        .keys()
        .any(|key| !matches!(key.as_str(), "op" | "from" | "amt" | "memo" | "ts"))
        || tx.get("op") != Some(&Value::Text("burn".into()))
    {
        return Err(RetirementDebitError::Block);
    }
    if let Some(created) = tx.get("ts")
        && nat(Some(created))? > u128::from(u64::MAX)
    {
        return Err(RetirementDebitError::Block);
    }
    let Some(Value::Array(account)) = tx.get("from") else {
        return Err(RetirementDebitError::Block);
    };
    if account.len() != 1 || blob(account.first())? != operator.as_slice() {
        return Err(RetirementDebitError::Binding);
    }
    let amount_cycles = nat(tx.get("amt"))?;
    if amount_cycles == 0 || fee_cycles == 0 || amount_cycles.checked_add(fee_cycles).is_none() {
        return Err(RetirementDebitError::Block);
    }
    let memo = blob(tx.get("memo"))?;
    if memo.len() > 32 {
        return Err(RetirementDebitError::Budget);
    }
    let mut cursor = std::io::Cursor::new(memo);
    let parsed: ciborium::value::Value =
        ciborium::de::from_reader(&mut cursor).map_err(|_| RetirementDebitError::Block)?;
    if cursor.position() != memo.len() as u64 {
        return Err(RetirementDebitError::Block);
    }
    let ciborium::value::Value::Array(values) = parsed else {
        return Err(RetirementDebitError::Block);
    };
    let [ciborium::value::Value::Bytes(destination)] = values.as_slice() else {
        return Err(RetirementDebitError::Block);
    };
    let destination =
        Principal::try_from_slice(destination).map_err(|_| RetirementDebitError::Block)?;
    Ok(RetirementWithdrawalRecord {
        ledger: ledger.to_text(),
        operator: operator.to_text(),
        destination: destination.to_text(),
        network_identity_sha256: network,
        block_sha256: hex_bytes(block.block.hash()),
        block_index: index,
        timestamp_ns,
        amount_cycles,
        fee_cycles,
    })
}

const fn map(value: &Value) -> Result<&BTreeMap<String, Value>, RetirementDebitError> {
    match value {
        Value::Map(fields) => Ok(fields),
        _ => Err(RetirementDebitError::Block),
    }
}

fn nat(value: Option<&Value>) -> Result<u128, RetirementDebitError> {
    match value {
        Some(Value::Nat(value)) => (&value.0)
            .try_into()
            .map_err(|_| RetirementDebitError::Block),
        Some(Value::Nat64(value)) => Ok(u128::from(*value)),
        _ => Err(RetirementDebitError::Block),
    }
}

fn blob(value: Option<&Value>) -> Result<&[u8], RetirementDebitError> {
    match value {
        Some(Value::Blob(value)) => Ok(value.as_ref()),
        _ => Err(RetirementDebitError::Block),
    }
}
