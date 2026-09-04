use super::*;
use canic_core::cdk::types::Cycles;

#[derive(CandidType)]
struct IncompletePredecessorPoolResponse {
    config: FleetSubnetCanisterPoolConfig,
}

#[derive(CandidType)]
enum IncompletePredecessorRootStatusResponse {
    Pool(Box<IncompletePredecessorPoolResponse>),
}

#[test]
fn exact_projection_normalizes_only_the_absent_recovery_count() {
    let current = CanisterPoolResponse {
        config: FleetSubnetCanisterPoolConfig {
            minimum_size: 1,
            maximum_size: 2,
            canister_cycles: Cycles::new(5_000_000_000_000),
            creation_execution_margin: Cycles::new(1_000_000_000_000),
        },
        tracked: 0,
        store: 0,
        store_deletion_pending: 0,
        pooled: 0,
        workload: 0,
        surplus: 0,
        ready: 0,
        pending_reset: 0,
        claimed: 0,
        recycling: 0,
        handing_off: 0,
        failed: 0,
        completed_handoffs: 0,
        pending_creation: None,
        pending_handoff: None,
        entries: Vec::new(),
        next_start_after: None,
    };
    let predecessor =
        PredecessorRootStatusResponse::Pool(Box::new(predecessor_from_current(&current)));
    let bytes = candid::encode_one(Ok::<_, canic_core::dto::error::Error>(predecessor))
        .expect("encode exact predecessor response");
    assert_eq!(
        decode_pool_response(&bytes).expect("decode predecessor response"),
        current
    );

    let incomplete = IncompletePredecessorRootStatusResponse::Pool(Box::new(
        IncompletePredecessorPoolResponse {
            config: current.config,
        },
    ));
    let bytes = candid::encode_one(Ok::<_, canic_core::dto::error::Error>(incomplete))
        .expect("encode incomplete predecessor response");
    assert!(matches!(
        decode_pool_response(&bytes),
        Err(PredecessorRootStatusError::Decode(_))
    ));
}
