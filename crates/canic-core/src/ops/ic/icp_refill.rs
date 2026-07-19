//! Module: ops::ic::icp_refill
//!
//! Responsibility: expose ICP refill ledger and CMC calls through approved ops APIs.
//! Does not own: refill policy, durable refill records, or cost-guard decisions.
//! Boundary: delegates refill mechanics to infra under workflow-owned permits.

use crate::{
    InternalError,
    cdk::{
        candid::Nat,
        types::{Account, Principal, Subaccount},
    },
    ids::BuildNetwork,
    infra::{
        InfraError,
        ic::icp_refill::{
            IcpRefillCanisterOverrides, IcpRefillCanisters, IcpRefillInfra,
            IcpXdrConversionRateResponse, NotifyTopUpArg, NotifyTopUpError, TransferArg,
            TransferError,
        },
    },
    ops::{cost_guard::CostGuardPermit, ic::IcOpsError},
};
use thiserror::Error as ThisError;

///
/// IcpRefillOpsError
///
/// Typed failure surface for ICP refill IC operations.
///

#[derive(Debug, ThisError)]
pub enum IcpRefillOpsError {
    #[error(transparent)]
    Infra(#[from] InfraError),
}

impl From<IcpRefillOpsError> for InternalError {
    fn from(err: IcpRefillOpsError) -> Self {
        IcOpsError::from(err).into()
    }
}

///
/// IcpRefillOps
///
/// Operations-layer facade for ICP refill ledger and CMC interactions.
///

pub struct IcpRefillOps;

impl IcpRefillOps {
    #[must_use]
    pub fn topup_memo() -> Vec<u8> {
        IcpRefillInfra::topup_memo()
    }

    pub fn cmc_topup_account(
        cmc_canister_id: Principal,
        target_canister: Principal,
    ) -> Result<Account, InternalError> {
        map_infra(IcpRefillInfra::cmc_topup_account(
            cmc_canister_id,
            target_canister,
        ))
    }

    #[must_use]
    pub fn transfer_arg(
        from_subaccount: Option<Subaccount>,
        to: Account,
        amount_e8s: u64,
        fee_e8s: u64,
        memo: Vec<u8>,
        created_at_time_ns: u64,
    ) -> TransferArg {
        IcpRefillInfra::transfer_arg(
            from_subaccount,
            to,
            amount_e8s,
            fee_e8s,
            memo,
            created_at_time_ns,
        )
    }

    pub fn checked_block_index(block_index: Nat) -> Result<u64, InternalError> {
        map_infra(IcpRefillInfra::checked_block_index(block_index))
    }

    pub fn resolve_canisters(
        build_network: BuildNetwork,
        overrides: IcpRefillCanisterOverrides,
    ) -> Result<IcpRefillCanisters, InternalError> {
        map_infra(IcpRefillInfra::resolve_canisters(build_network, overrides))
    }

    pub async fn icrc1_fee(ledger_id: Principal) -> Result<Nat, InternalError> {
        map_infra(IcpRefillInfra::icrc1_fee(ledger_id).await)
    }

    pub async fn icrc1_decimals(ledger_id: Principal) -> Result<u8, InternalError> {
        map_infra(IcpRefillInfra::icrc1_decimals(ledger_id).await)
    }

    pub async fn icrc1_transfer(
        _permit: &CostGuardPermit,
        ledger_id: Principal,
        args: TransferArg,
    ) -> Result<Result<Nat, TransferError>, InternalError> {
        map_infra(IcpRefillInfra::icrc1_transfer(ledger_id, args).await)
    }

    pub async fn notify_top_up(
        _permit: &CostGuardPermit,
        cmc_id: Principal,
        args: NotifyTopUpArg,
    ) -> Result<Result<Nat, NotifyTopUpError>, InternalError> {
        map_infra(IcpRefillInfra::notify_top_up(cmc_id, args).await)
    }

    pub async fn get_icp_xdr_conversion_rate(
        cmc_id: Principal,
    ) -> Result<IcpXdrConversionRateResponse, InternalError> {
        map_infra(IcpRefillInfra::get_icp_xdr_conversion_rate(cmc_id).await)
    }
}

fn map_infra<T>(result: Result<T, InfraError>) -> Result<T, InternalError> {
    result
        .map_err(IcpRefillOpsError::from)
        .map_err(InternalError::from)
}
