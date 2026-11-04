//! Mounted history binds its observer and cursor while retaining original identities.

use super::{PocketIc, UploadAdmissionRequest, contracts, query};
use contracts::dto::upload::history::{
    UploadContentState, UploadHistoryCursor, UploadHistoryFailure, UploadHistoryFilter,
    UploadHistoryPage, UploadHistoryRequest, UploadHistoryScope,
};

pub(super) fn check(pic: &PocketIc, permission: UploadAdmissionRequest, fenced: bool) {
    let request = UploadHistoryRequest {
        service: permission.upload.service,
        namespace: permission.upload.namespace,
        scope: UploadHistoryScope::Tenant(permission.upload.tenant),
        filter: UploadHistoryFilter::All,
        cursor: None,
    };
    let inspect = |actor, request| -> Result<UploadHistoryPage, UploadHistoryFailure> {
        query(pic, permission, actor, "blob_upload_history", request)
    };
    let response = inspect(permission.upload.tenant.into(), request).unwrap();
    assert_eq!(response.request, request);
    assert_eq!(response.fenced, fenced);
    assert!(response.scanned <= 64);
    assert!(response.entries.len() <= 32);
    assert!(
        response
            .entries
            .iter()
            .all(|entry| entry.request.tenant == permission.upload.tenant)
    );
    let original = response
        .entries
        .iter()
        .find(|entry| entry.request == permission.upload)
        .unwrap();
    assert_eq!(original.state, UploadContentState::Live);
    assert_eq!(
        inspect(permission.uploader.into(), request),
        Err(UploadHistoryFailure::Denied)
    );
    let wrong_cursor = UploadHistoryRequest {
        cursor: Some(UploadHistoryCursor {
            service: request.service,
            namespace: request.namespace + 1,
            scope: request.scope,
            filter: request.filter,
            after_tenant: permission.upload.tenant,
            after_request: permission.upload.upload,
        }),
        ..request
    };
    assert_eq!(
        inspect(permission.upload.tenant.into(), wrong_cursor),
        Err(UploadHistoryFailure::CursorScope)
    );
    let foreign = UploadHistoryRequest {
        scope: UploadHistoryScope::Tenant(permission.uploader),
        ..request
    };
    let empty = inspect(permission.uploader.into(), foreign).unwrap();
    assert_eq!(empty.request, foreign);
    assert!(empty.entries.is_empty());
    assert_eq!(empty.next, None);
}
