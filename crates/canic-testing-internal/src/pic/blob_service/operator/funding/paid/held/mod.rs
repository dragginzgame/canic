//! Held accepted errors preserve original intent while an ingress reply is uncollected.

use super::{LocalServiceStatus, LocalStatusFailure, OperatorScope, PocketIc, query};
use ic_platform::principal::ExternalPrincipal as Principal;
use ic_testkit::pic::fixtures::{
    RawReplyLimits, RawReplyMethod, ReplyMode, held::HeldReplyFixture,
};

pub(super) fn check(
    pic: &PocketIc,
    scope: OperatorScope,
    operator: Principal,
    trap_callback: bool,
) {
    let reply = ic_platform::candid_parser::parse_idl_args(
        "(variant { Err = variant { AccountBalanceOverflow } })",
    )
    .unwrap()
    .to_bytes()
    .unwrap();
    let mut fixture = HeldReplyFixture::new_with_cycle_acceptance(
        pic,
        scope.cashier.into(),
        &[RawReplyMethod {
            name: "account_top_up_v1",
            mode: ReplyMode::Update,
            reply: &reply,
        }],
        RawReplyLimits {
            methods: 1,
            reply_bytes: 4096,
            method_name_bytes: 64,
        },
        50_000_000_000,
        75,
        100_000,
    )
    .unwrap();
    pic.add_cycles(scope.cashier.into(), 1_000_000_000_000);
    pic.upgrade_canister(scope.cashier.into(), fixture.wasm().to_vec(), vec![], None)
        .unwrap();
    let original = pic
        .submit_call(
            scope.service.into(),
            operator,
            if trap_callback {
                "test_funding_callback_trap"
            } else {
                "test_funding_provider_error"
            },
            vec![],
        )
        .unwrap();
    let held = (0..30)
        .find_map(|_| {
            pic.tick();
            fixture.pending(pic).unwrap().into_iter().next()
        })
        .expect("original provider request reaches barrier");
    let status = || {
        let result: Result<LocalServiceStatus, LocalStatusFailure> =
            query(pic, scope, operator, "blob_local_status", scope);
        result.unwrap().funding
    };
    let pending = status();
    assert_eq!(pending.retained_intents, 5);
    assert_eq!(pending.reserved_or_uncertain, 100);
    assert_eq!(pending.transport_accepted, 150);
    for _ in 0..2 {
        pic.update_call(
            scope.service.into(),
            operator,
            "test_funding_uncertain_pending",
            vec![],
        )
        .unwrap();
        assert_eq!(fixture.pending(pic).unwrap(), vec![held.clone()]);
        assert_eq!(status(), pending);
    }
    fixture.release(pic, &held).unwrap();
    if trap_callback {
        let error = pic.await_call(original).unwrap_err();
        assert_eq!(
            error.error_code,
            ic_platform::pocket_ic::ErrorCode::CanisterCalledTrap
        );
        let mut fenced = pending;
        fenced.fenced = true;
        assert_eq!(status(), fenced);
    } else {
        for _ in 0..100 {
            pic.tick();
            if status().transport_accepted == 225 {
                break;
            }
        }
        assert_eq!(status().transport_accepted, 225);
    }
    assert!(fixture.pending(pic).unwrap().is_empty());
}
