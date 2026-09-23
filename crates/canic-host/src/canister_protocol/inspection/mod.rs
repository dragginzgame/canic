//! Module: canister_protocol::inspection
//!
//! Responsibility: reject an observed Root reserve shortfall before a protected inspection.
//! Does not own: funding authority, retry, target discovery, or balance caching.
//! Boundary: query through the caller's exact Candid binding; actual call admission remains on IC.

use crate::{
    canister_protocol::{CanisterProtocolError, query_with_candid},
    icp::IcpCli,
};
use candid::{
    CandidType, Deserialize, Principal,
    types::{FuncMode, TypeInner},
};
use candid_parser::utils::CandidSource;
use canic_core::{
    dto::canister::{CanisterInspectionRequest, CanisterInspectionReserveResponse},
    protocol,
};
use std::{fs, path::Path};

/// Controller-owned observation request for the exact next inspection target.
#[derive(CandidType)]
pub enum InspectionReserveRequest {
    InspectionReserve(CanisterInspectionRequest),
}

/// Indicative reserve evidence; this query cannot authorize a paid effect.
#[derive(CandidType, Deserialize)]
pub enum InspectionReserveResponse {
    InspectionReserve(CanisterInspectionReserveResponse),
}

/// Query afresh when the exact bound contract exposes the reserve selector.
///
/// Retained source evidence may expose only the protected inspection itself. Its
/// absence of a quote is not evidence of sufficient reserve. A declared query's
/// failure always propagates; it never permits bypassing preflight.
pub fn preflight_inspection(
    icp: &IcpCli,
    candid_path: &Path,
    root: Principal,
    target: Principal,
) -> Result<(), CanisterProtocolError> {
    let contract = fs::read_to_string(candid_path).map_err(|error| {
        CanisterProtocolError::InspectionContract {
            caller: root,
            detail: error.to_string(),
        }
    })?;
    if !declares_inspection_reserve(&contract).map_err(|detail| {
        CanisterProtocolError::InspectionContract {
            caller: root,
            detail,
        }
    })? {
        return Ok(());
    }
    let InspectionReserveResponse::InspectionReserve(evidence) = query_with_candid(
        icp,
        candid_path,
        root,
        protocol::CANIC_OBSERVABILITY,
        &InspectionReserveRequest::InspectionReserve(CanisterInspectionRequest {
            canister_id: target,
        }),
    )?;
    validate_inspection_reserve(root, target, evidence)
}

fn declares_inspection_reserve(contract: &str) -> Result<bool, String> {
    let (environment, actor) = CandidSource::Text(contract)
        .load()
        .map_err(|error| error.to_string())?;
    let actor = actor.ok_or_else(|| "missing service declaration".to_string())?;
    let service = environment
        .as_service(&actor)
        .map_err(|error| error.to_string())?;
    let Some((_, method)) = service
        .iter()
        .find(|(name, _)| name == protocol::CANIC_OBSERVABILITY)
    else {
        return Ok(false);
    };
    let function = environment
        .as_func(method)
        .map_err(|error| error.to_string())?;
    let [argument] = function.args.as_slice() else {
        return Err("observability must accept one request variant".to_string());
    };
    let argument = environment
        .trace_type(argument)
        .map_err(|error| error.to_string())?;
    let TypeInner::Variant(fields) = argument.as_ref() else {
        return Err("observability must accept one request variant".to_string());
    };
    let Some(field) = fields
        .iter()
        .find(|field| field.id.get_id() == candid::idl_hash("InspectionReserve"))
    else {
        return Ok(false);
    };
    if function.modes != [FuncMode::Query] {
        return Err("inspection reserve must be a query".to_string());
    }
    candid::types::subtype::equal(
        &mut candid::types::subtype::Gamma::new(),
        &environment,
        &field.ty,
        &CanisterInspectionRequest::ty(),
    )
    .map_err(|error| format!("inspection reserve target contract: {error}"))?;
    Ok(true)
}

fn validate_inspection_reserve(
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "opt-in local parsing measurement using an exact retained Candid artifact"]
    fn retained_contract_parsing_measurement() {
        let path = std::env::var_os("CANIC_BENCH_INSPECTION_CANDID")
            .expect("set CANIC_BENCH_INSPECTION_CANDID to a retained Root Candid file");
        let source = fs::read_to_string(&path).unwrap();
        let expected = declares_inspection_reserve(&source).unwrap();
        let mut samples = Vec::new();
        for _ in 0..5 {
            let started = std::time::Instant::now();
            for _ in 0..33 {
                let source = fs::read_to_string(&path).unwrap();
                assert_eq!(
                    declares_inspection_reserve(std::hint::black_box(&source)).unwrap(),
                    expected
                );
            }
            samples.push(started.elapsed().as_micros());
        }
        eprintln!(
            "[CANIC-INSPECTION-PARSE] {}",
            serde_json::json!({
                "source_sha256": canic_core::cdk::utils::hash::sha256_hex(source.as_bytes()),
                "source_bytes": source.len(),
                "inspections_per_sample": 33,
                "uncached_micros": samples,
            })
        );
    }

    #[test]
    fn inspection_preflight_uses_the_bound_selector_and_resolves_type_aliases() {
        let contract = r"
            type Target = principal;
            type Request = record { canister_id : Target };
            type Observation = variant { InspectionReserve : Request; CycleBalance };
            service : { canic_observability : (Observation) -> () query }
        ";
        assert_eq!(declares_inspection_reserve(contract), Ok(true));
        assert_eq!(
            declares_inspection_reserve(
                "service : { canic_observability : (variant { CycleBalance }) -> () query }"
            ),
            Ok(false)
        );
        // A name in unrelated declarations or explanatory text grants no capability.
        assert_eq!(
            declares_inspection_reserve(
                "type InspectionReserve = nat; service : { status : () -> () query }"
            ),
            Ok(false)
        );
        for invalid in [
            "invalid Candid".to_string(),
            "type Request = principal;".to_string(),
            contract.replace(" query", ""),
            contract.replace("Target = principal", "Target = text"),
            contract.replace("(Observation)", "(principal)"),
            contract.replace("(Observation)", "()"),
        ] {
            assert!(declares_inspection_reserve(&invalid).is_err());
        }
    }

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
        assert!(
            !error.is_rejected_with(
                canic_core::diagnostics::codes::PLATFORM_INSUFFICIENT_LIQUID_CYCLES
            )
        );
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
