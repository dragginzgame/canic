//! Relationship reports preserve provider data without acquiring funding authority.

use super::{OperatorScope, PocketIc, Principal, contracts, replies, status, update};
use contracts::dto::account::{
    AccountInspectionFailure, AccountInspectionKind, AccountInspectionRequest,
    AccountInspectionResponse, AccountObservation, AccountRelationshipView,
};

fn inspect(
    pic: &PocketIc,
    scope: OperatorScope,
    actor: Principal,
    requested: OperatorScope,
) -> Result<AccountInspectionResponse, AccountInspectionFailure> {
    update(
        pic,
        scope,
        actor,
        "blob_inspect_account",
        AccountInspectionRequest {
            scope: requested,
            kind: AccountInspectionKind::PaymentRelationship,
        },
    )
}

fn encoded(text: &str) -> Vec<u8> {
    ic_platform::candid_parser::parse_idl_args(text)
        .unwrap()
        .to_bytes()
        .unwrap()
}

fn present(scope: OperatorScope) -> Vec<u8> {
    encoded(&format!(
        r#"(variant {{ Ok = record {{ relationship = opt record {{
        paid_canister = principal "{}"; payment_account = principal "{}";
        spending_limit_per_day = -1 : int; current_period_spent = -2 : int;
        current_period_start = 3 : nat64; added_timestamp = 4 : nat64;
        expiration_timestamp = null;
        bandwidth_baseline_uploaded = 340282366920938463463374607431768211455 : nat;
        bandwidth_baseline_downloaded = 6 : nat; bandwidth_baseline_ts_ns = 7 : nat64;
    }} }} }})"#,
        scope.service, scope.payment_account
    ))
}

fn observe(
    pic: &PocketIc,
    scope: OperatorScope,
    operator: Principal,
    bytes: &[u8],
    expected: Result<AccountObservation, AccountInspectionFailure>,
) {
    replies(pic, scope, &[0], &[0], bytes, false);
    let before = status(pic, scope, operator);
    let result = inspect(pic, scope, operator, scope).map(|response| {
        assert_eq!(
            response.request,
            AccountInspectionRequest {
                scope,
                kind: AccountInspectionKind::PaymentRelationship,
            }
        );
        response.observation
    });
    assert_eq!(result, expected);
    assert_eq!(status(pic, scope, operator), before);
}

pub(super) fn check(pic: &PocketIc, scope: OperatorScope, operator: Principal) {
    observe(
        pic,
        scope,
        operator,
        &present(scope),
        Ok(AccountObservation::Relationship(Box::new(
            AccountRelationshipView {
                paid_canister: scope.service,
                payment_account: scope.payment_account,
                spending_limit_per_day: (-1).into(),
                current_period_spent: (-2).into(),
                current_period_start: 3,
                added_timestamp: 4,
                expiration_timestamp: None,
                bandwidth_baseline_uploaded: u128::MAX.into(),
                bandwidth_baseline_downloaded: 6_u64.into(),
                bandwidth_baseline_ts_ns: 7,
            },
        ))),
    );
    assert_eq!(
        inspect(pic, scope, scope.payment_account.into(), scope),
        Err(AccountInspectionFailure::Denied)
    );
    for wrong in super::super::wrong_scopes(scope) {
        assert_eq!(
            inspect(pic, scope, operator, wrong),
            Err(AccountInspectionFailure::Binding)
        );
    }
    for changed in [
        OperatorScope {
            service: Principal::from_slice(&[99]).into(),
            ..scope
        },
        OperatorScope {
            payment_account: Principal::from_slice(&[99]).into(),
            ..scope
        },
    ] {
        observe(
            pic,
            scope,
            operator,
            &present(changed),
            Err(AccountInspectionFailure::Binding),
        );
    }
    provider_outcomes(pic, scope, operator);
    observe(
        pic,
        scope,
        operator,
        &[0],
        Err(AccountInspectionFailure::InvalidReply),
    );
    observe(
        pic,
        scope,
        operator,
        &vec![0; 4097],
        Err(AccountInspectionFailure::ReplyTooLarge),
    );
}

fn provider_outcomes(pic: &PocketIc, scope: OperatorScope, operator: Principal) {
    let diagnostic = Principal::from_slice(&[98]);
    for (body, expected) in [
        (
            "Ok = record { relationship = null }".to_owned(),
            AccountObservation::NoRelationshipReported,
        ),
        (
            format!("Err = variant {{ RelationshipNotFound = principal \"{diagnostic}\" }}"),
            AccountObservation::RelationshipNotFound(diagnostic.into()),
        ),
        (
            format!("Err = variant {{ NotAuthorized = principal \"{diagnostic}\" }}"),
            AccountObservation::NotAuthorized(diagnostic.into()),
        ),
        (
            "Err = variant { InvalidRequest = \"provider diagnostic\" }".to_owned(),
            AccountObservation::InvalidRequest,
        ),
        (
            "Err = variant { InternalError = \"provider diagnostic\" }".to_owned(),
            AccountObservation::ProviderInternalError,
        ),
    ] {
        observe(
            pic,
            scope,
            operator,
            &encoded(&format!("(variant {{ {body} }})")),
            Ok(expected),
        );
    }
}
