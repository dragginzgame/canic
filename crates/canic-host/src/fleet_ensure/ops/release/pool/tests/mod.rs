//! Pool evidence preserves exact history and rejects unbounded or foreign observations.

use super::*;
use crate::fleet_ensure::{
    view::release::{
        FleetReleasePoolView,
        pool::{ReleasePoolCreationDisposition, ReleasePoolImportDisposition},
    },
    workflow::release::assess_pools,
};
use candid::CandidType;
use canic_contracts::{
    cycles::Cycles,
    dto::{
        pool::{
            CanisterPoolCreation, CanisterPoolCreationFailure, CanisterPoolCreationProgress,
            CanisterPoolHandoff,
        },
        pool_import::{
            PoolImportPhase, PoolImportReservation, PoolImportSource, PoolImportSourceProgress,
            PoolImportStatus,
        },
        root::RootPoolBootstrapReleaseEvidence,
    },
};

pub(in crate::fleet_ensure::ops::release) fn fixture(
    root: Principal,
    subnet: Principal,
) -> RootPoolReleaseResponse {
    let historical_operator = Principal::from_slice(&[21]);
    let source = Principal::from_slice(&[22]);
    RootPoolReleaseResponse {
        root,
        bootstrap: Some(RootPoolBootstrapReleaseEvidence {
            review_sha256: [1; 32],
            install_id: [2; 32],
            operator: historical_operator,
            store: Principal::from_slice(&[23]),
            sources: vec![source],
        }),
        capacity_import: Some(PoolImportStatus {
            reserved_at_ns: 11,
            reservation: PoolImportReservation {
                sequence: 0,
                plan_sha256: [3; 32],
                root_authority_sha256: [4; 32],
                root,
                operator: historical_operator,
                subnet,
                transitional_controllers: vec![root, historical_operator],
                final_controllers: vec![root],
                sources: vec![PoolImportSource {
                    canister_id: source,
                    controllers: vec![root],
                    module_sha256: Some([5; 32]),
                    canister_version: 6,
                    stopped: false,
                    disposition_sha256: [7; 32],
                    observed_cycles: 8_000,
                    observed_reserved_cycles: 9,
                    minimum_ready_cycles: 100,
                    maximum_debit_cycles: 1_000,
                }],
                observed_root_cycles: 10_000,
                observed_root_reserved_cycles: 12,
                minimum_root_cycles: 500,
                maximum_root_debit_cycles: 4_000,
                maximum_paid_calls: 72,
            },
            progress: vec![PoolImportSourceProgress::StopIssued],
            phase: PoolImportPhase::Reserved,
            paid_calls: 72,
            reserved_debit_cycles: 3_072,
            last_root_cycles: 9_000,
            root_receipt: None,
        }),
        creation: Some(CanisterPoolCreation {
            attempt_count: 3,
            operation_id: [8; 32],
            cycles_ledger: Principal::from_slice(&[24]),
            placement_subnet: subnet,
            root,
            ledger_amount: Cycles::new(1_000),
            ledger_fee: Cycles::new(10),
            readiness_floor: Cycles::new(500),
            creation_execution_margin: Cycles::new(100),
            management_creation_fee: Cycles::new(20),
            created_at_time_ns: 13,
            last_attempt_at_ns: Some(14),
            progress: CanisterPoolCreationProgress::Intent {
                uncertain_result: true,
            },
        }),
        handoff: Some(CanisterPoolHandoff {
            canister_id: source,
            recipient: historical_operator,
            prepared_at_ns: 15,
        }),
    }
}

pub(in crate::fleet_ensure::ops::release) fn wire(status: RootPoolReleaseResponse) -> Vec<u8> {
    candid::encode_one(Ok::<_, CanicError>(Response::PoolRelease(Box::new(status)))).unwrap()
}

pub(in crate::fleet_ensure::ops::release) fn refused(error: CanicError) -> Vec<u8> {
    candid::encode_one(Err::<Response, _>(error)).unwrap()
}

#[test]
fn exhausted_and_historical_authority_survives_exact_collection() {
    let root = Principal::from_slice(&[1]);
    let subnet = Principal::from_slice(&[2]);
    let mut status = fixture(root, subnet);
    for phase in [
        PoolImportPhase::Reserved,
        PoolImportPhase::Released {
            publication_sha256: [9; 32],
        },
    ] {
        status.capacity_import.as_mut().unwrap().phase = phase;
        let bytes = wire(status.clone());
        let mut remaining = MAXIMUM_CENSUS_BYTES;
        let observed = decode(root, &bytes, &mut remaining).unwrap();
        assert_eq!(observed, status);
        assert_eq!(validate(&observed, root, &subnet), Ok(()));
        assert_eq!(remaining, MAXIMUM_CENSUS_BYTES - bytes.len());
    }
}

#[test]
fn every_root_and_subnet_binding_is_checked_independently() {
    let root = Principal::from_slice(&[1]);
    let subnet = Principal::from_slice(&[2]);
    for change in [
        |s: &mut RootPoolReleaseResponse| s.root = Principal::anonymous(),
        |s: &mut RootPoolReleaseResponse| {
            s.creation.as_mut().unwrap().root = Principal::anonymous();
        },
        |s: &mut RootPoolReleaseResponse| {
            s.creation.as_mut().unwrap().placement_subnet = Principal::anonymous();
        },
        |s: &mut RootPoolReleaseResponse| {
            s.capacity_import.as_mut().unwrap().reservation.root = Principal::anonymous();
        },
        |s: &mut RootPoolReleaseResponse| {
            s.capacity_import.as_mut().unwrap().reservation.subnet = Principal::anonymous();
        },
    ] {
        let mut status = fixture(root, subnet);
        change(&mut status);
        assert_eq!(
            validate(&status, root, &subnet),
            Err(ReleasePoolStage::Binding)
        );
    }
}

#[test]
fn decoder_bounds_bytes_work_and_total_collection() {
    #[derive(CandidType)]
    enum Extended {
        PoolRelease { root: Principal, extra: Vec<()> },
    }

    let root = Principal::from_slice(&[1]);
    let bytes = wire(fixture(root, Principal::from_slice(&[2])));
    assert!(matches!(
        decode(root, &bytes, &mut 0),
        Err(ReleasePoolError::Observation {
            stage: ReleasePoolStage::Budget,
            ..
        })
    ));
    for invalid in [b"DIDL".to_vec(), vec![0; RESPONSE_BYTES + 1]] {
        let mut remaining = MAXIMUM_CENSUS_BYTES;
        assert!(matches!(
            decode(root, &invalid, &mut remaining),
            Err(ReleasePoolError::Observation {
                stage: ReleasePoolStage::Decode,
                ..
            })
        ));
    }
    // An unknown field containing millions of null values is tiny on the wire.
    let expensive = candid::encode_one(Ok::<_, CanicError>(Extended::PoolRelease {
        root,
        extra: vec![(); RESPONSE_BYTES * 8 + 1],
    }))
    .unwrap();
    assert!(expensive.len() < RESPONSE_BYTES);
    assert!(
        candid::decode_one::<Result<Response, CanicError>>(&expensive)
            .unwrap()
            .is_ok()
    );
    let mut remaining = MAXIMUM_CENSUS_BYTES;
    assert!(matches!(
        decode(root, &expensive, &mut remaining),
        Err(ReleasePoolError::Observation {
            stage: ReleasePoolStage::Decode,
            ..
        })
    ));
}

#[test]
fn empty_obligations_and_typed_refusal_are_distinct() {
    let root = Principal::from_slice(&[1]);
    let status = RootPoolReleaseResponse {
        root,
        bootstrap: None,
        capacity_import: None,
        creation: None,
        handoff: None,
    };
    let mut remaining = MAXIMUM_CENSUS_BYTES;
    assert_eq!(
        decode(root, &wire(status.clone()), &mut remaining).unwrap(),
        status
    );
    let rejection =
        CanicError::from_registered(canic_contracts::diagnostics::codes::AUTHORITY_UNAVAILABLE);
    let bytes = refused(rejection);
    assert!(matches!(decode(root, &bytes, &mut remaining),
        Err(ReleasePoolError::Rejected { root: target, rejection: observed })
            if target == root && observed == rejection));
}

#[test]
fn assessment_keeps_wire_evidence_and_every_custody_candidate() {
    let root = Principal::from_slice(&[1]);
    let mut status = fixture(root, Principal::from_slice(&[2]));
    let created = Principal::from_slice(&[31]);
    let handed_off = Principal::from_slice(&[32]);
    let imported = Principal::from_slice(&[33]);
    let receipted = Principal::from_slice(&[34]);
    status.creation.as_mut().unwrap().progress = CanisterPoolCreationProgress::Created {
        block_index: 42,
        canister_id: created,
    };
    status.handoff.as_mut().unwrap().canister_id = handed_off;
    let import = status.capacity_import.as_mut().unwrap();
    import.reservation.sources[0].canister_id = imported;
    import.progress = vec![PoolImportSourceProgress::Ready(
        canic_contracts::dto::pool_import::PoolImportSourceReceipt {
            root_sender_canister_version: 1,
            canister_id: receipted,
            canister_version: 2,
            before_uninstall_canister_version: 1,
            retained_cycles: 900,
            retained_reserved_cycles: 0,
            observed_debit_cycles: 100,
        },
    )];
    import.phase = PoolImportPhase::Released {
        publication_sha256: [99; 32],
    };
    import.reserved_debit_cycles = import.reservation.maximum_root_debit_cycles;
    let mut remaining = MAXIMUM_CENSUS_BYTES;
    let decoded = decode(root, &wire(status.clone()), &mut remaining).unwrap();
    let evidence = FleetReleasePoolView {
        roots: vec![decoded],
    };
    let assessed = assess_pools(evidence.clone());
    assert_eq!(assessed.evidence, evidence);
    let [assessment] = assessed.roots.as_slice() else {
        panic!("one selected Root");
    };
    assert_eq!(assessment.facts.root, root);
    assert_eq!(
        assessment.import,
        Some(ReleasePoolImportDisposition::RecordedCompletion)
    );
    assert_eq!(
        assessment.creation,
        Some(ReleasePoolCreationDisposition::InventoryRecovery {
            canister_id: created
        })
    );
    let facts = assessment.facts.import.as_ref().unwrap();
    assert_eq!(facts.plan_sha256, [3; 32]);
    assert_eq!(facts.sequence, 0);
    assert!(facts.call_budget_exhausted);
    assert!(facts.debit_budget_exhausted);
    assert_eq!(
        assessment.facts.creation.as_ref().unwrap().operation_id,
        [8; 32]
    );
    assert_eq!(
        assessment.facts.handoff.as_ref().unwrap().recipient,
        status.handoff.as_ref().unwrap().recipient
    );
    let bootstrap = status.bootstrap.as_ref().unwrap();
    let expected = bootstrap
        .sources
        .iter()
        .copied()
        .chain([bootstrap.store, created, handed_off, imported, receipted])
        .collect();
    assert_eq!(assessment.facts.custody_candidates, expected);
}

#[test]
fn decoded_creation_uncertainty_cannot_be_assessed_as_cancelable() {
    let root = Principal::from_slice(&[1]);
    for (progress, expected) in [
        (
            CanisterPoolCreationProgress::Intent {
                uncertain_result: true,
            },
            ReleasePoolCreationDisposition::LedgerReconciliation,
        ),
        (
            CanisterPoolCreationProgress::Blocked {
                failure: CanisterPoolCreationFailure::UnresolvedAfterLedgerWindow,
            },
            ReleasePoolCreationDisposition::UnresolvedLedgerCreation,
        ),
    ] {
        let mut status = fixture(root, Principal::from_slice(&[2]));
        status.creation.as_mut().unwrap().progress = progress;
        let mut remaining = MAXIMUM_CENSUS_BYTES;
        let decoded = decode(root, &wire(status.clone()), &mut remaining).unwrap();
        let assessment = assess_pools(FleetReleasePoolView {
            roots: vec![decoded],
        });
        assert_eq!(assessment.evidence.roots, [status]);
        assert_eq!(assessment.roots[0].creation, Some(expected));
    }
}
