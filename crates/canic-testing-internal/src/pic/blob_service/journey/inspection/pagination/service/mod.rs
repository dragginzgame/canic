//! Operator pagination must equal independently authorized tenant histories.

use super::{
    PocketIc, UploadAdmissionRequest, UploadHistoryFailure, UploadHistoryFilter, UploadHistoryPage,
    UploadHistoryRequest, UploadHistoryScope, contracts, query,
};
use contracts::dto::upload::history::UploadHistoryEntry;
use ic_platform::principal::ExternalPrincipal as Principal;
use std::collections::BTreeSet;

pub(super) fn check(
    pic: &PocketIc,
    permission: UploadAdmissionRequest,
    request: UploadHistoryRequest,
) {
    let request = UploadHistoryRequest {
        scope: UploadHistoryScope::Service,
        filter: UploadHistoryFilter::All,
        ..request
    };
    // The refusal journey enrolled this second tenant through the public endpoint.
    let tenants = [
        permission.upload.tenant,
        Principal::from_slice(&[19; 29]).into(),
    ];
    let mut expected = Vec::new();
    for tenant in tenants {
        let rows = scan(
            pic,
            permission,
            tenant.into(),
            UploadHistoryRequest {
                scope: UploadHistoryScope::Tenant(tenant),
                ..request
            },
        );
        assert!(!rows.is_empty());
        assert!(rows.iter().all(|row| row.request.tenant == tenant));
        expected.extend(rows);
    }
    let actual = scan(pic, permission, Principal::from_slice(&[7; 29]), request);
    expected.sort_by_key(|entry| (entry.request.tenant, entry.request.upload));
    assert_eq!(actual, expected);
}

fn scan(
    pic: &PocketIc,
    permission: UploadAdmissionRequest,
    actor: Principal,
    mut request: UploadHistoryRequest,
) -> Vec<UploadHistoryEntry> {
    let mut rows = Vec::new();
    let mut identities = BTreeSet::new();
    let mut positions = BTreeSet::new();
    for page in 1..=10 {
        let result: Result<UploadHistoryPage, UploadHistoryFailure> =
            query(pic, permission, actor, "blob_upload_history", request);
        let result = result.unwrap();
        assert_eq!(result.request, request);
        assert!(!result.fenced);
        assert!(result.scanned <= 64);
        assert!(result.entries.len() <= 32);
        for entry in result.entries {
            assert!(identities.insert((entry.request.tenant, entry.request.upload)));
            rows.push(entry);
        }
        if request.scope == UploadHistoryScope::Service {
            let denied: Result<UploadHistoryPage, UploadHistoryFailure> = query(
                pic,
                permission,
                permission.upload.tenant.into(),
                "blob_upload_history",
                request,
            );
            assert_eq!(denied, Err(UploadHistoryFailure::Denied));
        }
        let Some(next) = result.next else {
            if request.scope == UploadHistoryScope::Service {
                assert!(page > 1);
            }
            rows.sort_by_key(|entry| (entry.request.tenant, entry.request.upload));
            return rows;
        };
        assert_eq!(next.service, request.service);
        assert_eq!(next.namespace, request.namespace);
        assert_eq!(next.scope, request.scope);
        assert_eq!(next.filter, request.filter);
        assert!(positions.insert((next.after_tenant, next.after_request)));
        if request.scope == UploadHistoryScope::Service {
            let changed = UploadHistoryRequest {
                scope: UploadHistoryScope::Tenant(permission.upload.tenant),
                cursor: Some(next),
                ..request
            };
            let refused: Result<UploadHistoryPage, UploadHistoryFailure> = query(
                pic,
                permission,
                permission.upload.tenant.into(),
                "blob_upload_history",
                changed,
            );
            assert_eq!(refused, Err(UploadHistoryFailure::CursorScope));
        }
        request.cursor = Some(next);
    }
    panic!("fixture history must terminate without repeating a cursor")
}
