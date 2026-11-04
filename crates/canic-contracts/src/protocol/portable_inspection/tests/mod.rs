//! Owned Wire identities bound inspection requests and typed outcomes.
use super::*;
use crate::dto::portable_inspection::{Page, Refusal, Request};

#[test]
fn inspection_request_and_refusal_use_their_own_wire_identity() {
    let request = Request {
        after: Some(ic_platform_types::Principal::from_slice(&[7; 29])),
        limit: MAX_PAGE,
    };
    let bytes = ic_wire::native::encode_request(METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<Request>(METHOD_ID, &bytes, REQUEST_LIMITS).unwrap(),
        request
    );
    assert!(
        ic_wire::native::decode_request::<Request>(
            crate::protocol::portable_enrollment::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        )
        .is_err()
    );
    let response: Result<Page, Refusal> = Err(Refusal::Unavailable);
    let bytes = ic_wire::native::encode_response(METHOD_ID, &response, RESPONSE_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_response::<Result<Page, Refusal>>(
            METHOD_ID,
            &bytes,
            RESPONSE_LIMITS
        )
        .unwrap(),
        response
    );
}

#[test]
fn operator_hop_and_policy_contracts_preserve_separate_authority_and_refusals() {
    use crate::{
        dto::{
            portable_inspection::{PolicyReport, RuntimeState},
            root_observation::Request as RelayRequest,
        },
        protocol::root_observation::{portable_policy, portable_sessions},
    };
    let request = RelayRequest {
        canister_id: ic_platform_types::Principal::from_slice(&[8]),
        input: Request {
            after: None,
            limit: MAX_PAGE,
        },
    };
    let bytes = ic_wire::native::encode_request(
        portable_sessions::METHOD_ID,
        &request,
        portable_sessions::REQUEST_LIMITS,
    )
    .unwrap();
    assert!(ic_wire::native::decode_request::<Request>(METHOD_ID, &bytes, REQUEST_LIMITS).is_err());
    let response: Result<Result<PolicyReport, Refusal>, crate::dto::error::Error> =
        Ok(Ok(PolicyReport {
            observed_at_ns: 100,
            authority_generation: 2,
            declared: None,
            runtime: RuntimeState::Disabled,
        }));
    let bytes = ic_wire::native::encode_response(
        portable_policy::METHOD_ID,
        &response,
        portable_policy::RESPONSE_LIMITS,
    )
    .unwrap();
    let reply: Result<Result<PolicyReport, Refusal>, crate::dto::error::Error> =
        ic_wire::native::decode_response(
            portable_policy::METHOD_ID,
            &bytes,
            portable_policy::RESPONSE_LIMITS,
        )
        .unwrap();
    assert_eq!(reply.unwrap().unwrap().runtime, RuntimeState::Disabled);
}
