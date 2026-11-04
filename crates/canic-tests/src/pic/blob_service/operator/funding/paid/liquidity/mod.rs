//! Maximal journal offers refuse when complete operating holds leave no call budget.

use super::{
    LocalServiceStatus, LocalStatusFailure, OperatorScope, PocketIc, ServiceConfigurationInput,
    query,
};

pub(super) fn check(pic: &PocketIc, config: &ServiceConfigurationInput, restore: impl Fn()) {
    let scope = OperatorScope {
        service: config.service,
        cashier: config.billing.cashier,
        payment_account: config.payment_account,
        namespace: config.namespace,
    };
    pic.update_call(
        scope.service.into(),
        config.operator.into(),
        "test_funding_liquidity",
        vec![],
    )
    .unwrap();
    let status = || {
        let result: Result<LocalServiceStatus, LocalStatusFailure> = query(
            pic,
            scope,
            config.operator.into(),
            "blob_local_status",
            scope,
        );
        result.unwrap().funding
    };
    let refused = status();
    assert_eq!(refused.available_allocation, 10_000);
    assert_eq!(refused.attachment_allowance, 9_900);
    assert_eq!(refused.not_enqueued, 29_700);
    assert_eq!(refused.transport_accepted, 0);
    assert_eq!(refused.uncredited_accepted, 0);
    assert_eq!(refused.refunded, 200);
    assert_eq!(refused.reserved_or_uncertain, 0);
    assert_eq!(refused.retained_intents, 5);
    restore();
    pic.update_call(
        scope.service.into(),
        config.operator.into(),
        "test_funding_liquidity_replay",
        vec![],
    )
    .unwrap();
    assert_eq!(status(), refused);
}
