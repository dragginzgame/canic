//! Mounted discovery and headroom remain tenant-scoped observations, not grants.

use super::{PocketIc, Principal, UploadAdmissionRequest, contracts, query};
use contracts::dto::{
    reference::capacity::{
        ReferenceCapacityFailure, ReferenceCapacityRequest, ReferenceCapacityResponse,
    },
    tenant::TenantScope,
    upload::{
        discovery::{UploadDiscoveryFailure, UploadDiscoveryRequest, UploadDiscoveryResponse},
        history::UploadContentState,
    },
};

pub(super) fn check(pic: &PocketIc, permission: UploadAdmissionRequest, fenced: bool) {
    let scope = TenantScope {
        service: permission.upload.service,
        tenant: permission.upload.tenant,
        namespace: permission.upload.namespace,
    };
    for root in [permission.upload.root, [0; 32]] {
        discovery(
            pic,
            permission,
            UploadDiscoveryRequest { scope, root },
            fenced,
        );
        capacity(
            pic,
            permission,
            ReferenceCapacityRequest { scope, root },
            fenced,
        );
    }
}

fn discovery(
    pic: &PocketIc,
    permission: UploadAdmissionRequest,
    request: UploadDiscoveryRequest,
    fenced: bool,
) {
    let inspect = |actor, request| -> Result<UploadDiscoveryResponse, UploadDiscoveryFailure> {
        query(pic, permission, actor, "blob_lookup_content", request)
    };
    let response = inspect(permission.upload.tenant.into(), request).unwrap();
    assert_eq!(response.request, request);
    assert_eq!(response.fenced, fenced);
    if request.root == permission.upload.root {
        let content = response.content.unwrap();
        assert_eq!(content.request, permission.upload);
        assert_eq!(content.state, UploadContentState::Live);
    } else {
        assert_eq!(response.content, None);
    }
    assert_eq!(
        inspect(permission.uploader.into(), request),
        Err(UploadDiscoveryFailure::Denied)
    );
    let foreign = UploadDiscoveryRequest {
        scope: TenantScope {
            tenant: permission.uploader,
            ..request.scope
        },
        ..request
    };
    assert_eq!(
        inspect(permission.uploader.into(), foreign)
            .unwrap()
            .content,
        None
    );
    let wrong_service = UploadDiscoveryRequest {
        scope: TenantScope {
            service: Principal::from_slice(&[99]).into(),
            ..request.scope
        },
        ..request
    };
    assert_eq!(
        inspect(permission.upload.tenant.into(), wrong_service),
        Err(UploadDiscoveryFailure::Binding)
    );
}

fn capacity(
    pic: &PocketIc,
    permission: UploadAdmissionRequest,
    request: ReferenceCapacityRequest,
    fenced: bool,
) {
    let inspect = |actor, request| -> Result<ReferenceCapacityResponse, ReferenceCapacityFailure> {
        query(pic, permission, actor, "blob_reference_capacity", request)
    };
    let response = inspect(permission.upload.tenant.into(), request).unwrap();
    assert_eq!(response.request, request);
    assert_eq!(response.fenced, fenced);
    if request.root == permission.upload.root {
        let headroom = response.headroom.unwrap();
        assert_eq!(headroom.release_reserved_receipts, 1);
        assert!(headroom.fresh_retains <= headroom.reference_slots);
        assert!(headroom.fresh_retains <= headroom.unreserved_receipts);
    } else {
        assert_eq!(response.headroom, None);
    }
    assert_eq!(
        inspect(permission.uploader.into(), request),
        Err(ReferenceCapacityFailure::Denied)
    );
    let foreign = ReferenceCapacityRequest {
        scope: TenantScope {
            tenant: permission.uploader,
            ..request.scope
        },
        ..request
    };
    assert_eq!(
        inspect(permission.uploader.into(), foreign)
            .unwrap()
            .headroom,
        None
    );
}
