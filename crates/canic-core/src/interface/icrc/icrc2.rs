use crate::{
    Error, env::nns::ICP_LEDGER_CANISTER, interface::prelude::*, spec::icrc::icrc2::AllowanceArgs,
};

///
/// icp_icrc2_allowance
/// Convenience helper targeting the ICP ledger canister.
///
pub async fn icp_icrc2_allowance(
    account: impl Into<Account>,
    spender: impl Into<Account>,
) -> Result<Nat, Error> {
    icrc2_allowance(*ICP_LEDGER_CANISTER, account, spender).await
}

///
/// icrc2_allowance
/// Generic ICRC-2 allowance lookup for any compatible ledger.
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
        .await?;

    let allowance: Nat = res.candid()?;

    Ok(allowance)
}
