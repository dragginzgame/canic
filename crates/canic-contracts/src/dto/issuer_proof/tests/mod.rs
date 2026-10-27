//! Native public status keeps original authority identities and proof deadlines.
use super::*;
use crate::{dto::auth as stored, protocol::issuer_proof::status as wire};
use ic_platform::principal::ExternalPrincipal;

#[test]
fn status_frames_preserve_all_states_and_optional_evidence() {
    for status in [
        stored::ActiveDelegationProofStatus::Missing,
        stored::ActiveDelegationProofStatus::Valid,
        stored::ActiveDelegationProofStatus::RefreshNeeded,
        stored::ActiveDelegationProofStatus::Expired,
    ] {
        for populated in [false, true] {
            let original = stored::ActiveDelegationProofStatusResponse {
                status: status.clone(),
                root_pid: populated.then(|| ExternalPrincipal::from_slice(&[1; 29])),
                issuer_pid: populated.then(|| ExternalPrincipal::from_slice(&[2; 29])),
                cert_hash: populated.then_some([3; 32]),
                expires_at_ns: populated.then_some(u64::MAX),
                refresh_after_ns: populated.then_some(u64::MAX - 1),
            };
            let frame = ic_wire::native::encode_response(
                wire::METHOD_ID,
                &Ok::<_, crate::dto::error::Error>(ActiveDelegationProofStatusResponse::from(
                    original.clone(),
                )),
                wire::RESPONSE_LIMITS,
            )
            .unwrap();
            let decoded: Result<ActiveDelegationProofStatusResponse, crate::dto::error::Error> =
                ic_wire::native::decode_response(wire::METHOD_ID, &frame, wire::RESPONSE_LIMITS)
                    .unwrap();
            assert_eq!(
                stored::ActiveDelegationProofStatusResponse::from(decoded.unwrap()),
                original
            );
        }
    }
}

#[test]
fn status_unit_request_requires_exact_frame_identity_and_length() {
    let frame =
        ic_wire::native::encode_request(wire::METHOD_ID, &(), wire::REQUEST_LIMITS).unwrap();
    ic_wire::native::decode_request::<()>(wire::METHOD_ID, &frame, wire::REQUEST_LIMITS).unwrap();
    assert!(matches!(
        ic_wire::native::decode_request::<()>(wire::METHOD_ID + 1, &frame, wire::REQUEST_LIMITS),
        Err(ic_wire::Error::Identity)
    ));
    let mut trailing = frame;
    trailing.push(0);
    assert!(matches!(
        ic_wire::native::decode_request::<()>(wire::METHOD_ID, &trailing, wire::REQUEST_LIMITS),
        Err(ic_wire::Error::Trailing)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn status_schema_contains_the_complete_native_evidence() {
    ic_wire::typescript::native_schema::<()>().unwrap();
    ic_wire::typescript::native_schema::<
        Result<ActiveDelegationProofStatusResponse, crate::dto::error::Error>,
    >()
    .unwrap();
}
