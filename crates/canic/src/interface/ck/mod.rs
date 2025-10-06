//! ck Interfaces
//! Convenience bindings for ck-token ledgers deployed by the IC team.

use crate::{
    Error, env,
    interface::{icrc::icrc2::icrc2_allowance, prelude::*},
};

///
/// CkToken
/// Enumerates supported ck-ledger canisters with helper methods.
///

#[derive(Clone, Copy, Debug)]
pub enum CkToken {
    CkBtc,
    CkEth,

    // ERC-20
    CkLink,
    CkOct,
    CkPepe,
    CkUsdc,
    CkUsdt,
}

impl CkToken {
    #[must_use]
    pub fn ledger_canister(&self) -> Principal {
        match &self {
            Self::CkBtc => *env::ck::CKBTC_LEDGER_CANISTER,
            Self::CkEth => *env::ck::CKETH_LEDGER_CANISTER,

            Self::CkLink => *env::ck::CKLINK_LEDGER_CANISTER,
            Self::CkOct => *env::ck::CKOCT_LEDGER_CANISTER,
            Self::CkPepe => *env::ck::CKPEPE_LEDGER_CANISTER,
            Self::CkUsdc => *env::ck::CKUSDC_LEDGER_CANISTER,
            Self::CkUsdt => *env::ck::CKUSDT_LEDGER_CANISTER,
        }
    }
}

///
/// ck_icrc2_allowance
/// Retrieve the remaining allowance for a spender on a ck-ledger.
///
pub async fn ck_icrc2_allowance(
    token: CkToken,
    account: impl Into<Account>,
    spender: impl Into<Account>,
) -> Result<Nat, Error> {
    icrc2_allowance(token.ledger_canister(), account, spender).await
}
