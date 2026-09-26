//! Read-only default Cycles Ledger accounting for a completed estate survey.
//!
//! Preserve original operator and Root baselines. Other canister accounts remain
//! separately observed funds; no payment, native inspection or reset is authorized.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    CompletedEstateInventoryView, CompletedLedgerAccountView, CompletedLedgerBalancesView,
    model::{DesiredCanisterKind, MAX_FLEET_ENSURE_CANISTERS},
};
use candid::{Nat, Principal};
use canic_core::ids::CanonicalNetworkId;
use ic_agent::Agent;
use icrc_ledger_types::icrc1::account::Account;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::{Duration, Instant},
};
use thiserror::Error;

const PASS_LIFETIME: Duration = Duration::from_secs(60);
const QUERY_TIMEOUT: Duration = Duration::from_secs(10);
const REPLY_BYTES: usize = 1024;

/// A Ledger sample cannot account for the exact source accounts and receipts.
#[derive(Debug, Error)]
pub enum CompletedLedgerError {
    #[error("Ledger observation signer/network differs from completed source evidence")]
    Authority,
    #[error("completed source Ledger account ownership is missing, duplicated or invalid")]
    AccountSet,
    #[error("completed estate Ledger observations expired")]
    Expired,
    #[error("default account query for {owner} on Ledger {ledger} failed: {source}")]
    Query {
        ledger: Principal,
        owner: Principal,
        #[source]
        source: Box<ic_agent::AgentError>,
    },
    #[error("default account response for {owner} is malformed or exceeds the observation bound")]
    Response { owner: Principal },
    #[error(
        "operator Ledger balance changed outside original receipts: expected {expected}, observed {observed}; preserve the original evidence"
    )]
    OperatorMovement { expected: u128, observed: u128 },
    #[error(
        "Root {root} Ledger balance changed outside original receipts: expected {expected}, observed {observed}; preserve the original evidence"
    )]
    RootMovement {
        root: String,
        expected: u128,
        observed: u128,
    },
    #[error("completed estate Ledger balance arithmetic exceeds u128")]
    Overflow,
}

struct Accounts {
    ledger: Principal,
    operator: Principal,
    operator_cycles: u128,
    roots: BTreeMap<String, CompletedLedgerAccountView>,
    other: BTreeMap<String, Principal>,
}

pub(super) async fn observe(
    agent: &Agent,
    source: &CompletedEstateInventoryView,
    started: Instant,
) -> Result<CompletedLedgerBalancesView, CompletedLedgerError> {
    let accounts = accounts(source)?;
    let network = CanonicalNetworkId::from_der_root_trust_anchor(&agent.read_root_key())
        .map_err(|_| CompletedLedgerError::Authority)?;
    if agent.get_principal().ok() != Some(accounts.operator)
        || network != source.fleet.fleet.canonical_network_id
    {
        return Err(CompletedLedgerError::Authority);
    }
    let mut balances = BTreeMap::new();
    for owner in std::iter::once(accounts.operator)
        .chain(accounts.roots.values().map(|entry| entry.owner))
        .chain(accounts.other.values().copied())
    {
        let remaining = PASS_LIFETIME
            .checked_sub(started.elapsed())
            .filter(|left| !left.is_zero())
            .ok_or(CompletedLedgerError::Expired)?;
        let cycles = tokio::time::timeout(
            remaining.min(QUERY_TIMEOUT),
            balance(agent, accounts.ledger, owner),
        )
        .await
        .map_err(|_| CompletedLedgerError::Expired)??;
        balances.insert(owner, cycles);
    }
    if started.elapsed() > PASS_LIFETIME {
        return Err(CompletedLedgerError::Expired);
    }
    reconcile(accounts, balances)
}

fn accounts(source: &CompletedEstateInventoryView) -> Result<Accounts, CompletedLedgerError> {
    let operator = principal(&source.receipts.source_operator)?;
    let ledger = principal(&source.receipts.cycles_ledger)?;
    let operator_cycles = source
        .receipts
        .initial_operator_cycles
        .checked_sub(source.receipts.recorded_operator_debit_cycles)
        .ok_or(CompletedLedgerError::Overflow)?;
    let mut owners = BTreeSet::from([operator, ledger]);
    if owners.len() != 2
        || source.canisters.is_empty()
        || source.canisters.len() > MAX_FLEET_ENSURE_CANISTERS
    {
        return Err(CompletedLedgerError::AccountSet);
    }
    let mut roots = BTreeMap::new();
    let mut other = BTreeMap::new();
    for (name, entry) in &source.canisters {
        if !owners.insert(entry.principal) {
            return Err(CompletedLedgerError::AccountSet);
        }
        if entry.kind == DesiredCanisterKind::Root {
            let cycles = *source
                .receipts
                .initial_estate_funding_cycles_by_root
                .get(name)
                .ok_or(CompletedLedgerError::AccountSet)?;
            roots.insert(
                name.clone(),
                CompletedLedgerAccountView {
                    owner: entry.principal,
                    cycles,
                },
            );
        } else {
            other.insert(name.clone(), entry.principal);
        }
    }
    if roots.is_empty()
        || roots.len() != source.receipts.initial_estate_funding_cycles_by_root.len()
    {
        return Err(CompletedLedgerError::AccountSet);
    }
    Ok(Accounts {
        ledger,
        operator,
        operator_cycles,
        roots,
        other,
    })
}

fn principal(text: &str) -> Result<Principal, CompletedLedgerError> {
    let id = Principal::from_text(text).map_err(|_| CompletedLedgerError::AccountSet)?;
    if id.to_text() != text
        || id == Principal::anonymous()
        || id == Principal::management_canister()
    {
        return Err(CompletedLedgerError::AccountSet);
    }
    Ok(id)
}

async fn balance(
    agent: &Agent,
    ledger: Principal,
    owner: Principal,
) -> Result<u128, CompletedLedgerError> {
    let argument = candid::encode_one(Account {
        owner,
        subaccount: None,
    })
    .map_err(|_| CompletedLedgerError::Response { owner })?;
    let bytes = agent
        .query(&ledger, "icrc1_balance_of")
        .with_arg(argument)
        .call()
        .await
        .map_err(|source| CompletedLedgerError::Query {
            ledger,
            owner,
            source: Box::new(source),
        })?;
    decode(owner, &bytes)
}

fn decode(owner: Principal, bytes: &[u8]) -> Result<u128, CompletedLedgerError> {
    if bytes.len() > REPLY_BYTES {
        return Err(CompletedLedgerError::Response { owner });
    }
    let mut config = candid::de::DecoderConfig::new();
    config.set_decoding_quota(REPLY_BYTES * 64);
    config.set_skipping_quota(REPLY_BYTES);
    let value: Nat = candid::utils::decode_one_with_config(bytes, &config)
        .map_err(|_| CompletedLedgerError::Response { owner })?;
    u128::try_from(value.0).map_err(|_| CompletedLedgerError::Overflow)
}

fn reconcile(
    accounts: Accounts,
    mut balances: BTreeMap<Principal, u128>,
) -> Result<CompletedLedgerBalancesView, CompletedLedgerError> {
    let operator = balances
        .remove(&accounts.operator)
        .ok_or(CompletedLedgerError::AccountSet)?;
    if operator != accounts.operator_cycles {
        return Err(CompletedLedgerError::OperatorMovement {
            expected: accounts.operator_cycles,
            observed: operator,
        });
    }
    let mut total = 0_u128;
    for (name, expected) in &accounts.roots {
        let cycles = balances
            .remove(&expected.owner)
            .ok_or(CompletedLedgerError::AccountSet)?;
        if cycles != expected.cycles {
            return Err(CompletedLedgerError::RootMovement {
                root: name.clone(),
                expected: expected.cycles,
                observed: cycles,
            });
        }
        total = total
            .checked_add(cycles)
            .ok_or(CompletedLedgerError::Overflow)?;
    }
    let mut other_canister_accounts = BTreeMap::new();
    for (name, owner) in accounts.other {
        let cycles = balances
            .remove(&owner)
            .ok_or(CompletedLedgerError::AccountSet)?;
        total = total
            .checked_add(cycles)
            .ok_or(CompletedLedgerError::Overflow)?;
        other_canister_accounts.insert(name, CompletedLedgerAccountView { owner, cycles });
    }
    if !balances.is_empty() {
        return Err(CompletedLedgerError::AccountSet);
    }
    Ok(CompletedLedgerBalancesView {
        ledger: accounts.ledger,
        operator: CompletedLedgerAccountView {
            owner: accounts.operator,
            cycles: operator,
        },
        root_accounts: accounts.roots,
        other_canister_accounts,
        total_canister_ledger_cycles: total,
    })
}
