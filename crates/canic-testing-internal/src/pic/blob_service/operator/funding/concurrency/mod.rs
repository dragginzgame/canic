//! Concurrent ingress cannot duplicate a held funding attempt or spend another offer.

use super::super::{LocalServiceStatus, LocalStatusFailure, PocketIc, query};
use super::{OperatorScope, contracts};
use contracts::dto::configuration::ServiceConfigurationInput;
use ic_testkit::pic::fixtures::{
    RawReplyLimits, RawReplyMethod, ReplyMode, held::HeldReplyFixture,
};

pub(in crate::pic::blob_service) fn check(pic: &PocketIc, config: &ServiceConfigurationInput) {
    let service = config.service.into();
    let operator = config.operator.into();
    let cashier = config.billing.cashier.into();
    let scope = OperatorScope {
        service: config.service,
        cashier: config.billing.cashier,
        payment_account: config.payment_account,
        namespace: config.namespace,
    };
    pic.add_cycles(cashier, 1_000_000_000_000);
    let mut fixture = HeldReplyFixture::new(
        pic,
        cashier,
        &[RawReplyMethod {
            name: "account_top_up_v1",
            mode: ReplyMode::Update,
            reply: b"malformed",
        }],
        RawReplyLimits {
            methods: 1,
            reply_bytes: 32,
            method_name_bytes: 64,
        },
        50_000_000_000,
        100_000,
    )
    .unwrap();
    pic.upgrade_canister(cashier, fixture.wasm().to_vec(), vec![], None)
        .unwrap();
    // Leave the original ingress reply uncollected: mounted observations recover it.
    let _reply = pic
        .submit_call(service, operator, "test_funding", vec![])
        .unwrap();
    let mut held = None;
    for _ in 0..30 {
        pic.tick();
        if let Some(request) = fixture.pending(pic).unwrap().into_iter().next() {
            held = Some(request);
            break;
        }
    }
    let held = held.expect("original funding call reaches provider barrier");
    let status = || {
        let result: Result<LocalServiceStatus, LocalStatusFailure> =
            query(pic, scope, operator, "blob_local_status", scope);
        result.unwrap().funding
    };
    let pending = status();
    assert_eq!(pending.last_operation, Some(1));
    assert_eq!(pending.retained_intents, 1);
    assert_eq!(pending.reserved_or_uncertain, 100);
    for _ in 0..2 {
        pic.update_call(service, operator, "test_funding_pending", vec![])
            .unwrap();
        assert_eq!(fixture.pending(pic).unwrap(), vec![held.clone()]);
        assert_eq!(status(), pending);
    }
    fixture.release(pic, &held).unwrap();
    for _ in 0..100 {
        pic.tick();
        if super::super::refunded(pic, config, false) {
            break;
        }
    }
    assert!(super::super::refunded(pic, config, false));
    assert!(fixture.pending(pic).unwrap().is_empty());
}
