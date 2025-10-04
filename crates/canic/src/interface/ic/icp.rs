use crate::{
    Error,
    env::nns::ICP_LEDGER_CANISTER,
    interface::{icrc::icrc2::icrc2_allowance, prelude::*},
};

/// icp_icrc2_allowance
/// wrapper using the ICP_LEDGER_CANISTER
pub async fn icp_icrc2_allowance(
    account: impl Into<Account>,
    spender: impl Into<Account>,
) -> Result<Nat, Error> {
    icrc2_allowance(*ICP_LEDGER_CANISTER, account, spender).await
}
