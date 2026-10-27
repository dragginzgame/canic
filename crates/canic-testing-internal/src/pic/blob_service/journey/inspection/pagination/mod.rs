//! Populated mounted scans qualify continuation across result and work limits.

mod service;

use super::{PocketIc, UploadAdmissionRequest, contracts, query};
use contracts::dto::upload::{
    admission::{UploadAdmissionFailure, UploadAdmissionMutation},
    history::{
        UploadContentState, UploadHistoryFailure, UploadHistoryFilter, UploadHistoryPage,
        UploadHistoryRequest, UploadHistoryScope,
    },
};

pub(in crate::pic::blob_service) fn qualify(pic: &PocketIc, permission: UploadAdmissionRequest) {
    // More than both the 32-result and 64-scan bounds, without provider effects.
    let expected: Vec<_> = (20_u128..85).collect();
    for &id in &expected {
        let mut input = permission;
        input.upload.upload = id;
        input.upload.object = id;
        input.upload.root = super::super::root(u8::try_from(id).unwrap());
        input.expires_at_ns = pic.get_time().as_nanos_since_unix_epoch() + 3_600_000_000_000;
        let admitted: Result<UploadAdmissionMutation, UploadAdmissionFailure> =
            super::super::update(
                pic,
                input.upload.service.into(),
                input.upload.tenant.into(),
                "blob_admit_upload",
                input,
            );
        assert_eq!(admitted.unwrap().admission.permission, input);
    }
    let stable = pic.get_stable_memory(permission.upload.service.into());
    let request = UploadHistoryRequest {
        service: permission.upload.service,
        namespace: permission.upload.namespace,
        scope: UploadHistoryScope::Tenant(permission.upload.tenant),
        filter: UploadHistoryFilter::Active,
        cursor: None,
    };
    let (ids, pages) = collect(pic, permission, request, UploadContentState::Reserved);
    assert_eq!(ids, expected);
    assert!(pages > 1);
    let (empty, pages) = collect(
        pic,
        permission,
        UploadHistoryRequest {
            filter: UploadHistoryFilter::DeletionPending,
            ..request
        },
        UploadContentState::DeletionPending,
    );
    assert!(empty.is_empty());
    assert!(
        pages > 1,
        "filtered scans must continue after the work bound"
    );
    service::check(pic, permission, request);
    assert_eq!(
        pic.get_stable_memory(permission.upload.service.into()),
        stable
    );
}

fn collect(
    pic: &PocketIc,
    permission: UploadAdmissionRequest,
    mut request: UploadHistoryRequest,
    state: UploadContentState,
) -> (Vec<u128>, usize) {
    let mut ids = Vec::new();
    for page in 1..=10 {
        let result: Result<UploadHistoryPage, UploadHistoryFailure> = query(
            pic,
            permission,
            permission.upload.tenant.into(),
            "blob_upload_history",
            request,
        );
        let result = result.unwrap();
        assert_eq!(result.request, request);
        assert!(!result.fenced);
        assert!(result.scanned <= 64);
        assert!(result.entries.len() <= 32);
        for entry in result.entries {
            assert_eq!(entry.request.tenant, permission.upload.tenant);
            assert_eq!(entry.state, state);
            assert!(
                ids.last()
                    .is_none_or(|previous| *previous < entry.request.upload)
            );
            ids.push(entry.request.upload);
        }
        let Some(next) = result.next else {
            return (ids, page);
        };
        assert!(
            request
                .cursor
                .is_none_or(|previous| previous.after_request < next.after_request)
        );
        assert_eq!(next.scope, request.scope);
        assert_eq!(next.filter, request.filter);
        assert_eq!(next.after_tenant, permission.upload.tenant);
        for changed in [
            UploadHistoryRequest {
                filter: UploadHistoryFilter::All,
                cursor: Some(next),
                ..request
            },
            UploadHistoryRequest {
                scope: UploadHistoryScope::Tenant(permission.uploader),
                cursor: Some(next),
                ..request
            },
        ] {
            let actor = match changed.scope {
                UploadHistoryScope::Tenant(tenant) => tenant.into(),
                UploadHistoryScope::Service => unreachable!(),
            };
            let refused: Result<UploadHistoryPage, UploadHistoryFailure> =
                query(pic, permission, actor, "blob_upload_history", changed);
            assert_eq!(refused, Err(UploadHistoryFailure::CursorScope));
        }
        request.cursor = Some(next);
    }
    panic!("bounded fixture history must terminate without repeating a cursor")
}
