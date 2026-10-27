//! Mounted historical inspection preserves original authority and stable state.

mod content;
mod history;
mod pagination;
pub(in crate::pic::blob_service) use pagination::qualify as qualify_pagination;

use super::{PocketIc, Principal, UploadAdmissionRequest, contracts, decode_blob};
use contracts::dto::{
    reference::{
        ReferenceAction, ReferenceChange, ReferenceCommand, ReferenceFailure,
        ReferenceReceiptLookup, ReferenceReceiptResponse,
    },
    upload::{
        UploadState,
        admission::{UploadAdmissionFailure, UploadAdmissionResponse},
        manifest::{UploadManifestFailure, UploadManifestInspection, UploadManifestResponse},
    },
};

fn query<T: serde::Serialize + serde::de::DeserializeOwned>(
    pic: &PocketIc,
    permission: UploadAdmissionRequest,
    caller: Principal,
    method: &str,
    request: impl serde::Serialize,
) -> T {
    decode_blob(
        method,
        &pic.query_call(
            permission.upload.service.into(),
            caller,
            method,
            contracts::wire::encode_request(method, request).unwrap(),
        )
        .expect("mounted Blob inspection"),
    )
    .unwrap()
}

pub(super) fn registered(pic: &PocketIc, permission: UploadAdmissionRequest, fenced: bool) {
    let stable = pic.get_stable_memory(permission.upload.service.into());
    admission(pic, permission);
    manifest(pic, permission, true);
    content::check(pic, permission, fenced);
    history::check(pic, permission, fenced);
    assert_eq!(
        pic.get_stable_memory(permission.upload.service.into()),
        stable
    );
}

fn admission(pic: &PocketIc, permission: UploadAdmissionRequest) {
    let inspect = |actor, request| -> Result<UploadAdmissionResponse, UploadAdmissionFailure> {
        query(pic, permission, actor, "blob_upload_admission", request)
    };
    let response = inspect(permission.upload.tenant.into(), permission).unwrap();
    assert_eq!(response.permission, permission);
    assert_eq!(response.state, UploadState::Registered);
    assert_eq!(
        inspect(permission.uploader.into(), permission),
        Err(UploadAdmissionFailure::Denied)
    );
    let changed = UploadAdmissionRequest {
        expires_at_ns: permission.expires_at_ns + 1,
        ..permission
    };
    assert_eq!(
        inspect(permission.upload.tenant.into(), changed),
        Err(UploadAdmissionFailure::Conflict)
    );
}

pub(super) fn manifest(pic: &PocketIc, permission: UploadAdmissionRequest, prepared: bool) {
    let inspect = |actor| -> Result<UploadManifestResponse, UploadManifestFailure> {
        query(pic, permission, actor, "blob_upload_manifest", permission)
    };
    let expected = if prepared {
        UploadManifestInspection::Prepared(super::preparation(permission).declaration)
    } else {
        UploadManifestInspection::Unprepared
    };
    for actor in [permission.upload.tenant, permission.uploader] {
        let response = inspect(actor.into()).unwrap();
        assert_eq!(response.permission, permission);
        assert_eq!(response.manifest, expected);
    }
    assert_eq!(
        inspect(Principal::anonymous()),
        Err(UploadManifestFailure::Permission(
            UploadAdmissionFailure::Denied
        ))
    );
}

pub(super) fn receipt(pic: &PocketIc, permission: UploadAdmissionRequest, retained: bool) {
    let command = ReferenceCommand {
        upload: permission.upload,
        reference: 2,
        operation: 101,
        action: ReferenceAction::Retain,
    };
    let inspect = |actor, request| -> Result<ReferenceReceiptLookup, ReferenceFailure> {
        query(pic, permission, actor, "blob_reference_receipt", request)
    };
    let expected = if retained {
        ReferenceReceiptLookup::Found(ReferenceReceiptResponse {
            request: command,
            result: Ok(ReferenceChange::Changed),
        })
    } else {
        ReferenceReceiptLookup::Absent
    };
    assert_eq!(
        inspect(permission.upload.tenant.into(), command),
        Ok(expected)
    );
    assert_eq!(
        inspect(permission.uploader.into(), command),
        Err(ReferenceFailure::Denied)
    );
    if retained {
        assert_eq!(
            inspect(
                permission.upload.tenant.into(),
                ReferenceCommand {
                    action: ReferenceAction::Release,
                    ..command
                },
            ),
            Err(ReferenceFailure::Conflict)
        );
    }
}
