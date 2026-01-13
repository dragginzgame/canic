use crate::{
    InternalError, InternalErrorOrigin,
    infra::ic::ledger::{LedgerInfra, LedgerMeta},
    ops::ic::{IcOps, ledger::LedgerOps},
    workflow::prelude::*,
};

///
/// LedgerWorkflow
///

pub struct LedgerWorkflow;

impl LedgerWorkflow {
    /// transfer_with_allowance_check
    pub async fn transfer_with_allowance_check(
        ledger_id: Principal,
        payer: Principal,
        spender: Account,
        to: Account,
        amount: u64,
        memo: Option<Vec<u8>>,
    ) -> Result<u64, InternalError> {
        LedgerRules::validate_allowance(ledger_id, payer, spender, amount).await?;

        let block_index = LedgerOps::transfer_from(ledger_id, payer, to, amount, memo).await?;

        Ok(block_index)
    }
}

///
/// LedgerRules
///

pub struct LedgerRules;

impl LedgerRules {
    /// Return best-effort metadata for a ledger canister.
    #[must_use]
    pub fn ledger_meta(ledger_id: Principal) -> LedgerMeta {
        LedgerInfra::ledger_meta(ledger_id)
    }

    /// Validate that `payer` has approved at least `required_amount` for `spender`.
    pub async fn validate_allowance(
        ledger_id: Principal,
        payer: Principal,
        spender: Account,
        required_amount: u64,
    ) -> Result<(), InternalError> {
        let meta = Self::ledger_meta(ledger_id);
        let symbol = meta.symbol;

        let payer_account = Account {
            owner: payer,
            subaccount: None,
        };
        let allowance_record = LedgerOps::allowance(ledger_id, payer_account, spender).await?;
        let approved_amount = allowance_record.allowance;

        if approved_amount < required_amount {
            return Err(InternalError::domain(
                InternalErrorOrigin::Workflow,
                format!(
                    "insufficient {symbol} allowance: has {approved_amount}, needs {required_amount}"
                ),
            ));
        }

        if let Some(expires_at_nanos) = allowance_record.expires_at {
            let now_nanos = IcOps::now_nanos();

            if expires_at_nanos <= now_nanos {
                return Err(InternalError::domain(
                    InternalErrorOrigin::Workflow,
                    format!("{symbol} allowance expired at {expires_at_nanos} (now {now_nanos})"),
                ));
            }
        }

        Ok(())
    }
}
