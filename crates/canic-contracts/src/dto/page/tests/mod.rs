//! Qualify native pagination bytes and metadata from the actual public DTOs.

use super::{Page, PageRequest};
use ic_wire::{Error, Limits, native, typescript::native_schema};
use serde_json::json;

const LIMITS: Limits = Limits {
    bytes: 1024,
    allocation: 8192,
    nodes: 128,
    depth: 16,
};

#[test]
fn native_pagination_metadata_matches_serde_declaration_order() {
    assert_eq!(
        native_schema::<Page<String>>().unwrap(),
        json!({"k":"record","fields":[
            {"name":"entries","value":{"k":"list","item":{"k":"text"}}},
            {"name":"total","value":{"k":"u","bits":64}}
        ]})
    );
    assert_eq!(
        native_schema::<PageRequest>().unwrap(),
        json!({"k":"record","fields":[
            {"name":"limit","value":{"k":"u","bits":64}},
            {"name":"offset","value":{"k":"u","bits":64}}
        ]})
    );
    let request = PageRequest {
        limit: 128,
        offset: 1,
    };
    assert_eq!(native::encode(&request, LIMITS).unwrap(), [128, 1, 1]);
    let response = Page {
        entries: vec!["a".to_owned()],
        total: 2,
    };
    assert_eq!(native::encode(&response, LIMITS).unwrap(), [1, 1, b'a', 2]);
}

#[test]
fn native_pagination_frames_enforce_identity_direction_and_limits() {
    // A test-only method ID; production method IDs remain endpoint-owned.
    let frame = native::encode_response(
        7,
        &Page {
            entries: vec!["a".to_owned()],
            total: 2,
        },
        LIMITS,
    )
    .unwrap();
    let decoded: Page<String> = native::decode_response(7, &frame, LIMITS).unwrap();
    assert_eq!(decoded.entries, ["a"]);
    assert_eq!(decoded.total, 2);
    assert!(matches!(
        native::decode_response::<Page<String>>(8, &frame, LIMITS),
        Err(Error::Identity)
    ));
    assert!(matches!(
        native::decode_request::<Page<String>>(7, &frame, LIMITS),
        Err(Error::Identity)
    ));
    assert!(matches!(
        native::decode_response::<Page<String>>(
            7,
            &frame,
            Limits {
                bytes: frame.len() - 1,
                ..LIMITS
            }
        ),
        Err(Error::Limit)
    ));
    let mut trailing = frame;
    trailing.push(0);
    assert!(native::decode_response::<Page<String>>(7, &trailing, LIMITS).is_err());
}

#[test]
fn native_principal_pages_match_metadata_and_reject_oversized_identities() {
    use ic_platform::Principal;

    let schema = native_schema::<Page<Principal>>().unwrap();
    assert_eq!(
        schema["fields"][0]["value"]["item"],
        native_schema::<ic_wire::Bytes<29>>().unwrap()
    );
    let page = Page {
        entries: vec![
            Principal::management_canister(),
            Principal::anonymous(),
            Principal::from_slice(&[42; 29]),
        ],
        total: 3,
    };
    let frame = native::encode_response(18, &page, LIMITS).unwrap();
    let decoded: Page<Principal> = native::decode_response(18, &frame, LIMITS).unwrap();
    assert_eq!(decoded.entries, page.entries);
    assert_eq!(decoded.total, page.total);
    let invalid = Page {
        entries: vec![ic_wire::Bytes::<30>::new(vec![0; 30]).unwrap()],
        total: 1,
    };
    let invalid_frame = native::encode_response(18, &invalid, LIMITS).unwrap();
    assert!(native::decode_response::<Page<Principal>>(18, &invalid_frame, LIMITS).is_err());
}
