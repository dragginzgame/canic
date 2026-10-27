//! Shared external GC callbacks retain liabilities through acknowledgment and restore.

use super::{OperatorScope, PocketIc, Principal, contracts, status, update};
use contracts::dto::{
    reference::{ReferenceAction, ReferenceCommand, ReferenceFailure, ReferenceMutationResponse},
    upload::admission::UploadAdmissionRequest,
};
use ic_platform::candid::contract::encode_args;

const LIVE: &str = "_immutableObjectStorageBlobsAreLive";
const LIST: &str = "_immutableObjectStorageBlobsToDelete";
const ACK: &str = "_immutableObjectStorageConfirmBlobDeletion";

fn live(pic: &PocketIc, scope: OperatorScope, roots: Vec<Vec<u8>>) -> Vec<bool> {
    let bytes = pic
        .query_call(
            scope.service.into(),
            Principal::anonymous(),
            LIVE,
            encode_args((roots,)).unwrap(),
        )
        .unwrap();
    let (result,): (Vec<bool>,) =
        ic_platform::candid::decode_args(&bytes, canic::testing::CANDID_REPLY_LIMITS).unwrap();
    result
}

fn listed(pic: &PocketIc, scope: OperatorScope) -> Vec<Vec<u8>> {
    let bytes = pic
        .query_call(
            scope.service.into(),
            Principal::from_slice(&[42]),
            LIST,
            encode_args(()).unwrap(),
        )
        .unwrap();
    let (result,): (Vec<Vec<u8>>,) =
        ic_platform::candid::decode_args(&bytes, canic::testing::CANDID_REPLY_LIMITS).unwrap();
    result
}

fn acknowledge(pic: &PocketIc, scope: OperatorScope, roots: Vec<Vec<u8>>) {
    let bytes = pic
        .update_call(
            scope.service.into(),
            Principal::from_slice(&[42]),
            ACK,
            encode_args((roots,)).unwrap(),
        )
        .unwrap();
    let (): () =
        ic_platform::candid::decode_args(&bytes, canic::testing::CANDID_REPLY_LIMITS).unwrap();
}

pub(super) fn check(
    pic: &PocketIc,
    scope: OperatorScope,
    operator: Principal,
    permission: UploadAdmissionRequest,
) {
    bounds(pic, scope);
    let root = permission.upload.root.to_vec();
    assert_eq!(
        live(pic, scope, vec![root.clone(), vec![88; 32], root.clone()]),
        vec![true, true, true]
    );
    assert!(listed(pic, scope).is_empty());
    for actor in [
        operator,
        permission.upload.tenant.into(),
        Principal::anonymous(),
    ] {
        assert!(
            pic.query_call(scope.service.into(), actor, LIST, encode_args(()).unwrap())
                .is_err()
        );
        assert!(
            pic.update_call(
                scope.service.into(),
                actor,
                ACK,
                encode_args((vec![root.clone()],)).unwrap()
            )
            .is_err()
        );
    }
    assert!(
        pic.update_call(
            scope.service.into(),
            Principal::from_slice(&[42]),
            ACK,
            encode_args((vec![root.clone()],)).unwrap()
        )
        .is_err()
    );
    for malformed in [vec![], vec![0; 31], vec![0; 33]] {
        assert!(
            pic.query_call(
                scope.service.into(),
                Principal::anonymous(),
                LIVE,
                encode_args((vec![malformed],)).unwrap()
            )
            .is_err()
        );
    }
    let released: Result<ReferenceMutationResponse, ReferenceFailure> = update(
        pic,
        scope,
        permission.upload.tenant.into(),
        "blob_apply_reference",
        ReferenceCommand {
            upload: permission.upload,
            reference: permission.upload.first_reference,
            operation: 103,
            action: ReferenceAction::Release,
        },
    );
    released.unwrap();
    assert_eq!(live(pic, scope, vec![root.clone()]), vec![false]);
    assert_eq!(listed(pic, scope), vec![root.clone()]);
    let obligations = status(pic, scope, operator).uploads;
    assert!(obligations.physical_bytes > 0 && obligations.liability_bytes > 0);
    rejected_acknowledgments(pic, scope, &root);
    assert!(
        pic.update_call(
            scope.service.into(),
            Principal::from_slice(&[42]),
            ACK,
            encode_args((vec![root.clone(), vec![88; 32]],)).unwrap()
        )
        .is_err()
    );
    assert_eq!(listed(pic, scope), vec![root.clone()]);
    acknowledge(
        pic,
        scope,
        vec![root.clone(); contracts::provider::gc::MAX_ROOTS],
    );
    acknowledge(pic, scope, vec![root]);
    assert!(listed(pic, scope).is_empty());
    assert_eq!(status(pic, scope, operator).uploads, obligations);
}

fn rejected_acknowledgments(pic: &PocketIc, scope: OperatorScope, root: &[u8]) {
    use contracts::provider::gc::{MAX_REQUEST_BYTES, MAX_ROOTS};

    let requests = [
        encode_args((vec![root.to_vec(), vec![]],)).unwrap(),
        encode_args((vec![root.to_vec(), vec![0u8; 31]],)).unwrap(),
        encode_args((vec![root.to_vec(), vec![0u8; 33]],)).unwrap(),
        encode_args((vec![root.to_vec(); MAX_ROOTS + 1],)).unwrap(),
        vec![0; MAX_REQUEST_BYTES + 1],
    ];
    for request in requests {
        assert!(
            pic.update_call(
                scope.service.into(),
                Principal::from_slice(&[42]),
                ACK,
                request,
            )
            .is_err()
        );
        assert_eq!(listed(pic, scope), vec![root.to_vec()]);
        assert_eq!(live(pic, scope, vec![root.to_vec()]), vec![false]);
    }
}

pub(super) fn fenced(pic: &PocketIc, scope: OperatorScope, permission: UploadAdmissionRequest) {
    let roots = encode_args((vec![permission.upload.root.to_vec()],)).unwrap();
    assert!(
        pic.query_call(
            scope.service.into(),
            Principal::anonymous(),
            LIVE,
            roots.clone()
        )
        .is_err()
    );
    assert!(
        pic.query_call(
            scope.service.into(),
            Principal::from_slice(&[42]),
            LIST,
            encode_args(()).unwrap()
        )
        .is_err()
    );
    assert!(
        pic.update_call(
            scope.service.into(),
            Principal::from_slice(&[42]),
            ACK,
            roots
        )
        .is_err()
    );
}

pub(super) fn restored(
    pic: &PocketIc,
    scope: OperatorScope,
    operator: Principal,
    permission: UploadAdmissionRequest,
) {
    assert!(listed(pic, scope).is_empty());
    acknowledge(pic, scope, vec![permission.upload.root.to_vec()]);
    assert!(listed(pic, scope).is_empty());
    let response: Result<
        contracts::dto::gateway::GatewayRevocationResponse,
        contracts::dto::gateway::GatewayRevocationFailure,
    > = update(
        pic,
        scope,
        operator,
        "blob_revoke_gateway",
        contracts::dto::gateway::GatewayRevocationRequest {
            scope,
            gateway: Principal::from_slice(&[42]).into(),
        },
    );
    assert!(response.unwrap().removed);
    assert!(
        pic.query_call(
            scope.service.into(),
            Principal::from_slice(&[42]),
            LIST,
            encode_args(()).unwrap()
        )
        .is_err()
    );
    assert!(
        pic.update_call(
            scope.service.into(),
            Principal::from_slice(&[42]),
            ACK,
            encode_args((vec![permission.upload.root.to_vec()],)).unwrap()
        )
        .is_err()
    );
    assert_eq!(live(pic, scope, vec![vec![88; 32]]), vec![true]);
}

// Qualify shared decoder limits through the actual mounted external endpoint.
fn bounds(pic: &PocketIc, scope: OperatorScope) {
    use contracts::provider::gc::{MAX_REQUEST_BYTES, MAX_ROOTS};

    assert_eq!(
        live(pic, scope, vec![vec![88; 32]; MAX_ROOTS]),
        vec![true; MAX_ROOTS]
    );
    for request in [
        encode_args((vec![vec![88u8; 32]; MAX_ROOTS + 1],)).unwrap(),
        vec![0; MAX_REQUEST_BYTES + 1],
    ] {
        assert!(
            pic.query_call(scope.service.into(), Principal::anonymous(), LIVE, request)
                .is_err()
        );
    }
}
