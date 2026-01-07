use crate::{
    Error, ThisError,
    infra::ic::ledger::LedgerMeta,
    ops::ic::ledger::LedgerOps,
    workflow::{ic::IcWorkflowError, prelude::*},
};

///
/// LedgerWorkflowError
///

#[derive(Debug, ThisError)]
pub enum LedgerWorkflowError {
    #[error("insufficient {symbol} allowance: has {allowance}, needs {required}")]
    InsufficientAllowance {
        symbol: &'static str,
        allowance: u64,
        required: u64,
    },

    #[error("{symbol} allowance expired at {expires_at_nanos} (now {now_nanos})")]
    AllowanceExpired {
        symbol: &'static str,
        expires_at_nanos: u64,
        now_nanos: u64,
    },
}

impl From<LedgerWorkflowError> for Error {
    fn from(err: LedgerWorkflowError) -> Self {
        IcWorkflowError::LedgerWorkflow(err).into()
    }
}

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
    ) -> Result<u64, Error> {
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
        crate::infra::ic::ledger::ledger_meta(ledger_id)
    }

    /// Validate that `payer` has approved at least `required_amount` for `spender`.
    pub async fn validate_allowance(
        ledger_id: Principal,
        payer: Principal,
        spender: Account,
        required_amount: u64,
    ) -> Result<(), Error> {
        let meta = Self::ledger_meta(ledger_id);

        let payer_account = Account {
            owner: payer,
            subaccount: None,
        };

        let allowance = LedgerOps::allowance(ledger_id, payer_account, spender).await?;

        if allowance.allowance < required_amount {
            return Err(LedgerWorkflowError::InsufficientAllowance {
                symbol: meta.symbol,
                allowance: allowance.allowance,
                required: required_amount,
            }
            .into());
        }

        if let Some(expires_at_nanos) = allowance.expires_at {
            let now_nanos = crate::ops::ic::now_nanos();

            if expires_at_nanos <= now_nanos {
                return Err(LedgerWorkflowError::AllowanceExpired {
                    symbol: meta.symbol,
                    expires_at_nanos,
                    now_nanos,
                }
                .into());
            }
        }

        Ok(())
    }
}
