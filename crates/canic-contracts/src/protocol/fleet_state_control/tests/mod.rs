//! Exact mutation inputs and bounded cascade observations retain their meaning across transport.
use super::*;
use crate::dto::{
    cascade::{StateCascadeReport, StateCascadeTargetOutcome, StateCascadeTargetResult},
    error::Error,
    fleet_state_control::CommandResult,
    state::{
        FleetStateCommandResult, FleetStatus, SetCyclesFundingRequest, SetFleetStatusRequest,
        SetStateResponse,
    },
};
use ic_platform::principal::ExternalPrincipal;

#[test]
fn cascade_receipt_preserves_partial_failures_and_omissions() {
    let error = Error::from_registered(crate::diagnostics::codes::AUTHORITY_INVALID_STATE);
    let original = FleetStateCommandResult {
        change: SetStateResponse {
            previous: true,
            current: false,
            changed: true,
        },
        propagation: StateCascadeReport {
            successful_targets: 1,
            reconciliation_failures: 1,
            unconfirmed_targets: 2,
            omitted_targets: 1,
            targets: vec![
                StateCascadeTargetOutcome {
                    canister_id: ExternalPrincipal::from_slice(&[1; 29]),
                    result: StateCascadeTargetResult::Applied,
                },
                StateCascadeTargetOutcome {
                    canister_id: ExternalPrincipal::from_slice(&[2; 29]),
                    result: StateCascadeTargetResult::AppliedWithReconciliationFailure(error),
                },
                StateCascadeTargetOutcome {
                    canister_id: ExternalPrincipal::from_slice(&[3; 29]),
                    result: StateCascadeTargetResult::Unconfirmed(error),
                },
            ],
        },
        reconciliation_error: Some(error),
    };
    let reply = Ok::<_, Error>(CommandResult::from(original.clone()));
    let bytes =
        ic_wire::native::encode_response(funding::METHOD_ID, &reply, RESPONSE_LIMITS).unwrap();
    let decoded: Result<CommandResult<bool>, Error> =
        ic_wire::native::decode_response(funding::METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    assert_eq!(FleetStateCommandResult::from(decoded.unwrap()), original);
}

#[test]
fn state_inputs_retain_each_status_and_strict_method_identity() {
    for status in [
        FleetStatus::Active,
        FleetStatus::Readonly,
        FleetStatus::Stopped,
    ] {
        let request = SetFleetStatusRequest { status };
        let bytes =
            ic_wire::native::encode_request(status::METHOD_ID, &request, REQUEST_LIMITS).unwrap();
        let decoded: SetFleetStatusRequest =
            ic_wire::native::decode_request(status::METHOD_ID, &bytes, REQUEST_LIMITS).unwrap();
        assert_eq!(decoded, request);
        assert!(matches!(
            ic_wire::native::decode_request::<SetFleetStatusRequest>(
                funding::METHOD_ID,
                &bytes,
                REQUEST_LIMITS
            ),
            Err(ic_wire::Error::Identity)
        ));
        let mut trailing = bytes;
        trailing.push(0);
        assert!(matches!(
            ic_wire::native::decode_request::<SetFleetStatusRequest>(
                status::METHOD_ID,
                &trailing,
                REQUEST_LIMITS
            ),
            Err(ic_wire::Error::Trailing)
        ));
    }
    for enabled in [false, true] {
        let request = SetCyclesFundingRequest { enabled };
        let bytes =
            ic_wire::native::encode_request(funding::METHOD_ID, &request, REQUEST_LIMITS).unwrap();
        let decoded: SetCyclesFundingRequest =
            ic_wire::native::decode_request(funding::METHOD_ID, &bytes, REQUEST_LIMITS).unwrap();
        assert_eq!(decoded, request);
    }
}

#[cfg(feature = "wire-schema")]
#[test]
fn mutation_schemas_include_full_cascade_evidence() {
    ic_wire::typescript::native_schema::<SetCyclesFundingRequest>().unwrap();
    ic_wire::typescript::native_schema::<SetFleetStatusRequest>().unwrap();
    ic_wire::typescript::native_schema::<Result<CommandResult<bool>, Error>>().unwrap();
    ic_wire::typescript::native_schema::<Result<CommandResult<FleetStatus>, Error>>().unwrap();
}
