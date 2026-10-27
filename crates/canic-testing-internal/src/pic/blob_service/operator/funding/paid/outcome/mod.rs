//! Mounted paid outcomes distinguish acknowledgment, missing credit and unknown transfer.

use super::{FundingReply, OperatorScope, PocketIc, contracts, query};
use contracts::dto::funding::{
    FundingPhase,
    outcome::{
        FundingOutcomeFailure, FundingOutcomeRequest, FundingOutcomeResponse,
        FundingReconciliation, FundingReportedBalance, FundingResponse,
    },
};
use ic_platform::principal::ExternalPrincipal as Principal;

pub(super) fn acknowledged(
    pic: &PocketIc,
    scope: OperatorScope,
    operator: Principal,
    operations: &[u128],
    renewed: u128,
) {
    for &operation in operations {
        check(
            pic,
            operator,
            &FundingOutcomeResponse {
                request: request(scope, operation),
                phase: FundingPhase::Callback { refunded: 25 },
                response: Some(FundingResponse::ReportedSuccess(FundingReportedBalance {
                    total: 100,
                    prepaid: 80,
                    promotional: 10,
                    ledger: 10,
                })),
                reconciliation: FundingReconciliation::ProviderAcknowledged(75),
                renewed_allocation: renewed,
                fenced: false,
            },
        );
    }
}

pub(super) fn unresolved(
    pic: &PocketIc,
    scope: OperatorScope,
    operator: Principal,
    scenario: FundingReply,
    fenced: bool,
) {
    let (phase, response, reconciliation) = match scenario {
        FundingReply::Malformed => (
            FundingPhase::Callback { refunded: 25 },
            Some(FundingResponse::InvalidReply),
            FundingReconciliation::CreditRequired(75),
        ),
        FundingReply::ProviderError => (
            FundingPhase::Callback { refunded: 25 },
            Some(FundingResponse::AccountBalanceOverflow),
            FundingReconciliation::CreditRequired(75),
        ),
        FundingReply::CallbackTrap => (
            FundingPhase::Uncertain,
            None,
            FundingReconciliation::TransferUnknown(100),
        ),
        FundingReply::Liquidity | FundingReply::Budget => unreachable!("distinct profile"),
    };
    check(
        pic,
        operator,
        &FundingOutcomeResponse {
            request: request(scope, 5),
            phase,
            response,
            reconciliation,
            renewed_allocation: 0,
            fenced,
        },
    );
}

const fn request(scope: OperatorScope, operation: u128) -> FundingOutcomeRequest {
    FundingOutcomeRequest {
        scope,
        operation,
        offered: 100,
        target_balance: None,
    }
}

fn check(pic: &PocketIc, operator: Principal, expected: &FundingOutcomeResponse) {
    let original = expected.request;
    let stable = pic.get_stable_memory(original.scope.service.into());
    let inspect =
        |actor, request| -> Result<Option<FundingOutcomeResponse>, FundingOutcomeFailure> {
            query(pic, original.scope, actor, "blob_funding_outcome", request)
        };
    assert_eq!(inspect(operator, original), Ok(Some(*expected)));
    assert_eq!(inspect(operator, original), Ok(Some(*expected)));
    assert_eq!(
        inspect(Principal::anonymous(), original),
        Err(FundingOutcomeFailure::Denied)
    );
    assert_eq!(
        inspect(
            operator,
            FundingOutcomeRequest {
                offered: 101,
                ..original
            }
        ),
        Err(FundingOutcomeFailure::Conflict)
    );
    assert_eq!(pic.get_stable_memory(original.scope.service.into()), stable);
}
