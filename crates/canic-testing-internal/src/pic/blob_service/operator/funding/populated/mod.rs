//! Inspect original refunded operations before and after same-release restoration.

use super::{
    FundingHistoryCursor, FundingHistoryFailure, FundingHistoryPage, FundingHistoryRequest,
    FundingOutcomeFailure, FundingOutcomeRequest, FundingOutcomeResponse, OperatorScope, PocketIc,
    Principal, contracts, query,
};
use contracts::dto::funding::{
    FundingHistoryEntry, FundingPhase,
    outcome::{FundingReconciliation, FundingResponse},
};

pub(in crate::pic::blob_service::operator) fn check(
    pic: &PocketIc,
    operator: Principal,
    scope: OperatorScope,
    fenced: bool,
) {
    let stable = pic.get_stable_memory(scope.service.into());
    let entries = history(pic, operator, scope, fenced);
    for entry in entries {
        outcome(pic, operator, entry, fenced);
    }
    assert_eq!(pic.get_stable_memory(scope.service.into()), stable);
}

fn history(
    pic: &PocketIc,
    operator: Principal,
    scope: OperatorScope,
    fenced: bool,
) -> Vec<FundingHistoryEntry> {
    let request = FundingHistoryRequest {
        scope,
        cursor: None,
    };
    let inspect = |actor, request| -> Result<FundingHistoryPage, FundingHistoryFailure> {
        query(pic, scope, actor, "blob_funding_history", request)
    };
    let page = inspect(operator, request).unwrap();
    assert_eq!(page.request, request);
    assert_eq!(page.fenced, fenced);
    assert_eq!(page.next, None);
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.operation)
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
    for entry in &page.entries {
        assert_eq!(entry.scope, scope);
        assert_eq!(entry.offered, 100);
        assert_eq!(entry.target_balance, None);
        assert_eq!(entry.phase, FundingPhase::Callback { refunded: 100 });
    }
    for (before_operation, expected) in [(2, &page.entries[1..]), (1, &page.entries[2..])] {
        let request = FundingHistoryRequest {
            scope,
            cursor: Some(FundingHistoryCursor {
                scope,
                before_operation,
            }),
        };
        let continuation = inspect(operator, request).unwrap();
        assert_eq!(continuation.request, request);
        assert_eq!(continuation.entries, expected);
        assert_eq!(continuation.next, None);
        assert_eq!(continuation.fenced, fenced);
    }
    assert_eq!(
        inspect(scope.payment_account.into(), request),
        Err(FundingHistoryFailure::Denied)
    );
    page.entries
}

fn outcome(pic: &PocketIc, operator: Principal, entry: FundingHistoryEntry, fenced: bool) {
    let request = FundingOutcomeRequest {
        scope: entry.scope,
        operation: entry.operation,
        offered: entry.offered,
        target_balance: entry.target_balance,
    };
    let inspect =
        |actor, request| -> Result<Option<FundingOutcomeResponse>, FundingOutcomeFailure> {
            query(pic, entry.scope, actor, "blob_funding_outcome", request)
        };
    let response = inspect(operator, request).unwrap().unwrap();
    assert_eq!(response.request, request);
    assert_eq!(response.phase, entry.phase);
    if entry.operation == 1 {
        assert_eq!(response.response, Some(FundingResponse::InvalidReply));
    } else {
        assert!(matches!(
            response.response,
            Some(FundingResponse::Rejected(_))
        ));
    }
    assert_eq!(response.reconciliation, FundingReconciliation::NoTransfer);
    assert_eq!(response.renewed_allocation, 0);
    assert_eq!(response.fenced, fenced);
    assert_eq!(inspect(operator, request), Ok(Some(response)));
    for changed in [
        FundingOutcomeRequest {
            offered: request.offered + 1,
            ..request
        },
        FundingOutcomeRequest {
            target_balance: Some(200),
            ..request
        },
    ] {
        assert_eq!(
            inspect(operator, changed),
            Err(FundingOutcomeFailure::Conflict)
        );
    }
    assert_eq!(
        inspect(entry.scope.payment_account.into(), request),
        Err(FundingOutcomeFailure::Denied)
    );
    assert_eq!(
        inspect(
            operator,
            FundingOutcomeRequest {
                operation: 3,
                ..request
            }
        ),
        Ok(None)
    );
}
