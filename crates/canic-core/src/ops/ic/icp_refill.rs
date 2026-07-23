//! Module: ops::ic::icp_refill
//!
//! Responsibility: expose ICP refill ledger and CMC calls through approved ops APIs.
//! Does not own: refill policy, durable refill records, or cost-guard decisions.
//! Boundary: delegates refill mechanics to infra under workflow-owned permits.

use crate::{
    InternalError,
    ids::BuildNetwork,
    infra::ic::{
        IcInfraError,
        icp_refill::{
            IcpRefillCanisterOverrides, IcpRefillCanisters, IcpRefillInfra,
            IcpXdrConversionRateResponse, NotifyTopUpArg, NotifyTopUpError, TransferArg,
            TransferError,
        },
    },
    ops::{OpsError, cost_guard::CostGuardPermit},
};
use candid::{Nat, Principal};

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

    pub fn cmc_topup_subaccount(target_canister: Principal) -> Result<[u8; 32], InternalError> {
        map_infra(IcpRefillInfra::cmc_topup_subaccount(target_canister))
    }

    #[must_use]
    pub fn transfer_arg(
        from_subaccount: Option<[u8; 32]>,
        to_owner: Principal,
        to_subaccount: Option<[u8; 32]>,
        amount_e8s: u64,
        fee_e8s: u64,
        memo: Vec<u8>,
        created_at_time_ns: u64,
    ) -> TransferArg {
        IcpRefillInfra::transfer_arg(
            from_subaccount,
            to_owner,
            to_subaccount,
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

fn map_infra<T>(result: Result<T, IcInfraError>) -> Result<T, InternalError> {
    result.map_err(OpsError::from).map_err(InternalError::from)
}
