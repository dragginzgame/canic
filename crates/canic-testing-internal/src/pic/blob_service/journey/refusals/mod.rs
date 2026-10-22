//! Mounted refusals before exposure, independent of confirmed-reference cleanup.
use super::{contracts, preparation, root, update};
use contracts::dto::{
    tenant::{TenantEnrollmentResponse, TenantFailure, TenantScope, TenantUpdateRequest},
    upload::{
        admission::{
            UploadAdmissionFailure, UploadAdmissionMutation, UploadAdmissionRequest,
            UploadRevocationResponse,
        },
        manifest::{UploadManifestFailure, UploadManifestMutation},
    },
};
use ic_platform::pocket_ic::PocketIc;
use ic_platform::principal::ExternalPrincipal as Principal;
use std::time::Duration;

pub fn qualify(pic: &PocketIc, original: UploadAdmissionRequest) {
    let service = original.upload.service.into();
    let tenant = Principal::from_slice(&[19; 29]);
    let enrolled: Result<TenantEnrollmentResponse, TenantFailure> = update(
        pic,
        service,
        Principal::from_slice(&[7; 29]),
        "blob_update_tenant",
        TenantUpdateRequest {
            scope: TenantScope {
                service: original.upload.service,
                namespace: original.upload.namespace,
                tenant: tenant.into(),
            },
            expected: None,
            active: true,
        },
    );
    assert!(enrolled.unwrap().enrollment.unwrap().active);
    for revoked in [true, false] {
        let id = if revoked { 3 } else { 4 };
        let mut permission = original;
        permission.upload.tenant = tenant.into();
        permission.upload.upload = u128::from(id);
        permission.upload.object = u128::from(id);
        permission.upload.root = root(id);
        permission.expires_at_ns = pic.get_time().as_nanos_since_unix_epoch() + 1_000_000_000;
        let admitted: Result<UploadAdmissionMutation, UploadAdmissionFailure> =
            update(pic, service, tenant, "blob_admit_upload", permission);
        assert_eq!(admitted.unwrap().admission.permission, permission);
        let expected = if revoked {
            let response: Result<UploadRevocationResponse, UploadAdmissionFailure> =
                update(pic, service, tenant, "blob_revoke_upload", permission);
            assert!(response.unwrap().admission.revoked);
            UploadManifestFailure::Revoked
        } else {
            pic.advance_time(Duration::from_secs(2));
            UploadManifestFailure::Permission(UploadAdmissionFailure::Expired)
        };
        let refused: Result<UploadManifestMutation, UploadManifestFailure> = update(
            pic,
            service,
            permission.uploader.into(),
            "blob_prepare_upload",
            preparation(permission),
        );
        assert_eq!(refused, Err(expected));
    }
    certificate_arguments(pic, original);
}

fn certificate_arguments(pic: &PocketIc, original: UploadAdmissionRequest) {
    let limit = canic_blob_service::lifecycle::REQUEST_LIMITS.max_bytes;
    for (args, expected) in [
        (vec![0], ic_platform::pocket_ic::RejectCode::CanisterError),
        (
            vec![0; limit + 1],
            ic_platform::pocket_ic::RejectCode::CanisterReject,
        ),
    ] {
        let error = pic
            .update_call(
                original.upload.service.into(),
                original.uploader.into(),
                "_immutableObjectStorageCreateCertificate",
                args,
            )
            .unwrap_err();
        assert_eq!(error.reject_code, expected);
    }
}
