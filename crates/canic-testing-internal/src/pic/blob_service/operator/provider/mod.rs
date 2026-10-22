//! Controlled Cashier responses qualify mounted transport, never deployed provider facts.

mod account;
mod gateways;
mod relationship;

use super::{
    LocalServiceStatus, LocalStatusFailure, OperatorScope, PocketIc, Principal,
    ServiceConfigurationInput, contracts, decode_blob, query,
};
use ic_testkit::pic::fixtures::{RawReplyLimits, RawReplyMethod, ReplyMode, raw_reply_canister};

pub(in crate::pic::blob_service) fn cashier() -> Principal {
    // A routable application-subnet ID, created only inside this isolated PocketIC.
    Principal::from_text("um5iw-rqaaa-aaaaq-qaaba-cai").unwrap()
}

fn update<T: serde::Serialize + serde::de::DeserializeOwned>(
    pic: &PocketIc,
    scope: OperatorScope,
    actor: Principal,
    method: &str,
    request: impl serde::Serialize,
) -> T {
    decode_blob(
        method,
        &pic.update_call(
            scope.service.into(),
            actor,
            method,
            contracts::wire::encode_request(method, request).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn status(pic: &PocketIc, scope: OperatorScope, operator: Principal) -> LocalServiceStatus {
    let result: Result<LocalServiceStatus, LocalStatusFailure> =
        query(pic, scope, operator, "blob_local_status", scope);
    result.unwrap()
}

fn replies(
    pic: &PocketIc,
    scope: OperatorScope,
    gateways: &[u8],
    balance: &[u8],
    relationship: &[u8],
    initial: bool,
) {
    let wasm = raw_reply_canister(
        &[
            RawReplyMethod {
                name: "storage_gateway_list_v1",
                mode: ReplyMode::Update,
                reply: gateways,
            },
            RawReplyMethod {
                name: "account_balance_get_v1",
                mode: ReplyMode::Update,
                reply: balance,
            },
            RawReplyMethod {
                name: "payment_account_canister_get_v1",
                mode: ReplyMode::Update,
                reply: relationship,
            },
        ],
        RawReplyLimits {
            methods: 3,
            reply_bytes: 70_000,
            method_name_bytes: 128,
        },
    )
    .unwrap();
    if initial {
        pic.create_canister_with_id(None, None, scope.cashier.into())
            .unwrap();
        pic.install_canister(scope.cashier.into(), wasm, vec![], None);
    } else {
        pic.upgrade_canister(scope.cashier.into(), wasm, vec![], None)
            .unwrap();
    }
}

pub(in crate::pic::blob_service) fn qualify(
    pic: &PocketIc,
    config: &ServiceConfigurationInput,
    resume: impl Fn(),
    restore: impl Fn(),
) {
    let scope = OperatorScope {
        service: config.service,
        namespace: config.namespace,
        cashier: config.billing.cashier,
        payment_account: config.payment_account,
    };
    let operator = config.operator.into();
    let gateway = Principal::from_slice(&[42]);
    let members = ic_platform::candid::contract::encode_one(vec![gateway]).unwrap();
    let balance = ic_platform::candid_parser::parse_idl_args(&format!(
        "(variant {{ Ok = record {{ account = principal \"{}\"; account_cycle_balances = record {{ total = 600 : int; cycles_prepaid = 100 : int; cycles_promo = 200 : int; cycles_ledger = 300 : int; debt_target = variant {{ Prepaid }} }} }} }})", scope.payment_account
    )).unwrap().to_bytes().unwrap();
    replies(pic, scope, &members, &balance, &[0], true);
    resume();
    account::check(pic, scope, operator);
    gateways::check(pic, scope, operator, gateway);
    replies(pic, scope, &[0], &[0], &[0], false);
    resume();
    account::malformed(pic, scope, operator);
    gateways::failed_sync(
        pic,
        scope,
        operator,
        contracts::dto::gateway::sync::GatewaySyncFailure::InvalidReply,
    );
    replies(pic, scope, &vec![0; 65_537], &vec![0; 4_097], &[0], false);
    account::oversized(pic, scope, operator);
    gateways::failed_sync(
        pic,
        scope,
        operator,
        contracts::dto::gateway::sync::GatewaySyncFailure::ReplyTooLarge,
    );
    restore();
    account::fenced(pic, scope, operator);
    gateways::fenced(pic, scope, operator, gateway);
    resume();
    replies(pic, scope, &members, &balance, &[0], false);
    account::check(pic, scope, operator);
    gateways::check(pic, scope, operator, gateway);
    gateways::lost_reply(pic, scope, operator, gateway);
    relationship::check(pic, scope, operator);
}
