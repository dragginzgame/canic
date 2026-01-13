use crate::{api::prelude::*, workflow::ic::ledger::LedgerWorkflow};

///
/// LedgerApi
///

pub struct LedgerApi;

impl LedgerApi {
    /// Transfer tokens using an existing ICRC-2 allowance.
    ///
    /// This is the approved endpoint-facing API.
    pub async fn transfer_from(
        ledger_id: Principal,
        payer: Principal,
        spender: Account,
        to: Account,
        amount: u64,
        memo: Option<Vec<u8>>,
    ) -> Result<u64, Error> {
        LedgerWorkflow::transfer_with_allowance_check(ledger_id, payer, spender, to, amount, memo)
            .await
            .map_err(Error::from)
    }
}
