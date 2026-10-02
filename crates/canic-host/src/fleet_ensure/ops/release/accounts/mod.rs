//! Observe declared ICRC accounts without transferring balances or granting reset authority.
//!
//! Role-owned obligation discovery and recovery-artifact qualification remain separate owners.

#[cfg(test)]
mod tests;

use crate::{
    fleet_ensure::{
        model::release::{FleetReleaseAccountRecord, FleetReleaseReviewRecord},
        ops::release::observation::{ReleaseObservationError, verify_agent},
        policy::release::{FleetReleaseError, validate_account_inventory},
    },
    icp::IcpCli,
};
use std::time::Duration;

use candid::{Nat, Principal};
use ic_agent::Agent;
use icrc_ledger_types::icrc1::account::Account;
use thiserror::Error;

const RESPONSE_BYTES: usize = 4096;
const INVENTORY_DEADLINE: Duration = Duration::from_secs(60);
const QUERY_DEADLINE: Duration = Duration::from_secs(15);

/// Observation refusals return no partial account inventory.
#[derive(Debug, Error)]
pub enum ReleaseAccountError {
    #[error(transparent)]
    Authentication(#[from] ReleaseObservationError),

    #[error("Fleet release account observation exceeded its deadline")]
    Deadline,

    #[error(transparent)]
    Inventory(#[from] FleetReleaseError),

    #[error("Ledger {ledger} balance observation for {owner} failed at {stage:?}")]
    Observation {
        ledger: Principal,
        owner: Principal,
        stage: ReleaseAccountStage,
    },
}

/// Typed boundary that refused one declared account observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseAccountStage {
    Decode,
    Overflow,
    Query,
}

/// Read exact declared accounts, including zero balances, using the reviewed network and signer.
///
/// The balances are fresh observations, not copied review values. Recovery hashes remain
/// declarations requiring separate artifact qualification. This does not enumerate arbitrary
/// application subaccounts, settle paid effects, or freeze balances across queries.
pub async fn collect(
    icp: &IcpCli,
    review: &FleetReleaseReviewRecord,
) -> Result<Vec<FleetReleaseAccountRecord>, ReleaseAccountError> {
    validate_account_inventory(review)?;
    let agent = icp
        .authenticated_agent_with_response_limit(RESPONSE_BYTES)
        .map_err(|error| ReleaseObservationError::Authentication(Box::new(error)))?;
    collect_with_agent(&agent, review).await
}

async fn collect_with_agent(
    agent: &Agent,
    review: &FleetReleaseReviewRecord,
) -> Result<Vec<FleetReleaseAccountRecord>, ReleaseAccountError> {
    validate_account_inventory(review)?;
    verify_agent(agent, &review.authority)?;
    tokio::time::timeout(INVENTORY_DEADLINE, async {
        let mut observed = Vec::with_capacity(review.accounts.len());
        for expected in &review.accounts {
            let account = Account {
                owner: expected.owner,
                subaccount: expected.subaccount.filter(|value| *value != [0; 32]),
            };
            let fail = |stage| ReleaseAccountError::Observation {
                ledger: expected.ledger,
                owner: expected.owner,
                stage,
            };
            let argument =
                candid::encode_one(account).map_err(|_| fail(ReleaseAccountStage::Query))?;
            // Queries attach no cycles, issue no update, and have no automatic retry.
            let bytes = tokio::time::timeout(
                QUERY_DEADLINE,
                agent
                    .query(&expected.ledger, "icrc1_balance_of")
                    .with_arg(argument)
                    .call(),
            )
            .await
            .map_err(|_| fail(ReleaseAccountStage::Query))?
            .map_err(|_| fail(ReleaseAccountStage::Query))?;
            let balance = decode_balance(&bytes).map_err(fail)?;
            let mut current = expected.clone();
            current.subaccount = account.subaccount;
            current.observed_balance = balance;
            observed.push(current);
        }
        Ok(observed)
    })
    .await
    .map_err(|_| ReleaseAccountError::Deadline)?
}

fn decode_balance(bytes: &[u8]) -> Result<u128, ReleaseAccountStage> {
    if bytes.len() > RESPONSE_BYTES {
        return Err(ReleaseAccountStage::Decode);
    }
    let mut config = candid::de::DecoderConfig::new();
    config
        .set_decoding_quota(RESPONSE_BYTES * 8)
        .set_skipping_quota(RESPONSE_BYTES * 8)
        .set_max_type_len(16)
        .set_max_header_len(128);
    let value: Nat = candid::utils::decode_one_with_config(bytes, &config)
        .map_err(|_| ReleaseAccountStage::Decode)?;
    u128::try_from(value.0).map_err(|_| ReleaseAccountStage::Overflow)
}
