use crate::{
    Error,
    env::nns::ICP_LEDGER_CANISTER,
    interface::{icrc::icrc2::icrc2_allowance, prelude::*},
};

///
/// icp_icrc2_allowance
/// Convenience wrapper around `icrc2_allowance` targeting the ICP ledger.
///
pub async fn icp_icrc2_allowance(
    account: impl Into<Account>,
    spender: impl Into<Account>,
) -> Result<Nat, Error> {
    icrc2_allowance(*ICP_LEDGER_CANISTER, account, spender).await
}
