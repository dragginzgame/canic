//! Public mainnet default-account observations; no signer or transfer authority.
use crate::{Error, Network, Result, network, supplier};
use ic_platform::{
    agent::Agent,
    candid_codec::{self, decoder},
    icrc_ledger_types::Account,
};
use serde::Serialize;
use std::time::Duration;

#[cfg(test)]
mod tests;

const ICP_LEDGER: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";
const CYCLES_LEDGER: &str = "um5iw-rqaaa-aaaaq-qaaba-cai";
const LIMITS: decoder::Limits = decoder::Limits {
    bytes: 1024,
    header: 256,
    types: 16,
    work: 4096,
    allocation: 4096,
};

/// A mainnet wallet's default account, selected explicitly by its public principal.
#[derive(Debug, Serialize)]
pub struct Wallet {
    /// Canonical owner principal. No identity name or secret is accessed.
    pub principal: String,
    /// ICP legacy ledger receive address for the default (zero) subaccount.
    pub icp_address: String,
    /// Explicit network; this adapter never infers a local replica or alternative ledger.
    pub network: &'static str,
    /// These public query results are not certified balance proofs.
    pub certified: bool,
    /// ICP balance in the default account.
    pub icp: WalletBalance,
    /// Cycles-ledger account balance, excluding cycles already held by canisters.
    pub cycles: WalletBalance,
}

/// One independent balance observation. Failure is unknown, never a zero balance.
#[derive(Debug, Serialize)]
pub struct WalletBalance {
    /// Fixed mainnet ledger queried.
    pub ledger: &'static str,
    /// Exact decimal base units: e8s for ICP, cycles for the cycles ledger.
    pub amount: Option<String>,
    /// Exact human-readable amount, formatted without floating point.
    pub display: Option<String>,
    /// UTC Unix nanoseconds at observation completion.
    pub observed_at_ns: u64,
    /// Bounded provider/decoding diagnostic; absence does not certify the query.
    pub observation_error: Option<String>,
}

fn account(principal: &str) -> Result<Account> {
    let owner = network::principal(principal)?;
    if owner == ic_platform::Principal::anonymous()
        || owner == ic_platform::Principal::management_canister()
    {
        return Err(Error::Invalid(
            "wallet requires an owner other than anonymous or management".into(),
        ));
    }
    Ok(Account::from(owner))
}

/// Derive the default ICP receive address locally, without accessing an identity or network.
///
/// # Errors
/// Refuses malformed, anonymous and management principals.
pub fn wallet_address(principal: &str) -> Result<String> {
    let account = account(principal)?;
    Ok(crate::hex(
        &ic_platform::icp_ledger_types::account_identifier(account.owner, &[0; 32]),
    ))
}

fn amount(bytes: &[u8], decimals: u32, unit: &str) -> Result<(String, String)> {
    let value: candid_codec::Nat = decoder::decode_one(bytes, LIMITS).map_err(Error::Decode)?;
    let value: u128 = value.0.try_into().map_err(supplier)?;
    let scale = 10_u128.pow(decimals);
    Ok((
        value.to_string(),
        format!(
            "{}.{:0width$} {unit}",
            value / scale,
            value % scale,
            width = decimals as usize
        ),
    ))
}

fn report(
    ledger: &'static str,
    result: Result<Vec<u8>>,
    decimals: u32,
    unit: &str,
    observed_at_ns: u64,
) -> WalletBalance {
    let (amount, display, observation_error) =
        match result.and_then(|bytes| amount(&bytes, decimals, unit)) {
            Ok((amount, display)) => (Some(amount), Some(display), None),
            Err(error) => (
                None,
                None,
                Some(error.to_string().chars().take(500).collect()),
            ),
        };
    WalletBalance {
        ledger,
        amount,
        display,
        observed_at_ns,
        observation_error,
    }
}

async fn balance(
    agent: &Agent,
    ledger: &'static str,
    args: &[u8],
    decimals: u32,
    unit: &str,
) -> Result<WalletBalance> {
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        agent
            .query(&ledger.parse().map_err(supplier)?, "icrc1_balance_of")
            .with_arg(args)
            .call()
            .await
            .map_err(supplier)
    })
    .await
    .map_err(|_| Error::Blocked("wallet balance observation timed out".into()))
    .and_then(|value| value);
    Ok(report(ledger, result, decimals, unit, crate::now()?))
}

/// Observe both mainnet balances concurrently, with a five-second bound per ledger.
/// Uses the fixed IC trust root and anonymous queries; it never signs or updates.
///
/// # Errors
/// Refuses an invalid owner or local agent/clock failure. Provider failures remain
/// explicit per-balance unknowns so the receive address and other balance survive.
pub async fn wallet(principal: &str) -> Result<Wallet> {
    let account = account(principal)?;
    let icp_address = wallet_address(principal)?;
    let args = candid_codec::encode_one(&account, 256).map_err(Error::Encode)?;
    let agent = Network::Mainnet.agent()?;
    let (icp, cycles) = tokio::join!(
        balance(&agent, ICP_LEDGER, &args, 8, "ICP"),
        balance(&agent, CYCLES_LEDGER, &args, 12, "T cycles")
    );
    Ok(Wallet {
        principal: account.owner.to_text(),
        icp_address,
        network: "ic / mainnet",
        certified: false,
        icp: icp?,
        cycles: cycles?,
    })
}
