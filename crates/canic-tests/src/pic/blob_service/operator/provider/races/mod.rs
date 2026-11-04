//! Hold replies across explicit refresh supersession, cancellation and revocation.

use super::{OperatorScope, PocketIc, Principal, contracts, decode_blob, status, update};
use contracts::dto::gateway::{
    GatewayRevocationFailure, GatewayRevocationRequest, GatewayRevocationResponse,
    sync::{GatewaySyncCancellation, GatewaySyncFailure, GatewaySyncResponse},
};
use ic_testkit::pic::fixtures::{
    RawReplyLimits, RawReplyMethod, ReplyMode,
    held::{HeldReplyFixture, HeldRequest},
};

pub(super) fn check(pic: &PocketIc, scope: OperatorScope, operator: Principal, gateway: Principal) {
    let replacement =
        ic_platform::candid::contract::encode_one(vec![Principal::from_slice(&[99])]).unwrap();
    let original = ic_platform::candid::contract::encode_one(vec![gateway]).unwrap();
    let methods = [&replacement, &original, &replacement, &original].map(|reply| RawReplyMethod {
        name: "storage_gateway_list_v1",
        mode: ReplyMode::Update,
        reply,
    });
    pic.add_cycles(scope.cashier.into(), 1_000_000_000_000)
        .expect("PocketIC add_cycles");
    let mut fixture = HeldReplyFixture::new(
        pic,
        scope.cashier.into(),
        &methods,
        RawReplyLimits {
            methods: 4,
            reply_bytes: 128,
            method_name_bytes: methods.iter().map(|method| method.name.len()).sum(),
        },
        50_000_000_000,
        100_000,
    )
    .unwrap();
    pic.upgrade_canister(scope.cashier.into(), fixture.wasm().to_vec(), vec![], None)
        .unwrap();
    superseded(pic, scope, operator, gateway, &mut fixture);
    for (slot, revoke) in [(2, false), (3, true)] {
        let before = status(pic, scope, operator).gateways;
        assert_eq!(before.members, vec![gateway.into()]);
        let call = pic
            .submit_call(
                scope.service.into(),
                operator,
                "blob_sync_gateways",
                contracts::wire::encode_request("blob_sync_gateways", scope).unwrap(),
            )
            .unwrap();
        let held = pending(pic, &fixture, slot);
        let pending = status(pic, scope, operator).gateways;
        let sequence = pending.pending_sequence.unwrap();
        assert_eq!(sequence, before.last_sequence + 1);
        invalidate(pic, scope, operator, gateway, sequence, revoke);
        let committed = status(pic, scope, operator).gateways;
        assert_eq!(committed.pending_sequence, None);
        assert_eq!(committed.last_sequence, sequence);
        assert_eq!(
            committed.members,
            if revoke { vec![] } else { before.members }
        );
        // The original provider response is still held after the invalidation;
        // no scheduler timing assumption substitutes for this observed barrier.
        assert_eq!(fixture.pending(pic).unwrap(), vec![held.clone()]);
        fixture.release(pic, &held).unwrap();
        let bytes = pic.await_call(call).unwrap();
        let result: Result<GatewaySyncResponse, GatewaySyncFailure> =
            decode_blob("blob_sync_gateways", &bytes).unwrap();
        assert_eq!(result, Err(GatewaySyncFailure::Conflict));
        assert_eq!(status(pic, scope, operator).gateways, committed);
        assert!(fixture.pending(pic).unwrap().is_empty());
    }
}

fn superseded(
    pic: &PocketIc,
    scope: OperatorScope,
    operator: Principal,
    gateway: Principal,
    fixture: &mut HeldReplyFixture,
) {
    let before = status(pic, scope, operator).gateways;
    let request = contracts::wire::encode_request("blob_sync_gateways", scope).unwrap();
    let old_call = pic
        .submit_call(
            scope.service.into(),
            operator,
            "blob_sync_gateways",
            request.clone(),
        )
        .unwrap();
    let old_reply = pending(pic, fixture, 0);
    let old_sequence = status(pic, scope, operator)
        .gateways
        .pending_sequence
        .unwrap();
    assert_eq!(old_sequence, before.last_sequence + 1);
    let new_call = pic
        .submit_call(
            scope.service.into(),
            operator,
            "blob_sync_gateways",
            request,
        )
        .unwrap();
    let new_reply = pending(pic, fixture, 1);
    let replacement = status(pic, scope, operator).gateways;
    assert_eq!(replacement.pending_sequence, Some(old_sequence + 1));
    assert_eq!(replacement.members, before.members);
    let stale_cancel: Result<(), GatewaySyncFailure> = update(
        pic,
        scope,
        operator,
        "blob_cancel_gateway_sync",
        GatewaySyncCancellation {
            scope,
            sequence: old_sequence,
        },
    );
    assert_eq!(stale_cancel, Err(GatewaySyncFailure::Conflict));
    assert_eq!(status(pic, scope, operator).gateways, replacement);
    let held = fixture.pending(pic).unwrap();
    assert_eq!(held.len(), 2);
    assert!(held.contains(&old_reply));
    assert!(held.contains(&new_reply));

    fixture.release(pic, &new_reply).unwrap();
    let bytes = pic.await_call(new_call).unwrap();
    let result: Result<GatewaySyncResponse, GatewaySyncFailure> =
        decode_blob("blob_sync_gateways", &bytes).unwrap();
    assert_eq!(result.unwrap().sequence, old_sequence + 1);
    let committed = status(pic, scope, operator).gateways;
    assert_eq!(committed.pending_sequence, None);
    assert_eq!(committed.members, vec![gateway.into()]);
    assert_eq!(committed.last_sequence, old_sequence + 1);
    assert_eq!(fixture.pending(pic).unwrap(), vec![old_reply.clone()]);
    fixture.release(pic, &old_reply).unwrap();
    let bytes = pic.await_call(old_call).unwrap();
    let result: Result<GatewaySyncResponse, GatewaySyncFailure> =
        decode_blob("blob_sync_gateways", &bytes).unwrap();
    assert_eq!(result, Err(GatewaySyncFailure::Conflict));
    assert_eq!(status(pic, scope, operator).gateways, committed);
    assert!(fixture.pending(pic).unwrap().is_empty());
}

fn pending(pic: &PocketIc, fixture: &HeldReplyFixture, slot: usize) -> HeldRequest {
    for _ in 0..30 {
        pic.tick().expect("PocketIC tick");
        let requests = fixture.pending(pic).unwrap();
        if let Some(request) = requests.into_iter().find(|request| request.slot() == slot) {
            return request;
        }
    }
    panic!("original provider call must reach the held-response barrier")
}

fn invalidate(
    pic: &PocketIc,
    scope: OperatorScope,
    operator: Principal,
    gateway: Principal,
    sequence: u64,
    revoke: bool,
) {
    if revoke {
        let response: Result<GatewayRevocationResponse, GatewayRevocationFailure> = update(
            pic,
            scope,
            operator,
            "blob_revoke_gateway",
            GatewayRevocationRequest {
                scope,
                gateway: gateway.into(),
            },
        );
        assert!(response.unwrap().removed);
    } else {
        let response: Result<(), GatewaySyncFailure> = update(
            pic,
            scope,
            operator,
            "blob_cancel_gateway_sync",
            GatewaySyncCancellation { scope, sequence },
        );
        response.unwrap();
    }
}
