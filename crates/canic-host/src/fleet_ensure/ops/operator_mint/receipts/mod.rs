//! Module: fleet_ensure::ops::operator_mint::receipts
//!
//! Responsibility: authenticate Ledger evidence against retained mint intent.
//! Boundary: no calls, journal mutation or credit admission.

mod blocks;
mod certificate;
pub mod icp;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    model::operator_mint::{OperatorMintIntentRecord, OperatorMintNotificationOutcomeRecord},
    ops::operator_mint::{OperatorMintWireError, validate_intent},
};
use candid::{CandidType, Nat};
use ic_agent::Agent;
use icrc_ledger_types::{icrc::generic_value::Value, icrc3::blocks::ICRC3DataCertificate};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub use certificate::CertificateVerificationError;

/// Exact block projection returned by the reviewed Cycles Ledger wire contract.
///
/// Ops retains upstream `Value`, including its declared `Nat64` alternative;
/// the narrower `ICRC3Value` is not this Ledger's declared response type.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CyclesLedgerBlock {
    /// Ledger-local block index; bounded before admission.
    pub id: Nat,
    /// Untrusted generic block, authenticated only by the receipt verifier.
    pub block: Value,
}

/// Ops-owned work bounds selected by the receipt reader before consuming evidence.
/// The transport must separately bound response bytes before Candid decoding.
#[derive(Clone, Copy, Debug)]
pub struct ReceiptVerificationLimits {
    /// Combined certificate and witness byte budget.
    pub certificate_bytes: usize,
    /// Maximum contiguous blocks from the certified tip through the deposit.
    pub blocks: usize,
    /// Aggregate generic-value nodes across those blocks.
    pub value_nodes: usize,
    /// Aggregate scalar and map-key bytes before hashing values.
    pub value_bytes: usize,
    /// Maximum nesting for generic values and the outer CBOR evidence.
    pub depth: u8,
}

/// Ops-owned typed rejection; every failure leaves the conversion uncredited.
#[derive(Debug, Error)]
pub enum DepositVerificationError {
    #[error("mint intent is invalid")]
    Intent(#[from] OperatorMintWireError),
    #[error("retained reply has no mint deposit locator")]
    MissingMintLocator,
    #[error("receipt reader trust anchor differs from the reviewed network")]
    NetworkMismatch,
    #[error("receipt exceeds its verification budget")]
    BudgetExceeded,
    #[error(transparent)]
    Certificate(#[from] CertificateVerificationError),
    #[error("certificate does not bind this Ledger and tip witness")]
    WitnessMismatch,
    #[error("receipt does not provide the exact contiguous chain to its deposit")]
    ChainMismatch,
    #[error("receipt contains a block value unsupported by the ICRC3 hasher")]
    UnsupportedBlockValue,
    #[error("deposit block does not use the supported Cycles Ledger mint schema")]
    UnsupportedDeposit,
    #[error("deposit account or memo differs from the retained intent")]
    DepositBindingMismatch,
    #[error("deposit amounts are invalid or differ from the retained mint result")]
    AmountMismatch,
}

/// Authenticated deposit evidence owned by ops; only verification constructs it.
/// This ephemeral value cannot be deserialized or used as proof of the ICP debit.
#[derive(Debug)]
pub struct VerifiedCyclesDeposit {
    intent: OperatorMintIntentRecord,
    block_index: u128,
    net_credit_cycles: u128,
    deposit_fee_cycles: u128,
}

impl VerifiedCyclesDeposit {
    /// Exact intent whose destination, memo and network were verified.
    #[must_use]
    pub const fn intent(&self) -> &OperatorMintIntentRecord {
        &self.intent
    }

    /// Authenticated deposit identity within the intent's Cycles Ledger.
    #[must_use]
    pub const fn block_index(&self) -> u128 {
        self.block_index
    }

    /// Actual account credit, excluding the deposit fee.
    #[must_use]
    pub const fn net_credit_cycles(&self) -> u128 {
        self.net_credit_cycles
    }

    /// Fee recorded inside the mint transaction, not its block-level zero fee.
    #[must_use]
    pub const fn deposit_fee_cycles(&self) -> u128 {
        self.deposit_fee_cycles
    }
}

/// Bind a reviewed network to its selected DER root key, independently of gateway URL.
///
/// Obtain that key through the selected network's trusted configuration, never
/// from the untrusted receipt response being verified.
#[must_use]
pub fn network_identity_sha256(root_key: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"canic:operator-mint:network-root:v1\0");
    hash.update(root_key);
    hash.finalize().into()
}

/// Authenticate the tip before selecting a bounded history range to fetch.
pub fn certified_tip_index(
    agent: &Agent,
    intent: &OperatorMintIntentRecord,
    evidence: &ICRC3DataCertificate,
    limits: ReceiptVerificationLimits,
) -> Result<u128, DepositVerificationError> {
    validate_intent(intent)?;
    if network_identity_sha256(&agent.read_root_key()) != intent.authority.network_identity_sha256 {
        return Err(DepositVerificationError::NetworkMismatch);
    }
    Ok(certificate::verify(agent, intent.authority.cycles_ledger, evidence, limits)?.index)
}

/// Verify a current Cycles Ledger mint using blocks in descending index order.
///
/// The configured Agent owns certificate time/delegation validation; its trusted
/// root key must remain fixed throughout verification. No query or update is sent.
/// A successful result still needs an authenticated ICP transfer and once-only
/// journal admission before funding can resume.
pub fn verify_cycles_deposit(
    agent: &Agent,
    intent: &OperatorMintIntentRecord,
    outcome: &OperatorMintNotificationOutcomeRecord,
    evidence: &ICRC3DataCertificate,
    chain: &[CyclesLedgerBlock],
    limits: ReceiptVerificationLimits,
) -> Result<VerifiedCyclesDeposit, DepositVerificationError> {
    validate_intent(intent)?;
    let OperatorMintNotificationOutcomeRecord::Minted {
        deposit_block_index,
        gross_minted_cycles,
        ..
    } = outcome
    else {
        return Err(DepositVerificationError::MissingMintLocator);
    };
    let root_key = agent.read_root_key();
    if network_identity_sha256(&root_key) != intent.authority.network_identity_sha256 {
        return Err(DepositVerificationError::NetworkMismatch);
    }
    blocks::check_budget(chain, limits)?;
    let tip = certificate::verify(agent, intent.authority.cycles_ledger, evidence, limits)?;
    // Detect configuration changes while the Agent was authenticating evidence.
    if agent.read_root_key() != root_key {
        return Err(DepositVerificationError::NetworkMismatch);
    }
    let deposit = blocks::verify_chain(chain, &tip, *deposit_block_index)?;
    let (net_credit_cycles, deposit_fee_cycles) =
        blocks::deposit_amounts(deposit, intent, *gross_minted_cycles)?;
    Ok(VerifiedCyclesDeposit {
        intent: intent.clone(),
        block_index: *deposit_block_index,
        net_credit_cycles,
        deposit_fee_cycles,
    })
}
