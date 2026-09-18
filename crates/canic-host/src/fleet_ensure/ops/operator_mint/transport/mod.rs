//! Module: fleet_ensure::ops::operator_mint::transport
//!
//! Responsibility: bounded exact calls and authenticated receipt acquisition.
//! Boundary: workflow must retain approval before any payment or notification.

use crate::{
    fleet_ensure::{
        model::operator_mint::{OperatorMintIntentRecord, OperatorMintNotificationOutcomeRecord},
        ops::operator_mint::receipts::{
            self, CyclesLedgerBlock, DepositVerificationError, ReceiptVerificationLimits,
            VerifiedCyclesDeposit,
            icp::{
                self, IcpReadError, IcpReadLimits, IcpReadOutcome, IcpTransferRead,
                VerifiedIcpTransfer,
            },
        },
        view::operator_mint::OperatorMintRateQuote,
    },
    icp::{IcpCli, IcpManagementCallError},
};
use candid::{CandidType, Nat, Principal};
use ic_agent::{Agent, AgentError, agent::RequestStatusResponse};
use icrc_ledger_types::icrc3::blocks::{GetBlocksRequest, ICRC3DataCertificate};
use serde::{Deserialize, de::DeserializeOwned};
use std::time::Duration;
use thiserror::Error;

#[derive(CandidType, Deserialize)]
struct Rate {
    timestamp_seconds: u64,
    xdr_permyriad_per_icp: u64,
}

#[derive(CandidType, Deserialize)]
struct RateReply {
    data: Rate,
}

// These are bounded observation budgets, not a payment validity window.
const RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const CALL_TIMEOUT: Duration = Duration::from_secs(120);
const DEPOSIT_LIMITS: ReceiptVerificationLimits = ReceiptVerificationLimits {
    certificate_bytes: 256 * 1024,
    blocks: 1024,
    value_nodes: 100_000,
    value_bytes: RESPONSE_BYTES,
    depth: 32,
};
const ICP_LIMITS: IcpReadLimits = IcpReadLimits {
    certificate_bytes: RESPONSE_BYTES,
    certificate_depth: 32,
    reply_bytes: 1024 * 1024,
    decoding_quota: 2_000_000,
    skipping_quota: 100_000,
};

///
/// OperatorMintTransportError
///
/// Ops-owned bounded observation failure; none permits a replacement payment.
///
#[derive(Debug, Error)]
pub enum OperatorMintTransportError {
    #[error(transparent)]
    Agent(#[from] AgentError),
    #[error("operator mint response exceeded its observation budget")]
    BudgetExceeded,
    #[error("operator mint response encoding or decoding failed")]
    Candid(#[from] candid::Error),
    #[error(transparent)]
    Deposit(#[from] DepositVerificationError),
    #[error(transparent)]
    Identity(Box<IcpManagementCallError>),
    #[error(transparent)]
    Icp(Box<IcpReadError>),
    #[error("operator mint receipt remains unavailable; retain the original intent")]
    ReceiptUnavailable,
    #[error("operator mint async runtime could not start")]
    Runtime(#[from] std::io::Error),
    #[error("operator mint selected network or signer differs from the retained review")]
    ReaderMismatch,
    #[error("operator mint response could not be serialized for verification")]
    Serialization,
    #[error("operator mint call timed out; its outcome remains unresolved")]
    Timeout,
    #[error(transparent)]
    Wire(#[from] crate::fleet_ensure::ops::operator_mint::OperatorMintWireError),
}

impl From<IcpManagementCallError> for OperatorMintTransportError {
    fn from(error: IcpManagementCallError) -> Self {
        Self::Identity(Box::new(error))
    }
}

impl From<IcpReadError> for OperatorMintTransportError {
    fn from(error: IcpReadError) -> Self {
        Self::Icp(Box::new(error))
    }
}

///
/// OperatorMintTransport
///
/// Ops-owned Agent fixed to the selected signer, network and HTTP byte budget.
///
pub struct OperatorMintTransport {
    agent: Agent,
}

#[derive(CandidType, Deserialize)]
struct BlocksReply {
    blocks: Vec<CyclesLedgerBlock>,
}

impl OperatorMintTransport {
    /// Read only the selected operator's default Cycles Ledger account.
    pub fn operator_balance(&self, ledger: Principal) -> Result<u128, OperatorMintTransportError> {
        tokio::runtime::Runtime::new()?.block_on(self.read_operator_balance(ledger))
    }

    /// Async observation of the same default account; not receipt evidence.
    pub async fn read_operator_balance(
        &self,
        ledger: Principal,
    ) -> Result<u128, OperatorMintTransportError> {
        let account = icrc_ledger_types::icrc1::account::Account {
            owner: self.operator()?,
            subaccount: None,
        };
        let amount: Nat = self
            .query(ledger, "icrc1_balance_of", candid::encode_one(account)?)
            .await?;
        (&amount.0)
            .try_into()
            .map_err(|_| OperatorMintTransportError::BudgetExceeded)
    }
    /// Read the advisory quote for a synchronous host command.
    pub fn quote_blocking(
        &self,
        icp_ledger: Principal,
        cmc: Principal,
        cycles_ledger: Principal,
    ) -> Result<OperatorMintRateQuote, OperatorMintTransportError> {
        tokio::runtime::Runtime::new()?.block_on(self.quote(icp_ledger, cmc, cycles_ledger))
    }
    #[cfg(test)]
    pub(in crate::fleet_ensure) const fn from_test_agent(agent: Agent) -> Self {
        Self { agent }
    }

    /// Observe fees and an advisory CMC rate without submitting any payment.
    pub async fn quote(
        &self,
        icp_ledger: Principal,
        cmc: Principal,
        cycles_ledger: Principal,
    ) -> Result<OperatorMintRateQuote, OperatorMintTransportError> {
        let rate: RateReply = self
            .query(cmc, "get_icp_xdr_conversion_rate", candid::encode_args(())?)
            .await?;
        let transfer: Nat = self
            .query(icp_ledger, "icrc1_fee", candid::encode_args(())?)
            .await?;
        let deposit: Nat = self
            .query(cycles_ledger, "icrc1_fee", candid::encode_args(())?)
            .await?;
        Ok(OperatorMintRateQuote {
            rate_timestamp_seconds: rate.data.timestamp_seconds,
            xdr_permyriad_per_icp: rate.data.xdr_permyriad_per_icp,
            transfer_fee_e8s: (&transfer.0)
                .try_into()
                .map_err(|_| OperatorMintTransportError::BudgetExceeded)?,
            estimated_deposit_fee_cycles: (&deposit.0)
                .try_into()
                .map_err(|_| OperatorMintTransportError::BudgetExceeded)?,
        })
    }
    /// Resolve the existing ICP identity and trusted network; no payment occurs.
    pub fn from_icp(icp: &IcpCli) -> Result<Self, OperatorMintTransportError> {
        Ok(Self {
            agent: icp.authenticated_agent_with_response_limit(RESPONSE_BYTES)?,
        })
    }

    /// Selected trust-anchor identity for the effect-free review.
    #[must_use]
    pub fn network_identity(&self) -> [u8; 32] {
        receipts::network_identity_sha256(&self.agent.read_root_key())
    }

    /// Canonical identity of the same trust anchor used by this transport.
    pub fn canonical_network_id(
        &self,
    ) -> Result<canic_core::ids::CanonicalNetworkId, OperatorMintTransportError> {
        canic_core::ids::CanonicalNetworkId::from_der_root_trust_anchor(&self.agent.read_root_key())
            .map_err(|_| OperatorMintTransportError::ReaderMismatch)
    }

    /// Selected signer, checked against the desired operator before reviewing.
    pub fn operator(&self) -> Result<Principal, OperatorMintTransportError> {
        self.agent
            .get_principal()
            .map_err(|_| OperatorMintTransportError::ReaderMismatch)
    }

    /// Submit one exact, already-approved ICP transfer argument.
    pub async fn transfer(
        &self,
        intent: &OperatorMintIntentRecord,
        argument: &[u8],
    ) -> Result<Vec<u8>, OperatorMintTransportError> {
        self.verify_reader(intent)?;
        if argument != crate::fleet_ensure::ops::operator_mint::transfer_argument(intent)? {
            return Err(OperatorMintTransportError::ReaderMismatch);
        }
        self.update(intent.authority.icp_ledger, "transfer", argument)
            .await
    }

    /// Notify only for an authenticated ICP block and already-retained arguments.
    pub async fn notify(
        &self,
        transfer: &VerifiedIcpTransfer,
        argument: &[u8],
    ) -> Result<Vec<u8>, OperatorMintTransportError> {
        self.verify_reader(transfer.intent())?;
        if argument
            != crate::fleet_ensure::ops::operator_mint::notification_argument(
                transfer.intent(),
                transfer.block_index(),
            )?
        {
            return Err(OperatorMintTransportError::ReaderMismatch);
        }
        self.update(
            transfer.intent().authority.cmc,
            "notify_mint_cycles",
            argument,
        )
        .await
    }

    async fn update(
        &self,
        canister: Principal,
        method: &str,
        argument: &[u8],
    ) -> Result<Vec<u8>, OperatorMintTransportError> {
        Ok(tokio::time::timeout(
            CALL_TIMEOUT,
            self.agent
                .update(&canister, method)
                .with_arg(argument)
                .call_and_wait(),
        )
        .await
        .map_err(|_| OperatorMintTransportError::Timeout)??)
    }

    fn verify_reader(
        &self,
        intent: &OperatorMintIntentRecord,
    ) -> Result<(), OperatorMintTransportError> {
        if self.operator()? != intent.authority.operator
            || self.network_identity() != intent.authority.network_identity_sha256
        {
            return Err(OperatorMintTransportError::ReaderMismatch);
        }
        Ok(())
    }

    /// Authenticate one ICP block with at most one Ledger-authorized archive hop.
    pub async fn read_transfer(
        &self,
        intent: &OperatorMintIntentRecord,
        block: u64,
    ) -> Result<VerifiedIcpTransfer, OperatorMintTransportError> {
        self.verify_reader(intent)?;
        let read = icp::prepare_read(&self.agent, intent, block)?;
        match self.read_icp(&read).await? {
            IcpReadOutcome::Transfer(transfer) => Ok(transfer),
            IcpReadOutcome::Archive(archive) => {
                let read = icp::prepare_archive_read(&self.agent, &archive)?;
                match self.read_icp(&read).await? {
                    IcpReadOutcome::Transfer(transfer) => Ok(transfer),
                    IcpReadOutcome::Archive(_) => {
                        Err(OperatorMintTransportError::ReceiptUnavailable)
                    }
                }
            }
        }
    }

    async fn read_icp(
        &self,
        read: &IcpTransferRead,
    ) -> Result<IcpReadOutcome, OperatorMintTransportError> {
        tokio::time::timeout(CALL_TIMEOUT, async {
            self.agent
                .update_signed(read.canister(), read.signed_request().to_vec())
                .await?;
            loop {
                let (status, certificate) = self
                    .agent
                    .request_status_raw(read.request_id(), read.canister())
                    .await?;
                match status {
                    RequestStatusResponse::Replied(_) => {
                        let mut bytes = Vec::new();
                        ciborium::into_writer(&certificate, &mut bytes)
                            .map_err(|_| OperatorMintTransportError::Serialization)?;
                        return Ok(icp::verify_read(&self.agent, read, &bytes, ICP_LIMITS)?);
                    }
                    RequestStatusResponse::Rejected(_) | RequestStatusResponse::Done => {
                        return Err(OperatorMintTransportError::ReceiptUnavailable);
                    }
                    _ => tokio::time::sleep(Duration::from_secs(1)).await,
                }
            }
        })
        .await
        .map_err(|_| OperatorMintTransportError::Timeout)?
    }

    /// Fetch a bounded certified tip-to-deposit chain; incomplete history stays unresolved.
    pub async fn read_deposit(
        &self,
        intent: &OperatorMintIntentRecord,
        outcome: &OperatorMintNotificationOutcomeRecord,
    ) -> Result<VerifiedCyclesDeposit, OperatorMintTransportError> {
        self.verify_reader(intent)?;
        let OperatorMintNotificationOutcomeRecord::Minted {
            deposit_block_index,
            ..
        } = outcome
        else {
            return Err(OperatorMintTransportError::ReceiptUnavailable);
        };
        let ledger = intent.authority.cycles_ledger;
        let evidence: Option<ICRC3DataCertificate> = self
            .query(
                ledger,
                "icrc3_get_tip_certificate",
                candid::encode_args(())?,
            )
            .await?;
        let evidence = evidence.ok_or(OperatorMintTransportError::ReceiptUnavailable)?;
        let tip = receipts::certified_tip_index(&self.agent, intent, &evidence, DEPOSIT_LIMITS)?;
        let count = tip
            .checked_sub(*deposit_block_index)
            .and_then(|n| n.checked_add(1))
            .filter(|count| *count <= DEPOSIT_LIMITS.blocks as u128)
            .ok_or(OperatorMintTransportError::BudgetExceeded)?;
        let reply: BlocksReply = self
            .query(
                ledger,
                "icrc3_get_blocks",
                candid::encode_one(vec![GetBlocksRequest {
                    start: Nat::from(*deposit_block_index),
                    length: Nat::from(count),
                }])?,
            )
            .await?;
        let mut blocks = reply.blocks;
        blocks.sort_by(|a, b| b.id.cmp(&a.id));
        Ok(receipts::verify_cycles_deposit(
            &self.agent,
            intent,
            outcome,
            &evidence,
            &blocks,
            DEPOSIT_LIMITS,
        )?)
    }

    async fn query<T: CandidType + DeserializeOwned>(
        &self,
        canister: Principal,
        method: &str,
        args: Vec<u8>,
    ) -> Result<T, OperatorMintTransportError> {
        let bytes = tokio::time::timeout(
            CALL_TIMEOUT,
            self.agent.query(&canister, method).with_arg(args).call(),
        )
        .await
        .map_err(|_| OperatorMintTransportError::Timeout)??;
        let mut config = candid::de::DecoderConfig::new();
        config
            .set_decoding_quota(2_000_000)
            .set_skipping_quota(100_000)
            .set_max_type_len(10_000);
        Ok(candid::utils::decode_one_with_config(&bytes, &config)?)
    }
}
