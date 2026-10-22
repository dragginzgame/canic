//! Gateway refresh, revocation and pending-attempt cancellation use shared durable state.

use super::{OperatorScope, PocketIc, Principal, contracts, status, update};
use contracts::dto::gateway::{
    GatewayRevocationFailure, GatewayRevocationRequest, GatewayRevocationResponse,
    sync::{GatewaySyncCancellation, GatewaySyncFailure, GatewaySyncResponse},
};

fn sync(
    pic: &PocketIc,
    scope: OperatorScope,
    actor: Principal,
) -> Result<GatewaySyncResponse, GatewaySyncFailure> {
    update(pic, scope, actor, "blob_sync_gateways", scope)
}

pub(super) fn lost_reply(
    pic: &PocketIc,
    scope: OperatorScope,
    operator: Principal,
    gateway: Principal,
) {
    let before = status(pic, scope, operator).gateways.last_sequence;
    let _message = pic
        .submit_call(
            scope.service.into(),
            operator,
            "blob_sync_gateways",
            contracts::wire::encode_request("blob_sync_gateways", scope).unwrap(),
        )
        .unwrap();
    for _ in 0..30 {
        pic.tick();
        let observed = status(pic, scope, operator).gateways;
        if observed.last_sequence == before + 1 && observed.pending_sequence.is_none() {
            assert_eq!(observed.members, vec![gateway.into()]);
            return;
        }
    }
    panic!("discarded sync reply must remain recoverable from the original sequence");
}

pub(super) fn check(pic: &PocketIc, scope: OperatorScope, operator: Principal, gateway: Principal) {
    assert_eq!(
        sync(pic, scope, scope.payment_account.into()),
        Err(GatewaySyncFailure::Denied)
    );
    let before = status(pic, scope, operator).gateways;
    for changed in super::super::wrong_scopes(scope) {
        let result: Result<GatewaySyncResponse, GatewaySyncFailure> =
            update(pic, scope, operator, "blob_sync_gateways", changed);
        assert_eq!(result, Err(GatewaySyncFailure::Binding));
    }
    let response = sync(pic, scope, operator).unwrap();
    assert_eq!(response.scope, scope);
    assert!(response.sequence > before.last_sequence);
    let after = status(pic, scope, operator).gateways;
    assert_eq!(after.members, vec![gateway.into()]);
    assert_eq!(after.pending_sequence, None);
    assert_eq!(after.last_sequence, response.sequence);
    let request = GatewayRevocationRequest {
        scope,
        gateway: gateway.into(),
    };
    let denied: Result<GatewayRevocationResponse, GatewayRevocationFailure> = update(
        pic,
        scope,
        scope.payment_account.into(),
        "blob_revoke_gateway",
        request,
    );
    assert_eq!(denied, Err(GatewayRevocationFailure::Denied));
    for removed in [true, false] {
        let response: Result<GatewayRevocationResponse, GatewayRevocationFailure> =
            update(pic, scope, operator, "blob_revoke_gateway", request);
        assert_eq!(
            response.unwrap(),
            GatewayRevocationResponse { request, removed }
        );
        assert!(status(pic, scope, operator).gateways.members.is_empty());
    }
}

pub(super) fn failed_sync(
    pic: &PocketIc,
    scope: OperatorScope,
    operator: Principal,
    failure: GatewaySyncFailure,
) {
    assert_eq!(sync(pic, scope, operator), Err(failure));
    let pending = status(pic, scope, operator).gateways;
    let sequence = pending
        .pending_sequence
        .expect("provider refusal retains original attempt");
    assert_eq!(sync(pic, scope, operator), Err(GatewaySyncFailure::Busy));
    let cancel = |actor, sequence| -> Result<(), GatewaySyncFailure> {
        update(
            pic,
            scope,
            actor,
            "blob_cancel_gateway_sync",
            GatewaySyncCancellation { scope, sequence },
        )
    };
    assert_eq!(
        cancel(scope.payment_account.into(), sequence),
        Err(GatewaySyncFailure::Denied)
    );
    assert_eq!(
        cancel(operator, sequence + 1),
        Err(GatewaySyncFailure::Conflict)
    );
    assert_eq!(status(pic, scope, operator).gateways, pending);
    assert_eq!(cancel(operator, sequence), Ok(()));
    let after = status(pic, scope, operator).gateways;
    assert_eq!(after.members, pending.members);
    assert_eq!(after.last_sequence, pending.last_sequence);
    assert_eq!(after.pending_sequence, None);
    assert_eq!(
        cancel(operator, sequence),
        Err(GatewaySyncFailure::Conflict)
    );
}

pub(super) fn fenced(
    pic: &PocketIc,
    scope: OperatorScope,
    operator: Principal,
    gateway: Principal,
) {
    let before = status(pic, scope, operator).gateways;
    assert!(before.fenced);
    assert_eq!(sync(pic, scope, operator), Err(GatewaySyncFailure::Fenced));
    let cancelled: Result<(), GatewaySyncFailure> = update(
        pic,
        scope,
        operator,
        "blob_cancel_gateway_sync",
        GatewaySyncCancellation { scope, sequence: 1 },
    );
    assert_eq!(cancelled, Err(GatewaySyncFailure::Fenced));
    let revoked: Result<GatewayRevocationResponse, GatewayRevocationFailure> = update(
        pic,
        scope,
        operator,
        "blob_revoke_gateway",
        GatewayRevocationRequest {
            scope,
            gateway: gateway.into(),
        },
    );
    assert_eq!(revoked, Err(GatewayRevocationFailure::Fenced));
    assert_eq!(status(pic, scope, operator).gateways, before);
}
