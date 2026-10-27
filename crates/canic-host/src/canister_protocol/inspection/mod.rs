//! Require fresh, caller-bound reserve evidence before a protected inspection.
//! The query is indicative; actual outbound admission remains owned by IC.
use crate::{
    canister_protocol::CanisterProtocolError,
    icp::{IcpCli, IcpJsonResponseError},
};
use canic_contracts::{
    dto::{
        canister::CanisterInspectionReserveResponse, error::Error,
        inspection_reserve::InspectionReserve,
    },
    protocol::inspection_reserve as protocol,
};
use ic_platform::principal::ExternalPrincipal as Principal;

/// Query the maintained Root endpoint; unavailable evidence never permits a paid inspection.
pub fn preflight_inspection(
    icp: &IcpCli,
    root: Principal,
    target: Principal,
) -> Result<(), CanisterProtocolError> {
    let target_native: ic_platform::Principal = target.into();
    let evidence: Result<InspectionReserve, Error> = icp
        .canister_query_wire(
            &root.to_text(),
            protocol::METHOD,
            protocol::METHOD_ID,
            &target_native,
            protocol::REQUEST_LIMITS,
            protocol::RESPONSE_LIMITS,
        )
        .map_err(|source| CanisterProtocolError::WireQuery {
            canister: root,
            method: protocol::METHOD,
            source,
        })?;
    let evidence = evidence.map_err(|error| CanisterProtocolError::Response {
        canister: root,
        method: protocol::METHOD,
        source: IcpJsonResponseError::Rejected(error),
    })?;
    validate_inspection_reserve(root, target, evidence.into())
}

/// Bind reserve evidence to the exact Root and target before any paid inspection.
pub fn validate_inspection_reserve(
    root: Principal,
    target: Principal,
    evidence: CanisterInspectionReserveResponse,
) -> Result<(), CanisterProtocolError> {
    if evidence.caller != root
        || evidence.canister_id != target
        || evidence.available_liquid_cycles > evidence.native_cycles
    {
        return Err(CanisterProtocolError::InvalidInspectionReserve {
            caller: root,
            target,
        });
    }
    if evidence.available_liquid_cycles < evidence.required_liquid_cycles {
        return Err(CanisterProtocolError::InspectionPreflightReserve(Box::new(
            evidence,
        )));
    }
    Ok(())
}

/// Invoke one exact native observation; callers retain ownership of reserve preflight.
pub fn inspect_outcome(
    icp: &IcpCli,
    root: Principal,
    target: Principal,
) -> Result<canic_contracts::dto::canister::CanisterInspectionOutcome, CanisterProtocolError> {
    use canic_contracts::{dto::inspection, protocol::inspection as wire};
    let response: inspection::Response = super::call_wire(
        icp,
        root,
        wire::METHOD,
        wire::METHOD_ID,
        &inspection::Request {
            canister_id: target.into(),
        },
        wire::REQUEST_LIMITS,
        wire::RESPONSE_LIMITS,
    )?;
    response
        .try_into()
        .map_err(|source| CanisterProtocolError::WireUpdate {
            canister: root,
            method: wire::METHOD,
            source: crate::icp::IcpWireQueryError::Wire(source),
        })
}

/// Preserve reserve refusals as exact Root/target-bound errors after lossless decoding.
pub fn inspect_status(
    icp: &IcpCli,
    root: Principal,
    target: Principal,
) -> Result<canic_contracts::dto::canister::CanisterStatusResponse, CanisterProtocolError> {
    use canic_contracts::dto::canister::CanisterInspectionOutcome;
    match inspect_outcome(icp, root, target)? {
        CanisterInspectionOutcome::Status(status) => Ok(*status),
        CanisterInspectionOutcome::ReserveRequired(evidence) => Err(
            CanisterProtocolError::inspection_reserve(root, target, evidence),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inspection_preflight_binds_authority_and_rejects_invalid_or_insufficient_samples() {
        let root = Principal::from_slice(&[1]);
        let target = Principal::from_slice(&[2]);
        let sample = CanisterInspectionReserveResponse {
            caller: root,
            canister_id: target,
            native_cycles: 1000,
            available_liquid_cycles: 100,
            required_liquid_cycles: 100,
        };
        assert!(validate_inspection_reserve(root, target, sample.clone()).is_ok());
        // Admission owns the cost schedule; the host must not invent a minimum fee.
        assert!(
            validate_inspection_reserve(
                root,
                target,
                CanisterInspectionReserveResponse {
                    available_liquid_cycles: 0,
                    required_liquid_cycles: 0,
                    ..sample
                }
            )
            .is_ok()
        );
        let shortfall = CanisterInspectionReserveResponse {
            available_liquid_cycles: 99,
            ..sample
        };
        let error = validate_inspection_reserve(root, target, shortfall.clone()).unwrap_err();
        assert!(!error.is_rejected_with(
            canic_contracts::diagnostics::codes::PLATFORM_INSUFFICIENT_LIQUID_CYCLES
        ));
        let CanisterProtocolError::InspectionPreflightReserve(actual) = error else {
            panic!("preflight shortfall is distinct from SDK admission failure");
        };
        assert_eq!(*actual, shortfall);
        for invalid in [
            CanisterInspectionReserveResponse {
                caller: target,
                ..sample
            },
            CanisterInspectionReserveResponse {
                canister_id: root,
                ..sample
            },
            CanisterInspectionReserveResponse {
                native_cycles: 99,
                ..sample
            },
        ] {
            assert!(matches!(
                validate_inspection_reserve(root, target, invalid),
                Err(CanisterProtocolError::InvalidInspectionReserve { .. })
            ));
        }
    }
}
