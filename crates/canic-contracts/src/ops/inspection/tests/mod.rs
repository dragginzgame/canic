//! Whole-observation conversion preserves canonical external evidence and typed refusal.
use super::*;
use crate::protocol::authority::encode;
use ic_platform::candid::contract::Nat;
use num_bigint::BigUint;

#[test]
fn complete_inspection_preserves_overflow_evidence_and_external_encoding() {
    let overflow = Nat(BigUint::from(1_u8) << 256);
    let original = external::CanisterStatusResponse {
        version: u64::MAX,
        status: external::CanisterStatusType::Running,
        settings: external::CanisterSettings {
            controllers: vec![ic_platform::principal::ExternalPrincipal::from_slice(
                &[7; 29],
            )],
            compute_allocation: overflow.clone(),
            memory_allocation: overflow.clone(),
            freezing_threshold: overflow.clone(),
            reserved_cycles_limit: overflow.clone(),
            log_visibility: external::LogVisibility::AllowedViewers(vec![
                ic_platform::principal::ExternalPrincipal::anonymous(),
            ]),
            log_memory_limit: overflow.clone(),
            wasm_memory_limit: overflow.clone(),
            wasm_memory_threshold: overflow.clone(),
            environment_variables: vec![external::EnvironmentVariable {
                name: "name".into(),
                value: "value".into(),
            }],
        },
        module_hash: Some(vec![4; 32]),
        memory_size: overflow.clone(),
        memory_metrics: external::MemoryMetrics {
            wasm_memory_size: overflow.clone(),
            stable_memory_size: overflow.clone(),
            global_memory_size: overflow.clone(),
            wasm_binary_size: overflow.clone(),
            custom_sections_size: overflow.clone(),
            canister_history_size: overflow.clone(),
            wasm_chunk_store_size: overflow.clone(),
            snapshots_size: overflow.clone(),
        },
        cycles: overflow.clone(),
        reserved_cycles: overflow.clone(),
        idle_cycles_burned_per_day: overflow.clone(),
        query_stats: external::QueryStats {
            num_calls_total: overflow.clone(),
            num_instructions_total: overflow.clone(),
            request_payload_bytes_total: overflow.clone(),
            response_payload_bytes_total: overflow,
        },
    };
    let canonical = encode(&original).unwrap();
    let native = wire::Status::try_from(original).unwrap();
    assert_eq!(native.cycles.as_slice().len(), 33);
    let limits = ic_wire::Limits {
        bytes: 8192,
        allocation: 65536,
        nodes: 2048,
        depth: 32,
    };
    let frame = ic_wire::native::encode_response(73, &native, limits).unwrap();
    let decoded: wire::Status = ic_wire::native::decode_response(73, &frame, limits).unwrap();
    let restored = external::CanisterStatusResponse::try_from(decoded).unwrap();
    assert!(u128::try_from(restored.cycles.0.clone()).is_err());
    assert_eq!(encode(restored).unwrap(), canonical);
    let mut invalid = native;
    invalid.cycles = ic_wire::Bytes::new(vec![0, 1]).unwrap();
    assert!(matches!(
        external::CanisterStatusResponse::try_from(invalid),
        Err(Error::Malformed)
    ));
}

#[test]
fn reserve_refusal_keeps_exact_caller_target_and_amounts() {
    let original = external::CanisterInspectionReserveResponse {
        caller: ic_platform::principal::ExternalPrincipal::from_slice(&[1; 29]),
        canister_id: ic_platform::principal::ExternalPrincipal::from_slice(&[2; 29]),
        native_cycles: u128::MAX,
        available_liquid_cycles: 4,
        required_liquid_cycles: 5,
    };
    let native = wire::Response::try_from(external::CanisterInspectionOutcome::ReserveRequired(
        original.clone(),
    ))
    .unwrap();
    let restored = external::CanisterInspectionOutcome::try_from(native).unwrap();
    let external::CanisterInspectionOutcome::ReserveRequired(evidence) = restored else {
        panic!("reserve refusal became status")
    };
    assert_eq!(evidence, original);
}

#[cfg(feature = "wire-schema")]
#[test]
fn complete_inspection_graph_has_native_metadata() {
    ic_wire::typescript::native_schema::<wire::Request>().unwrap();
    ic_wire::typescript::native_schema::<Result<wire::Response, crate::dto::error::Error>>()
        .unwrap();
}
