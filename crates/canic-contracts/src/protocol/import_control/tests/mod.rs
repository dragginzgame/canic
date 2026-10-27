//! Each phase binds the original retained identity and rejects another phase's frame.
use super::*;
use crate::dto::{import_control::*, pool_import::PoolImportIdentity};

#[test]
fn import_steps_preserve_identity_and_reject_phase_substitution() {
    let identity = PoolImportIdentity {
        sequence: u64::MAX,
        plan_sha256: [7; 32],
    };
    let request = AdvanceRequest {
        identity,
        canister_id: ic_platform::Principal::from_slice(&[4; 29]),
    };
    let bytes =
        ic_wire::native::encode_request(advance::METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<AdvanceRequest>(
            advance::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        )
        .unwrap(),
        request
    );
    assert!(matches!(
        ic_wire::native::decode_request::<AdvanceRequest>(
            release::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
    let request = ReleaseRequest {
        identity,
        publication_sha256: [8; 32],
    };
    let bytes =
        ic_wire::native::encode_request(release::METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<ReleaseRequest>(
            release::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        )
        .unwrap(),
        request
    );
    let bytes =
        ic_wire::native::encode_request(settle::METHOD_ID, &identity, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<PoolImportIdentity>(
            settle::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        )
        .unwrap(),
        identity
    );
}

#[cfg(feature = "wire-schema")]
#[test]
fn all_import_phases_have_complete_native_metadata() {
    ic_wire::typescript::native_schema::<crate::dto::pool_census::Reservation>().unwrap();
    ic_wire::typescript::native_schema::<AdvanceRequest>().unwrap();
    ic_wire::typescript::native_schema::<PoolImportIdentity>().unwrap();
    ic_wire::typescript::native_schema::<ReleaseRequest>().unwrap();
    ic_wire::typescript::native_schema::<
        Result<crate::dto::pool_census::ImportStatus, crate::dto::error::Error>,
    >()
    .unwrap();
}
