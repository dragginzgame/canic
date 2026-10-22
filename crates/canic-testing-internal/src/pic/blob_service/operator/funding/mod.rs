//! Empty funding history and passive assessment must not manufacture payment evidence.

use super::{PocketIc, Principal, contracts, query, wrong_scopes};
use contracts::dto::{
    funding::{
        FundingHistoryCursor, FundingHistoryFailure, FundingHistoryPage, FundingHistoryRequest,
        assessment::{
            FundingPreparationBlocker, FundingPreparationFailure, FundingPreparationRequest,
            FundingPreparationResponse,
        },
        outcome::{FundingOutcomeFailure, FundingOutcomeRequest, FundingOutcomeResponse},
    },
    operator::{LocalFundingStatus, OperatorScope},
};

pub(super) fn check(
    pic: &PocketIc,
    operator: Principal,
    scope: OperatorScope,
    journal: LocalFundingStatus,
) {
    history(pic, operator, scope, journal.fenced);
    outcome(pic, operator, scope);
    assessment(pic, operator, scope, journal);
}

fn history(pic: &PocketIc, operator: Principal, scope: OperatorScope, fenced: bool) {
    let request = FundingHistoryRequest {
        scope,
        cursor: None,
    };
    let inspect = |actor, request| -> Result<FundingHistoryPage, FundingHistoryFailure> {
        query(pic, scope, actor, "blob_funding_history", request)
    };
    let page = inspect(operator, request).unwrap();
    assert_eq!(page.request, request);
    assert!(page.entries.is_empty());
    assert_eq!(page.next, None);
    assert_eq!(page.fenced, fenced);
    assert_eq!(
        inspect(scope.payment_account.into(), request),
        Err(FundingHistoryFailure::Denied)
    );
    for changed in wrong_scopes(scope) {
        assert_eq!(
            inspect(
                operator,
                FundingHistoryRequest {
                    scope: changed,
                    ..request
                }
            ),
            Err(FundingHistoryFailure::Binding)
        );
    }
    let cursor = FundingHistoryCursor {
        scope,
        before_operation: 1,
    };
    let continuation = FundingHistoryRequest {
        cursor: Some(cursor),
        ..request
    };
    assert!(inspect(operator, continuation).unwrap().entries.is_empty());
    let invalid = FundingHistoryRequest {
        cursor: Some(FundingHistoryCursor {
            before_operation: 0,
            ..cursor
        }),
        ..request
    };
    assert_eq!(
        inspect(operator, invalid),
        Err(FundingHistoryFailure::Invalid)
    );
    let foreign = FundingHistoryRequest {
        cursor: Some(FundingHistoryCursor {
            scope: wrong_scopes(scope)[1],
            ..cursor
        }),
        ..request
    };
    assert_eq!(
        inspect(operator, foreign),
        Err(FundingHistoryFailure::CursorScope)
    );
}

fn outcome(pic: &PocketIc, operator: Principal, scope: OperatorScope) {
    let request = FundingOutcomeRequest {
        scope,
        operation: 1,
        offered: 1,
        target_balance: None,
    };
    let inspect =
        |actor, request| -> Result<Option<FundingOutcomeResponse>, FundingOutcomeFailure> {
            query(pic, scope, actor, "blob_funding_outcome", request)
        };
    assert_eq!(inspect(operator, request), Ok(None));
    assert_eq!(
        inspect(scope.payment_account.into(), request),
        Err(FundingOutcomeFailure::Denied)
    );
    for changed in wrong_scopes(scope) {
        assert_eq!(
            inspect(
                operator,
                FundingOutcomeRequest {
                    scope: changed,
                    ..request
                }
            ),
            Err(FundingOutcomeFailure::Binding)
        );
    }
    for invalid in [
        FundingOutcomeRequest {
            operation: 0,
            ..request
        },
        FundingOutcomeRequest {
            offered: 0,
            ..request
        },
        FundingOutcomeRequest {
            target_balance: Some(0),
            ..request
        },
    ] {
        assert_eq!(
            inspect(operator, invalid),
            Err(FundingOutcomeFailure::Invalid)
        );
    }
}

fn assessment(
    pic: &PocketIc,
    operator: Principal,
    scope: OperatorScope,
    journal: LocalFundingStatus,
) {
    let request = FundingPreparationRequest {
        scope,
        operation: 1,
        offered: 1,
        target_balance: Some(200),
    };
    let inspect =
        |actor, request| -> Result<FundingPreparationResponse, FundingPreparationFailure> {
            query(
                pic,
                scope,
                actor,
                "blob_funding_preparation_assessment",
                request,
            )
        };
    let response = inspect(operator, request).unwrap();
    assert_eq!(response.request, request);
    assert_eq!(response.journal, journal);
    for blocker in [
        FundingPreparationBlocker::ProviderUnqualified,
        FundingPreparationBlocker::RecoveryUnknown,
        FundingPreparationBlocker::FundingUnknown,
        FundingPreparationBlocker::SpendabilityUnknown,
    ] {
        assert!(response.blockers.contains(&blocker));
    }
    assert_eq!(
        response
            .blockers
            .contains(&FundingPreparationBlocker::JournalFenced),
        journal.fenced
    );
    assert_eq!(
        inspect(scope.payment_account.into(), request),
        Err(FundingPreparationFailure::Denied)
    );
    for changed in wrong_scopes(scope) {
        assert_eq!(
            inspect(
                operator,
                FundingPreparationRequest {
                    scope: changed,
                    ..request
                }
            ),
            Err(FundingPreparationFailure::Binding)
        );
    }
    assert_eq!(
        inspect(
            operator,
            FundingPreparationRequest {
                offered: 0,
                ..request
            }
        ),
        Err(FundingPreparationFailure::Invalid)
    );
    let excessive = FundingPreparationRequest {
        offered: journal.available_allocation + 1,
        ..request
    };
    let response = inspect(operator, excessive).unwrap();
    assert_eq!(response.request, excessive);
    assert_eq!(response.journal, journal);
    assert!(
        response
            .blockers
            .contains(&FundingPreparationBlocker::AllocationReserve {
                transferable_cycles: journal.attachment_allowance
            })
    );
}
