//! Native child identity, pagination and bounded framing.

use super::*;
use crate::dto::{
    canister::CanisterInfo,
    error::Error,
    page::{Page, PageRequest},
};

fn child() -> CanisterInfo {
    CanisterInfo {
        pid: ic_platform::Principal::from_slice(&[7; 29]),
        role: "worker".into(),
        parent_pid: Some(ic_platform::Principal::from_slice(&[8; 29])),
        module_hash: Some(vec![9; 32]),
        created_at: u64::MAX,
    }
}

#[test]
fn child_page_preserves_native_identity_and_pagination() {
    let request = PageRequest {
        limit: 1000,
        offset: 42,
    };
    let bytes = ic_wire::native::encode_request(METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<PageRequest>(METHOD_ID, &bytes, REQUEST_LIMITS).unwrap(),
        request
    );
    let response: Result<_, Error> = Ok(Page {
        entries: vec![child(); 1000],
        total: 2042,
    });
    let bytes = ic_wire::native::encode_response(METHOD_ID, &response, RESPONSE_LIMITS).unwrap();
    let actual: Result<Page<CanisterInfo>, Error> =
        ic_wire::native::decode_response(METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    let actual = actual.unwrap();
    assert_eq!(actual.total, 2042);
    assert_eq!(actual.entries, vec![child(); 1000]);
    assert!(matches!(
        ic_wire::native::decode_response::<Result<Page<CanisterInfo>, Error>>(
            METHOD_ID + 1,
            &bytes,
            RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[test]
fn child_page_refuses_oversized_payloads() {
    let mut value = child();
    value.module_hash = Some(vec![0; RESPONSE_LIMITS.bytes]);
    let response: Result<_, Error> = Ok(Page {
        entries: vec![value],
        total: 1,
    });
    assert!(matches!(
        ic_wire::native::encode_response(METHOD_ID, &response, RESPONSE_LIMITS),
        Err(ic_wire::Error::Limit)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn child_schema_uses_native_principals() {
    let schema = ic_wire::typescript::native_schema::<CanisterInfo>().unwrap();
    assert_eq!(schema["fields"][0]["name"], "pid");
    assert_eq!(
        schema["fields"][0]["value"],
        ic_wire::typescript::native_schema::<ic_platform::Principal>().unwrap()
    );
    ic_wire::typescript::native_schema::<Result<Page<CanisterInfo>, Error>>().unwrap();
}
