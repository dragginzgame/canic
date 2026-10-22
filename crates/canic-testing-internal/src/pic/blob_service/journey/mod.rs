//! Mounted trusted-registration qualification with controlled content.
//! This does not fetch provider bytes or qualify deployed browser delivery.

mod inspection;
mod refusals;
pub(super) use refusals::qualify as qualify_refusals;

use super::tests::decode_blob;
use canic_blob_service::__private::ic_blob_storage_contracts as contracts;
use contracts::{
    dto::{
        download::{DownloadFailure, DownloadRequest, DownloadResponse},
        reference::{
            ReferenceAction, ReferenceChange, ReferenceCommand, ReferenceFailure,
            ReferenceMutationResponse,
        },
        upload::{
            UploadState, UploadStatusFailure, UploadStatusResponse,
            admission::{
                UploadAdmissionFailure, UploadAdmissionMutation, UploadAdmissionRequest,
                UploadRevocationResponse,
            },
            certificate::CaffeineUploadCertificateResponse,
            manifest::{
                UploadManifestDeclaration, UploadManifestFailure, UploadManifestHeader,
                UploadManifestMutation, UploadManifestRequest,
            },
        },
    },
    identity::caffeine::{
        CaffeineHashLimits, CaffeineHeader,
        manifest::builder::{CaffeineBuiltManifest, CaffeineManifestBuilder},
    },
};
use ic_platform::pocket_ic::PocketIc;
use ic_platform::principal::ExternalPrincipal as Principal;
use std::num::{NonZeroU64, NonZeroUsize};

fn manifest(byte: u8) -> CaffeineBuiltManifest {
    let mut builder = CaffeineManifestBuilder::new(
        1,
        &[
            CaffeineHeader {
                name: "Content-Length",
                value: "1",
            },
            CaffeineHeader {
                name: "Content-Type",
                value: "text/plain",
            },
        ],
        CaffeineHashLimits {
            max_content_bytes: NonZeroU64::MIN,
            max_append_bytes: NonZeroUsize::MIN,
            max_headers: NonZeroUsize::new(2).unwrap(),
            max_header_bytes: NonZeroUsize::new(128).unwrap(),
        },
        NonZeroUsize::MIN,
    )
    .unwrap();
    builder.append(0, &[byte]).unwrap();
    builder.finish().unwrap()
}

pub(super) fn root(byte: u8) -> [u8; 32] {
    *manifest(byte).hashes().provider_root.as_bytes()
}

fn update<T: serde::Serialize + serde::de::DeserializeOwned>(
    pic: &PocketIc,
    service: Principal,
    caller: Principal,
    method: &str,
    request: impl serde::Serialize,
) -> T {
    decode_blob(
        method,
        &pic.update_call(
            service,
            caller,
            method,
            contracts::wire::encode_request(method, request).unwrap(),
        )
        .expect("mounted Blob update"),
    )
    .unwrap()
}

pub(super) fn qualify(
    pic: &PocketIc,
    permission: UploadAdmissionRequest,
    restore: impl Fn(),
) -> UploadAdmissionRequest {
    let service = permission.upload.service.into();
    let uploader = permission.uploader.into();
    let built = manifest(u8::try_from(permission.upload.upload).unwrap());
    let input = preparation(permission);
    inspection::manifest(pic, permission, false);
    let denied: Result<UploadManifestMutation, UploadManifestFailure> = update(
        pic,
        service,
        permission.upload.tenant.into(),
        "blob_prepare_upload",
        &input,
    );
    assert_eq!(
        denied,
        Err(UploadManifestFailure::Permission(
            UploadAdmissionFailure::Denied
        ))
    );
    let prepared: Result<UploadManifestMutation, UploadManifestFailure> =
        update(pic, service, uploader, "blob_prepare_upload", &input);
    assert!(prepared.unwrap().changed);
    inspection::manifest(pic, permission, true);
    restore();
    let replay: Result<UploadManifestMutation, UploadManifestFailure> =
        update(pic, service, uploader, "blob_prepare_upload", &input);
    assert!(!replay.unwrap().changed);
    certificate(pic, permission, built.hashes().provider_root.to_string());
    restore();
    check_registration(pic, &permission, false);
    references(pic, permission);
    restore();
    inspection::receipt(pic, permission, true);
    revoke_registered(pic, permission);
    permission
}

fn certificate(pic: &PocketIc, permission: UploadAdmissionRequest, root: String) {
    let service = permission.upload.service.into();
    let args = ic_platform::candid::contract::encode_args((root.clone(),)).unwrap();
    let denied = pic
        .update_call(
            service,
            permission.upload.tenant.into(),
            "_immutableObjectStorageCreateCertificate",
            args.clone(),
        )
        .unwrap_err();
    assert_eq!(
        denied.reject_code,
        ic_platform::pocket_ic::RejectCode::CanisterError
    );
    if permission.upload.upload == 2 {
        lose_certificate_reply(pic, permission, args.clone());
    } else {
        let bytes = pic
            .update_call(
                service,
                permission.uploader.into(),
                "_immutableObjectStorageCreateCertificate",
                args.clone(),
            )
            .unwrap();
        let (certificate,): (CaffeineUploadCertificateResponse,) =
            ic_platform::candid::decode_args(&bytes, canic::testing::CANDID_REPLY_LIMITS).unwrap();
        assert_eq!(certificate.method, "upload");
        assert_eq!(certificate.blob_hash, root);
    }
    let repeated = pic
        .update_call(
            service,
            permission.uploader.into(),
            "_immutableObjectStorageCreateCertificate",
            args,
        )
        .unwrap_err();
    assert_eq!(
        repeated.reject_code,
        ic_platform::pocket_ic::RejectCode::CanisterError
    );
}

fn lose_certificate_reply(pic: &PocketIc, permission: UploadAdmissionRequest, args: Vec<u8>) {
    // Never collect the ingress reply. Recover only the original retained intent.
    let _message = pic
        .submit_call(
            permission.upload.service.into(),
            permission.uploader.into(),
            "_immutableObjectStorageCreateCertificate",
            args,
        )
        .unwrap();
    for _ in 0..30 {
        pic.tick();
        let status: Result<UploadStatusResponse, UploadStatusFailure> = decode_blob(
            "blob_upload_status",
            &pic.query_call(
                permission.upload.service.into(),
                permission.upload.tenant.into(),
                "blob_upload_status",
                contracts::wire::encode_request("blob_upload_status", permission.upload).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let status = status.unwrap();
        assert_eq!(status.upload, permission.upload);
        assert!(!status.revoked);
        if status.state == UploadState::Registered {
            return;
        }
        assert_eq!(status.state, UploadState::Reserved);
    }
    panic!("original certificate exposure must become observable without its reply");
}

fn references(pic: &PocketIc, permission: UploadAdmissionRequest) {
    let service = permission.upload.service.into();
    let tenant = permission.upload.tenant.into();
    let mut command = ReferenceCommand {
        upload: permission.upload,
        reference: 2,
        operation: 101,
        action: ReferenceAction::Retain,
    };
    inspection::receipt(pic, permission, false);
    let denied: Result<ReferenceMutationResponse, ReferenceFailure> = update(
        pic,
        service,
        permission.uploader.into(),
        "blob_apply_reference",
        command,
    );
    assert_eq!(denied, Err(ReferenceFailure::Denied));
    let retained: Result<ReferenceMutationResponse, ReferenceFailure> =
        update(pic, service, tenant, "blob_apply_reference", command);
    let retained = retained.unwrap();
    assert_eq!(retained.receipt.result, Ok(ReferenceChange::Changed));
    assert!(!retained.replayed);
    let replay: Result<ReferenceMutationResponse, ReferenceFailure> =
        update(pic, service, tenant, "blob_apply_reference", command);
    let replay = replay.unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.receipt, retained.receipt);
    inspection::receipt(pic, permission, true);
    let request = DownloadRequest {
        service: permission.upload.service,
        tenant: permission.upload.tenant,
        namespace: permission.upload.namespace,
        root: permission.upload.root,
        object: permission.upload.object,
        incarnation: permission.upload.incarnation,
        reference: 2,
    };
    let denied: Result<DownloadResponse, DownloadFailure> = update(
        pic,
        service,
        permission.uploader.into(),
        "blob_download_descriptor",
        request,
    );
    assert_eq!(denied, Err(DownloadFailure::Denied));
    let response: Result<DownloadResponse, DownloadFailure> =
        update(pic, service, tenant, "blob_download_descriptor", request);
    let response = response.unwrap();
    assert_eq!(response.request, request);
    assert_eq!(response.owner, permission.upload.service);
    assert_eq!(response.bytes, 1);
    command.operation = 102;
    command.action = ReferenceAction::Release;
    let released: Result<ReferenceMutationResponse, ReferenceFailure> =
        update(pic, service, tenant, "blob_apply_reference", command);
    assert_eq!(
        released.unwrap().receipt.result,
        Ok(ReferenceChange::Changed)
    );
    let unavailable: Result<DownloadResponse, DownloadFailure> =
        update(pic, service, tenant, "blob_download_descriptor", request);
    assert_eq!(unavailable, Err(DownloadFailure::Unavailable));
}

pub(super) fn check_registration(
    pic: &PocketIc,
    permission: &UploadAdmissionRequest,
    fenced: bool,
) {
    use contracts::dto::reference::status::{ReferenceStatusRequest, ReferenceStatusResponse};
    let request = ReferenceStatusRequest {
        upload: permission.upload,
        reference: permission.upload.first_reference,
    };
    let response: Result<ReferenceStatusResponse, ReferenceFailure> = decode_blob(
        "blob_reference_status",
        &pic.query_call(
            permission.upload.service.into(),
            permission.upload.tenant.into(),
            "blob_reference_status",
            contracts::wire::encode_request("blob_reference_status", request).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let response = response.unwrap();
    assert_eq!(response.request, request);
    assert!(response.live);
    assert_eq!(response.fenced, fenced);
    inspection::registered(pic, *permission, fenced);
}

/// Refusals must happen before any fresh permission consumes bounded capacity.
pub(super) fn admission_refusals(pic: &PocketIc, permission: UploadAdmissionRequest) {
    let mut wrong_service = permission;
    wrong_service.upload.service = Principal::from_slice(&[99; 29]).into();
    let mut wrong_namespace = permission;
    wrong_namespace.upload.namespace += 1;
    let mut expired = permission;
    expired.expires_at_ns = pic.get_time().as_nanos_since_unix_epoch();
    for (request, expected) in [
        (wrong_service, UploadAdmissionFailure::Binding),
        (wrong_namespace, UploadAdmissionFailure::Binding),
        (expired, UploadAdmissionFailure::Expired),
    ] {
        let response: Result<UploadAdmissionMutation, UploadAdmissionFailure> = update(
            pic,
            permission.upload.service.into(),
            permission.upload.tenant.into(),
            "blob_admit_upload",
            request,
        );
        assert_eq!(response, Err(expected));
    }
}

fn revoke_registered(pic: &PocketIc, permission: UploadAdmissionRequest) {
    let service = permission.upload.service.into();
    let tenant = permission.upload.tenant.into();
    let denied: Result<UploadRevocationResponse, UploadAdmissionFailure> = update(
        pic,
        service,
        permission.uploader.into(),
        "blob_revoke_upload",
        permission,
    );
    assert_eq!(denied, Err(UploadAdmissionFailure::Denied));
    for changed in [true, false] {
        let response: Result<UploadRevocationResponse, UploadAdmissionFailure> =
            update(pic, service, tenant, "blob_revoke_upload", permission);
        let response = response.unwrap();
        assert_eq!(response.changed, changed);
        assert_eq!(response.admission.permission, permission);
        assert_eq!(response.admission.state, UploadState::Registered);
        assert!(response.admission.revoked);
    }
    // Withdrawing issuance permission does not revoke already-registered references.
    let request = DownloadRequest {
        service: permission.upload.service,
        tenant: permission.upload.tenant,
        namespace: permission.upload.namespace,
        root: permission.upload.root,
        object: permission.upload.object,
        incarnation: permission.upload.incarnation,
        reference: permission.upload.first_reference,
    };
    let response: Result<DownloadResponse, DownloadFailure> =
        update(pic, service, tenant, "blob_download_descriptor", request);
    assert_eq!(response.unwrap().request, request);
}

fn preparation(permission: UploadAdmissionRequest) -> UploadManifestRequest {
    let built = manifest(u8::try_from(permission.upload.upload).unwrap());
    UploadManifestRequest {
        permission,
        declaration: UploadManifestDeclaration {
            chunks: built
                .manifest()
                .chunks()
                .iter()
                .map(|chunk| *chunk.as_bytes())
                .collect(),
            headers: vec![
                UploadManifestHeader {
                    name: "Content-Length".into(),
                    value: "1".into(),
                },
                UploadManifestHeader {
                    name: "Content-Type".into(),
                    value: "text/plain".into(),
                },
            ],
        },
    }
}
