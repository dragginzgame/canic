use crate::{
    Error, cdk::call::Call, env::nns::ICP_LEDGER_CANISTER, interface::prelude::*,
    spec::icrc::icrc2::AllowanceArgs,
};

/// icp_icrc2_allowance
/// wrapper using the ICP_LEDGER_CANISTER
pub async fn icp_icrc2_allowance(
    account: impl Into<Account>,
    spender: impl Into<Account>,
) -> Result<Nat, Error> {
    icrc2_allowance(*ICP_LEDGER_CANISTER, account, spender).await
}

///
/// icrc2_allowance
/// this is generic for any ICRC-2 ledger (ICP, ckBTC, ckETH, SNS ledgers, etc.).
///
pub async fn icrc2_allowance(
    ledger_pid: Principal,
    account: impl Into<Account>,
    spender: impl Into<Account>,
) -> Result<Nat, Error> {
    let args = AllowanceArgs {
        account: account.into(),
        spender: spender.into(),
    };

    let res = Call::unbounded_wait(ledger_pid, "icrc2_allowance")
        .with_args(&(args,))
        .await
        .map_err(InterfaceError::CallFailed)?;

    let allowance: Nat = res.candid().map_err(InterfaceError::CandidDecodeFailed)?;

    Ok(allowance)
}
