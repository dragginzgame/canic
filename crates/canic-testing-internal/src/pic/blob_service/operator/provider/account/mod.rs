//! Replicated balance reports remain distinct from credit and decoding failures.

use super::{OperatorScope, PocketIc, Principal, contracts, status, update};
use contracts::dto::{
    account::{
        AccountInspectionFailure, AccountInspectionKind, AccountInspectionRequest,
        AccountInspectionResponse, AccountObservation,
    },
    funding::outcome::FundingReportedBalance,
};

fn inspect(
    pic: &PocketIc,
    scope: OperatorScope,
    actor: Principal,
    request: OperatorScope,
) -> Result<AccountInspectionResponse, AccountInspectionFailure> {
    update(
        pic,
        scope,
        actor,
        "blob_inspect_account",
        AccountInspectionRequest {
            scope: request,
            kind: AccountInspectionKind::Balance,
        },
    )
}

pub(super) fn check(pic: &PocketIc, scope: OperatorScope, operator: Principal) {
    let funding = status(pic, scope, operator).funding;
    let response = inspect(pic, scope, operator, scope).unwrap();
    assert_eq!(
        response.request,
        AccountInspectionRequest {
            scope,
            kind: AccountInspectionKind::Balance
        }
    );
    assert_eq!(
        response.observation,
        AccountObservation::Balance(FundingReportedBalance {
            total: 600,
            prepaid: 100,
            promotional: 200,
            ledger: 300,
        })
    );
    assert_eq!(
        inspect(pic, scope, scope.payment_account.into(), scope),
        Err(AccountInspectionFailure::Denied)
    );
    for changed in super::super::wrong_scopes(scope) {
        assert_eq!(
            inspect(pic, scope, operator, changed),
            Err(AccountInspectionFailure::Binding)
        );
    }
    assert_eq!(status(pic, scope, operator).funding, funding);
}

pub(super) fn malformed(pic: &PocketIc, scope: OperatorScope, operator: Principal) {
    assert_eq!(
        inspect(pic, scope, operator, scope),
        Err(AccountInspectionFailure::InvalidReply)
    );
}

pub(super) fn fenced(pic: &PocketIc, scope: OperatorScope, operator: Principal) {
    assert_eq!(
        inspect(pic, scope, operator, scope),
        Err(AccountInspectionFailure::Fenced)
    );
}

pub(super) fn oversized(pic: &PocketIc, scope: OperatorScope, operator: Principal) {
    let before = status(pic, scope, operator);
    assert_eq!(
        inspect(pic, scope, operator, scope),
        Err(AccountInspectionFailure::ReplyTooLarge)
    );
    assert_eq!(status(pic, scope, operator), before);
}
