use super::*;
use canic_core::dto::canister::CanisterInspectionReserveResponse;

#[test]
fn funding_inspection_reserve_requires_exact_call_participants_and_native_bounds() {
    let root = Principal::from_slice(&[1; 29]);
    let child = Principal::from_slice(&[2; 29]);
    let evidence = CanisterInspectionReserveResponse {
        caller: root,
        canister_id: child,
        native_cycles: 20,
        available_liquid_cycles: 10,
        required_liquid_cycles: 30,
    };
    assert!(matches!(
        inspection_response(
            Response::InspectionReserveRequired(evidence.clone()),
            root,
            child
        ),
        Err(FundingObservationError::Underfunded)
    ));
    for evidence in [
        CanisterInspectionReserveResponse {
            caller: child,
            ..evidence.clone()
        },
        CanisterInspectionReserveResponse {
            canister_id: root,
            ..evidence.clone()
        },
        CanisterInspectionReserveResponse {
            available_liquid_cycles: 21,
            ..evidence
        },
    ] {
        assert!(matches!(
            inspection_response(Response::InspectionReserveRequired(evidence), root, child),
            Err(FundingObservationError::AuthorityMismatch)
        ));
    }
}
