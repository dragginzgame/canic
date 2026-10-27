//! Paid callbacks use upstream cycle acceptance; acknowledgments are not credit evidence.

mod budget;
mod held;
mod liquidity;
mod outcome;

use super::super::{LocalServiceStatus, LocalStatusFailure, PocketIc, query};
use super::{OperatorScope, contracts};
use contracts::dto::configuration::ServiceConfigurationInput;
use ic_testkit::pic::fixtures::{RawReplyLimits, RawReplyMethod, ReplyMode, cycle_reply_canister};

/// Independent installed failure scenarios; none clears another scenario's uncertainty.
#[derive(Clone, Copy)]
pub(in crate::pic::blob_service) enum FundingReply {
    Malformed,
    ProviderError,
    CallbackTrap,
    Liquidity,
    Budget,
}

pub(in crate::pic::blob_service) fn check(
    pic: &PocketIc,
    config: &ServiceConfigurationInput,
    reply: FundingReply,
    restore: impl Fn(),
) {
    if matches!(reply, FundingReply::Liquidity) {
        liquidity::check(pic, config, restore);
        return;
    }
    let scope = OperatorScope {
        service: config.service,
        cashier: config.billing.cashier,
        payment_account: config.payment_account,
        namespace: config.namespace,
    };
    install_provider(pic, scope, matches!(reply, FundingReply::Budget));
    pic.update_call(
        scope.service.into(),
        config.operator.into(),
        "test_funding_paid",
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
    let paid = status();
    assert_eq!(paid.transport_accepted, 150);
    assert_eq!(paid.provider_acknowledged, 150);
    assert_eq!(paid.refunded, 250);
    assert_eq!(paid.uncredited_accepted, 150);
    assert_eq!(paid.reserved_or_uncertain, 0);
    assert_eq!(paid.cumulative_allocation, 10_150);
    assert_eq!(paid.available_allocation, 10_000);
    assert_eq!(paid.retained_intents, 4);
    pic.update_call(
        scope.service.into(),
        config.operator.into(),
        "test_funding_paid_replay",
        vec![],
    )
    .unwrap();
    assert_eq!(status(), paid);
    outcome::acknowledged(pic, scope, config.operator.into(), &[3, 4], 75);
    restore();
    pic.update_call(
        scope.service.into(),
        config.operator.into(),
        "test_funding_paid_replay",
        vec![],
    )
    .unwrap();
    assert_eq!(status(), paid);
    outcome::acknowledged(pic, scope, config.operator.into(), &[3, 4], 75);
    if matches!(reply, FundingReply::Budget) {
        budget::check(pic, scope, config.operator.into(), restore);
        return;
    }
    let trapped = matches!(reply, FundingReply::CallbackTrap);
    match reply {
        FundingReply::ProviderError | FundingReply::CallbackTrap => {
            held::check(pic, scope, config.operator.into(), trapped);
        }
        FundingReply::Liquidity | FundingReply::Budget => {
            unreachable!("handled before paid scenarios")
        }
        FundingReply::Malformed => {
            pic.update_call(
                scope.service.into(),
                config.operator.into(),
                "test_funding_uncertain",
                vec![],
            )
            .unwrap();
        }
    }
    let uncertain = status();
    check_uncertain(uncertain, trapped);
    outcome::unresolved(pic, scope, config.operator.into(), reply, trapped);
    restore();
    pic.update_call(
        scope.service.into(),
        config.operator.into(),
        if trapped {
            "test_funding_uncertain_pending"
        } else {
            "test_funding_uncertain_replay"
        },
        vec![],
    )
    .unwrap();
    let mut recovered = uncertain;
    recovered.fenced = false;
    assert_eq!(status(), recovered);
    outcome::unresolved(pic, scope, config.operator.into(), reply, false);
}

fn install_provider(pic: &PocketIc, scope: OperatorScope, budget: bool) {
    let reply = ic_platform::candid_parser::parse_idl_args(
        "(variant { Ok = record { balance = record { total = 100 : int; cycles_prepaid = 80 : int; cycles_promo = 10 : int; cycles_ledger = 10 : int; debt_target = variant { Prepaid } }; message = \"accepted\" } })",
    ).unwrap().to_bytes().unwrap();
    let method = RawReplyMethod {
        name: "account_top_up_v1",
        mode: ReplyMode::Update,
        reply: &reply,
    };
    let methods = if budget {
        vec![method; 4]
    } else {
        vec![
            method,
            method,
            RawReplyMethod {
                reply: b"malformed",
                ..method
            },
        ]
    };
    let wasm = cycle_reply_canister(
        &methods,
        RawReplyLimits {
            methods: 4,
            reply_bytes: 4096,
            method_name_bytes: 4 * "account_top_up_v1".len(),
        },
        75,
        100_000,
    )
    .unwrap();
    pic.upgrade_canister(scope.cashier.into(), wasm, vec![], None)
        .unwrap();
}

fn check_uncertain(uncertain: contracts::dto::operator::LocalFundingStatus, trapped: bool) {
    assert_eq!(
        uncertain.transport_accepted,
        if trapped { 150 } else { 225 }
    );
    assert_eq!(uncertain.provider_acknowledged, 150);
    assert_eq!(
        uncertain.uncredited_accepted,
        if trapped { 150 } else { 225 }
    );
    assert_eq!(uncertain.refunded, if trapped { 250 } else { 275 });
    assert_eq!(
        uncertain.available_allocation,
        if trapped { 9_900 } else { 9_925 }
    );
    assert_eq!(
        uncertain.reserved_or_uncertain,
        if trapped { 100 } else { 0 }
    );
    assert_eq!(uncertain.retained_intents, 5);
    assert_eq!(uncertain.fenced, trapped);
}
