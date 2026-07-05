//! Module: ops::ic::ledger
//!
//! Responsibility: provide observable ICRC ledger calls through approved ops APIs.
//! Does not own: allowance policy, funding workflow, or ledger DTO definitions.
//! Boundary: records ledger metrics and delegates ledger call mechanics to infra.

use crate::{
    InternalError,
    cdk::spec::standards::icrc::icrc2::{Allowance, TransferFromArgs, TransferFromResult},
    domain::metrics::{
        PlatformCallMetricMode, PlatformCallMetricOutcome, PlatformCallMetricReason,
        PlatformCallMetricSurface,
    },
    infra::{InfraError, ic::ledger::LedgerInfra},
    ops::{ic::IcOpsError, prelude::*, runtime::metrics::platform_call::PlatformCallMetrics},
};
use thiserror::Error as ThisError;

///
/// LedgerOpsError
///
/// Typed failure surface for ledger operations.
///

#[derive(Debug, ThisError)]
pub enum LedgerOpsError {
    /// Any infra failure (platform call failed, candid errors, ledger rejection mapped in infra, etc.)
    #[error(transparent)]
    Infra(#[from] InfraError),
}

impl From<LedgerOpsError> for InternalError {
    fn from(err: LedgerOpsError) -> Self {
        IcOpsError::from(err).into()
    }
}

///
/// LedgerMeta
///
/// Best-effort ledger metadata returned through the ops facade.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LedgerMeta {
    pub symbol: &'static str,
    pub decimals: u8,
    pub is_known: bool,
}

///
/// LedgerOps
///
/// Operations-layer facade for observable ICRC ledger calls.
///

pub struct LedgerOps;

impl LedgerOps {
    /// Best-effort metadata for a ledger canister.
    #[must_use]
    pub fn ledger_meta(ledger_id: Principal) -> LedgerMeta {
        let meta = LedgerInfra::ledger_meta(ledger_id);
        LedgerMeta {
            symbol: meta.symbol,
            decimals: meta.decimals,
            is_known: meta.is_known,
        }
    }

    /// Query ICRC-2 allowance (raw ledger response).
    pub async fn allowance(
        ledger_id: Principal,
        payer: Account,
        spender: Account,
    ) -> Result<Allowance, InternalError> {
        record_ledger_call(
            PlatformCallMetricOutcome::Started,
            PlatformCallMetricReason::Ok,
        );
        let allowance = match LedgerInfra::icrc2_allowance(ledger_id, payer, spender).await {
            Ok(allowance) => allowance,
            Err(err) => {
                record_ledger_call(
                    PlatformCallMetricOutcome::Failed,
                    PlatformCallMetricReason::Infra,
                );
                return Err(LedgerOpsError::from(err).into());
            }
        };
        record_ledger_call(
            PlatformCallMetricOutcome::Completed,
            PlatformCallMetricReason::Ok,
        );

        Ok(allowance)
    }

    /// Execute an ICRC-2 transfer_from and return the block index on success.
    pub async fn transfer_from(
        ledger_id: Principal,
        from: Principal,
        to: Account,
        amount: u64,
        memo: Option<Vec<u8>>,
    ) -> Result<u64, InternalError> {
        // Note: created_at_time is set at the call site here because ops owns
        // execution conventions; infra owns mechanics.
        let from_account = Account {
            owner: from,
            subaccount: None,
        };

        let args = TransferFromArgs {
            from: from_account,
            to,
            amount,
            memo,
            created_at_time: Some(crate::cdk::api::time()),
        };

        record_ledger_call(
            PlatformCallMetricOutcome::Started,
            PlatformCallMetricReason::Ok,
        );
        let result: TransferFromResult =
            match LedgerInfra::icrc2_transfer_from(ledger_id, args).await {
                Ok(result) => result,
                Err(err) => {
                    record_ledger_call(
                        PlatformCallMetricOutcome::Failed,
                        PlatformCallMetricReason::Infra,
                    );
                    return Err(LedgerOpsError::from(err).into());
                }
            };

        match result {
            TransferFromResult::Ok(block_index) => {
                record_ledger_call(
                    PlatformCallMetricOutcome::Completed,
                    PlatformCallMetricReason::Ok,
                );
                Ok(block_index)
            }

            // By construction, infra::ic::ledger::icrc2_transfer_from already maps Err(...)
            // into InfraError (lossless), so this branch should be unreachable. Keep it anyway
            // to be robust to future infra changes.
            TransferFromResult::Err(_) => {
                record_ledger_call(
                    PlatformCallMetricOutcome::Failed,
                    PlatformCallMetricReason::LedgerRejected,
                );
                unreachable!()
                /*
                Err(LedgerOpsError::Infra(InfraError::from(
                LedgerInfraError::TransferFromRejected {
                    symbol: LedgerInfra::ledger_meta(ledger_id).symbol,
                    // We can't recover the error here without matching; infra should not return Err(...)
                    // if it wants ops to handle it. Prefer keeping infra mapping.
                    error: unreachable!(
                        "infra::ic::ledger maps TransferFromResult::Err into InfraError"
                    ),
                },*/
            }
        }
    }
}

// Record one ledger-call metric with no ledger, account, or token labels.
fn record_ledger_call(outcome: PlatformCallMetricOutcome, reason: PlatformCallMetricReason) {
    PlatformCallMetrics::record(
        PlatformCallMetricSurface::Ledger,
        PlatformCallMetricMode::Update,
        outcome,
        reason,
    );
}
