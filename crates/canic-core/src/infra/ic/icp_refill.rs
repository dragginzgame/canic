//! Module: infra::ic::icp_refill
//!
//! Responsibility: perform raw ICP ledger and cycles minting canister refill calls.
//! Does not own: refill policy, replay recovery, or endpoint error mapping.
//! Boundary: ops calls this after policy approves an ICP-to-cycles refill attempt.

use crate::infra::ic::{
    IcInfraError,
    call::Call,
    known::{CYCLES_MINTING_CANISTER, ICP_LEDGER_CANISTER},
};
use canic_contracts::ids::BuildNetwork;
use ic_platform::{candid::contract::Nat, principal::ExternalPrincipal as Principal};
use thiserror::Error as ThisError;

const CMC_TOPUP_MEMO_BYTES: &[u8] = b"TPUP\0\0\0\0";
const CMC_TOPUP_SUBACCOUNT_MAX_PRINCIPAL_BYTES: usize = 31;

pub use ic_platform::{
    cmc_types::{IcpXdrConversionRateResponse, NotifyError, NotifyTopUpArg},
    icrc_ledger_types::{Account, Memo, TransferArg, TransferError},
};

///
/// IcpRefillInfraError
///
/// Mechanical ICP refill failure returned by ledger and CMC helpers.
/// Owned by ICP refill infra and converted into `IcInfraError`.
///

#[derive(Debug, ThisError)]
pub enum IcpRefillInfraError {
    #[error("ledger block index {value} does not fit in u64")]
    LedgerBlockIndexOverflow { value: Nat },

    #[error("build_network=ic rejects ICP ledger / CMC overrides without unsafe override flag")]
    MainnetSystemCanisterOverrideRejected,

    #[error("target principal is too long for CMC top-up subaccount: len={len}")]
    PrincipalTooLongForCmcSubaccount { len: usize },
}

///
/// IcpRefillCanisterOverrides
///
/// Optional ledger and CMC canister overrides for non-mainnet refill tests.
/// Owned by ICP refill infra and resolved before raw calls are issued.
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IcpRefillCanisterOverrides {
    pub ledger_canister_id: Option<Principal>,
    pub cmc_canister_id: Option<Principal>,
    pub allow_ic_overrides: bool,
}

///
/// IcpRefillCanisters
///
/// Resolved ledger and CMC canisters used for one refill flow.
/// Owned by ICP refill infra and consumed by ops refill orchestration.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IcpRefillCanisters {
    pub ledger_canister_id: Principal,
    pub cmc_canister_id: Principal,
}

///
/// IcpRefillInfra
///
/// Raw ICP refill adapter for ledger and cycles minting canister calls.
/// Owned by IC infra and consumed by ops refill adapters.
///

pub struct IcpRefillInfra;

impl IcpRefillInfra {
    /// Return the CMC top-up memo bytes.
    #[must_use]
    pub fn topup_memo() -> Vec<u8> {
        CMC_TOPUP_MEMO_BYTES.to_vec()
    }

    /// Build the CMC top-up subaccount for a target canister.
    pub fn cmc_topup_subaccount(target_canister: Principal) -> Result<[u8; 32], IcInfraError> {
        let bytes = target_canister.as_slice();
        if bytes.len() > CMC_TOPUP_SUBACCOUNT_MAX_PRINCIPAL_BYTES {
            return Err(
                IcpRefillInfraError::PrincipalTooLongForCmcSubaccount { len: bytes.len() }.into(),
            );
        }

        let mut subaccount = [0_u8; 32];
        subaccount[0] = u8::try_from(bytes.len()).expect("principal length fits in u8");
        subaccount[1..=bytes.len()].copy_from_slice(bytes);

        Ok(subaccount)
    }

    /// Build an ICRC-1 transfer argument for a refill transfer.
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
        TransferArg {
            from_subaccount,
            to: Account {
                owner: to_owner,
                subaccount: to_subaccount,
            },
            fee: Some(Nat::from(fee_e8s)),
            created_at_time: Some(created_at_time_ns),
            memo: Some(Memo::from(memo)),
            amount: Nat::from(amount_e8s),
        }
    }

    /// Convert a ledger block index into `u64` with overflow detection.
    pub fn checked_block_index(block_index: Nat) -> Result<u64, IcInfraError> {
        u64::try_from(block_index.0.clone()).map_err(|_| {
            IcpRefillInfraError::LedgerBlockIndexOverflow { value: block_index }.into()
        })
    }

    /// Resolve refill canister IDs while enforcing mainnet override rules.
    pub fn resolve_canisters(
        build_network: BuildNetwork,
        overrides: IcpRefillCanisterOverrides,
    ) -> Result<IcpRefillCanisters, IcInfraError> {
        if build_network == BuildNetwork::Ic
            && !overrides.allow_ic_overrides
            && (overrides.ledger_canister_id.is_some() || overrides.cmc_canister_id.is_some())
        {
            return Err(IcpRefillInfraError::MainnetSystemCanisterOverrideRejected.into());
        }

        Ok(IcpRefillCanisters {
            ledger_canister_id: overrides.ledger_canister_id.unwrap_or(*ICP_LEDGER_CANISTER),
            cmc_canister_id: overrides
                .cmc_canister_id
                .unwrap_or(*CYCLES_MINTING_CANISTER),
        })
    }

    /// Query `icrc1_fee` on the selected ICP ledger.
    pub async fn icrc1_fee(ledger_id: Principal) -> Result<Nat, IcInfraError> {
        Call::unbounded_wait(ledger_id, "icrc1_fee")
            .execute()
            .await?
            .candid()
    }

    /// Query `icrc1_decimals` on the selected ICP ledger.
    pub async fn icrc1_decimals(ledger_id: Principal) -> Result<u8, IcInfraError> {
        Call::unbounded_wait(ledger_id, "icrc1_decimals")
            .execute()
            .await?
            .candid()
    }

    /// Query the exact source account balance on the selected ICP ledger.
    pub async fn icrc1_balance_of(
        ledger_id: Principal,
        account: Account,
    ) -> Result<Nat, IcInfraError> {
        Call::unbounded_wait(ledger_id, "icrc1_balance_of")
            .with_arg(account)?
            .execute()
            .await?
            .candid()
    }

    /// Execute `icrc1_transfer` and return the raw ledger result.
    pub async fn icrc1_transfer(
        ledger_id: Principal,
        args: TransferArg,
    ) -> Result<Result<Nat, TransferError>, IcInfraError> {
        Call::unbounded_wait(ledger_id, "icrc1_transfer")
            .with_arg(args)?
            .execute()
            .await?
            .candid()
    }

    /// Notify the cycles minting canister about a top-up transfer.
    pub async fn notify_top_up(
        cmc_id: Principal,
        args: NotifyTopUpArg,
    ) -> Result<Result<Nat, NotifyError>, IcInfraError> {
        Call::unbounded_wait(cmc_id, "notify_top_up")
            .with_arg(args)?
            .execute()
            .await?
            .candid()
    }

    /// Query the cycles minting canister for the current ICP/XDR conversion rate.
    pub async fn get_icp_xdr_conversion_rate(
        cmc_id: Principal,
    ) -> Result<IcpXdrConversionRateResponse, IcInfraError> {
        Call::unbounded_wait(cmc_id, "get_icp_xdr_conversion_rate")
            .execute()
            .await?
            .candid()
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn principal(byte: u8) -> Principal {
        Principal::from_slice(&[byte; 29])
    }

    #[test]
    fn cmc_topup_subaccount_encodes_target_principal() {
        let target = principal(7);
        let subaccount = IcpRefillInfra::cmc_topup_subaccount(target)
            .expect("principal should fit in CMC subaccount");

        assert_eq!(subaccount[0], 29);
        assert_eq!(&subaccount[1..30], &[7_u8; 29]);
        assert_eq!(&subaccount[30..], &[0_u8; 2]);
    }

    #[test]
    fn checked_block_index_accepts_u64() {
        assert_eq!(
            IcpRefillInfra::checked_block_index(Nat::from(u64::MAX)).expect("u64 fits"),
            u64::MAX
        );
    }

    #[test]
    fn checked_block_index_rejects_overflow() {
        let too_large = Nat::from_str("18446744073709551616").expect("valid nat");

        assert!(matches!(
            IcpRefillInfra::checked_block_index(too_large),
            Err(IcInfraError::IcpRefillInfra(
                IcpRefillInfraError::LedgerBlockIndexOverflow { .. }
            ))
        ));
    }

    #[test]
    fn mainnet_resolution_rejects_overrides_without_flag() {
        let overrides = IcpRefillCanisterOverrides {
            ledger_canister_id: Some(principal(1)),
            cmc_canister_id: None,
            allow_ic_overrides: false,
        };

        assert!(matches!(
            IcpRefillInfra::resolve_canisters(BuildNetwork::Ic, overrides),
            Err(IcInfraError::IcpRefillInfra(
                IcpRefillInfraError::MainnetSystemCanisterOverrideRejected
            ))
        ));
    }

    #[test]
    fn mainnet_resolution_uses_canonical_ids_without_overrides() {
        let canisters = IcpRefillInfra::resolve_canisters(
            BuildNetwork::Ic,
            IcpRefillCanisterOverrides::default(),
        )
        .expect("canonical mainnet IDs should resolve");

        assert_eq!(canisters.ledger_canister_id, *ICP_LEDGER_CANISTER);
        assert_eq!(canisters.cmc_canister_id, *CYCLES_MINTING_CANISTER);
    }

    #[test]
    fn transfer_arg_preserves_recovery_identity() {
        let to_owner = principal(9);
        let to_subaccount = Some([8_u8; 32]);
        let args = IcpRefillInfra::transfer_arg(
            Some([7_u8; 32]),
            to_owner,
            to_subaccount,
            100_000_000,
            10_000,
            IcpRefillInfra::topup_memo(),
            42,
        );

        assert_eq!(args.from_subaccount, Some([7_u8; 32]));
        assert_eq!(args.to.owner, to_owner);
        assert_eq!(args.to.subaccount, to_subaccount);
        assert_eq!(args.amount, Nat::from(100_000_000_u64));
        assert_eq!(args.fee, Some(Nat::from(10_000_u64)));
        assert_eq!(args.created_at_time, Some(42));
        assert_eq!(args.memo, Some(Memo::from(b"TPUP\0\0\0\0".to_vec())));
    }
}
