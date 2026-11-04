//! Installed budget and lifetime limits remain unchanged by refusal or recovery.

use super::{LocalServiceStatus, LocalStatusFailure, OperatorScope, PocketIc, query};
use ic_platform::principal::ExternalPrincipal as Principal;

pub(super) fn check(pic: &PocketIc, scope: OperatorScope, operator: Principal, restore: impl Fn()) {
    pic.update_call(
        scope.service.into(),
        operator,
        "test_funding_budget",
        vec![],
    )
    .unwrap();
    let status = || {
        let result: Result<LocalServiceStatus, LocalStatusFailure> =
            query(pic, scope, operator, "blob_local_status", scope);
        result.unwrap().funding
    };
    let full = status();
    assert_eq!(full.cumulative_allocation, 10_150);
    assert_eq!(full.renewal_ceiling, 10_150);
    assert_eq!(full.available_allocation, 9_850);
    assert_eq!(full.transport_accepted, 300);
    assert_eq!(full.provider_acknowledged, 300);
    assert_eq!(full.uncredited_accepted, 300);
    assert_eq!(full.refunded, 300);
    assert_eq!(full.retained_intents, full.intent_capacity);
    assert_eq!(full.last_operation, Some(6));
    pic.update_call(
        scope.service.into(),
        operator,
        "test_funding_budget_replay",
        vec![],
    )
    .unwrap();
    assert_eq!(status(), full);
    super::outcome::acknowledged(pic, scope, operator, &[5, 6], 0);
    restore();
    pic.update_call(
        scope.service.into(),
        operator,
        "test_funding_budget_replay",
        vec![],
    )
    .unwrap();
    assert_eq!(status(), full);
    super::outcome::acknowledged(pic, scope, operator, &[5, 6], 0);
}
