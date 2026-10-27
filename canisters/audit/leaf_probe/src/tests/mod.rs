//! The frontend probes' declared budgets admit their actual native result shapes.

use super::*;

#[test]
fn frontend_wire_limits_cover_principal_results() {
    for id in [FRONTEND_PROBE_ID, FRONTEND_UPDATE_PROBE_ID] {
        let request = ic_wire::native::encode_request(id, &(), FRONTEND_PROBE_LIMITS).unwrap();
        ic_wire::native::decode_request::<()>(id, &request, FRONTEND_PROBE_LIMITS).unwrap();
        for principal in [
            ic_platform::Principal::management_canister(),
            ic_platform::Principal::anonymous(),
            ic_platform::Principal::from_slice(&[9; 29]),
        ] {
            let response: Result<ic_platform::Principal, Error> = Ok(principal);
            let bytes =
                ic_wire::native::encode_response(id, &response, FRONTEND_PROBE_LIMITS).unwrap();
            let decoded =
                ic_wire::native::decode_response::<Result<ic_platform::Principal, Error>>(
                    id,
                    &bytes,
                    FRONTEND_PROBE_LIMITS,
                )
                .unwrap();
            assert_eq!(decoded, response);
        }
    }
}

#[test]
fn cohort_native_frames_preserve_each_selected_slot_and_value() {
    for slot in 1..=5_u8 {
        let request = protocol::CohortRequest {
            slot,
            value: 0x1234_5678,
        };
        let bytes =
            ic_wire::native::encode_request(protocol::COHORT, &request, protocol::REQUEST_LIMITS)
                .unwrap();
        let decoded =
            ic_wire::native::decode_request(protocol::COHORT, &bytes, protocol::REQUEST_LIMITS)
                .unwrap();
        let response: Result<GenericPageCohortResponse, Error> = Ok(cohort_response(decoded));
        let bytes = ic_wire::native::encode_response(
            protocol::COHORT,
            &response,
            protocol::RESPONSE_LIMITS,
        )
        .unwrap();
        let decoded: Result<GenericPageCohortResponse, Error> =
            ic_wire::native::decode_response(protocol::COHORT, &bytes, protocol::RESPONSE_LIMITS)
                .unwrap();
        let (selected, total, values) = match decoded.unwrap() {
            GenericPageCohortResponse::Slot1(page) => (
                1,
                page.total,
                page.entries
                    .into_iter()
                    .map(|row| row.value)
                    .collect::<Vec<_>>(),
            ),
            GenericPageCohortResponse::Slot2(page) => (
                2,
                page.total,
                page.entries
                    .into_iter()
                    .map(|row| row.value)
                    .collect::<Vec<_>>(),
            ),
            GenericPageCohortResponse::Slot3(page) => (
                3,
                page.total,
                page.entries
                    .into_iter()
                    .map(|row| row.value)
                    .collect::<Vec<_>>(),
            ),
            GenericPageCohortResponse::Slot4(page) => (
                4,
                page.total,
                page.entries
                    .into_iter()
                    .map(|row| row.value)
                    .collect::<Vec<_>>(),
            ),
            GenericPageCohortResponse::Slot5(page) => (
                5,
                page.total,
                page.entries
                    .into_iter()
                    .map(|row| row.value)
                    .collect::<Vec<_>>(),
            ),
        };
        assert_eq!(selected, slot);
        assert_eq!(total, 1);
        assert_eq!(values, [request.value]);
    }
}
