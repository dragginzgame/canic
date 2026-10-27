//! Retained funding evidence survives exhausted retries; malformed censuses never grant authority.

use super::*;
use crate::fleet_ensure::{
    view::release::{
        FleetReleaseFundingView, FleetReleaseRootFundingView,
        funding::{RecordedRefillCycles, ReleaseRefillDisposition},
    },
    workflow::release::assess_funding,
};
use canic_contracts::dto::{
    fleet_coordinator::CoordinatorFundingStatusResponse,
    icp_refill::{IcpRefillErrorCode, IcpRefillResponse, IcpRefillStatus, IcpRefillTrigger},
    root::RootIcpRefillReleaseEvidence,
};

fn root() -> Principal {
    Principal::from_slice(&[1])
}

fn page(ids: impl IntoIterator<Item = u64>, next_after: Option<u64>) -> RootFundingReleaseResponse {
    RootFundingReleaseResponse {
        fleet_subnet_root: root(),
        policy_generation: 1,
        policy_hash: [2; 32],
        icp_refill_policy: None,
        current_request: None,
        accepted_grant: None,
        rotation_current: None,
        icp_refills: ids
            .into_iter()
            .map(|id| {
                let mut operation_id = [0; 32];
                operation_id[..8].copy_from_slice(&id.to_le_bytes());
                RootIcpRefillReleaseEvidence {
                    transfer_uncertain: true,
                    record_id: id,
                    trigger: IcpRefillTrigger::Manual,
                    policy_hash: [3; 32],
                    source_canister: root(),
                    source_subaccount: Some([4; 32]),
                    target_canister: root(),
                    ledger_canister_id: Principal::from_slice(&[5]),
                    cmc_canister_id: Principal::from_slice(&[6]),
                    cmc_to_account_owner: Principal::from_slice(&[6]),
                    cmc_to_account_subaccount: Some([7; 32]),
                    amount_e8s: 100,
                    fee_e8s: 10,
                    budget_window_start_secs: 20,
                    budget_reserved: true,
                    memo: vec![8],
                    created_at_time_ns: 30,
                    notify_attempts: 10,
                    response: IcpRefillResponse {
                        operation_id,
                        status: IcpRefillStatus::Failed,
                        ledger_block_index: Some(40),
                        cycles_sent: None,
                        error_code: Some(IcpRefillErrorCode::NotifyMaxAttempts),
                        error_message: None,
                    },
                    refund_block_index: Some(41),
                    transaction_too_old_min_block_index: Some(42),
                }
            })
            .collect(),
        next_after,
    }
}

fn wire(page: RootFundingReleaseResponse) -> Vec<u8> {
    let native = canic_contracts::dto::funding_read::ReleasePage::try_from(page).unwrap();
    ic_wire::native::encode_response(
        protocol::funding_read::release::METHOD_ID,
        &Ok::<_, CanicError>(native),
        protocol::funding_read::RESPONSE_LIMITS,
    )
    .unwrap()
}

#[test]
fn release_assessment_retains_accounts_and_does_not_treat_overflow_as_zero() {
    let mut history = page([1, 2], None);
    let completed = &mut history.icp_refills[0];
    completed.transfer_uncertain = false;
    completed.response.status = IcpRefillStatus::Completed;
    completed.response.error_code = None;
    completed.response.cycles_sent = Some(ic_platform::candid::contract::Nat::from(123_u64));
    completed.refund_block_index = None;
    completed.transaction_too_old_min_block_index = None;
    history.icp_refills[1].refund_block_index = None;
    history.icp_refills[1].transaction_too_old_min_block_index = None;
    history.icp_refills[1].response.status = IcpRefillStatus::Completed;
    history.icp_refills[1].response.error_code = None;
    history.icp_refills[1].response.cycles_sent = Some(
        ic_platform::candid::contract::Nat::from(u128::MAX)
            + ic_platform::candid::contract::Nat::from(1_u8),
    );
    let expected = wire(history.clone());
    let evidence = FleetReleaseFundingView {
        coordinator: coordinator_status(&registry()),
        roots: vec![FleetReleaseRootFundingView {
            root: root(),
            pages: vec![history],
        }],
    };
    let facts = assessment_facts(&evidence).unwrap();
    assert_eq!(facts[0].refills[1].cycles, RecordedRefillCycles::Overflow);
    let assessed = assess_funding(evidence).unwrap();
    assert_eq!(wire(assessed.evidence.roots[0].pages[0].clone()), expected);
    assert_eq!(
        assessed.roots[0].refills[0].disposition,
        ReleaseRefillDisposition::RecordedConversion {
            ledger_block: 40,
            cycles: 123
        }
    );
    assert_eq!(
        assessed.roots[0].refills[1].disposition,
        ReleaseRefillDisposition::InvalidReceipt
    );
    assert!(assessed.roots[0].coordinator_operations.is_empty());
    assert_eq!(assessed.roots[0].rotation_operation, None);
}

#[test]
fn release_assessment_requires_observed_header_even_when_history_is_empty() {
    let evidence = FleetReleaseFundingView {
        coordinator: coordinator_status(&registry()),
        roots: vec![FleetReleaseRootFundingView {
            root: root(),
            pages: vec![],
        }],
    };
    assert!(matches!(
        assess_funding(evidence),
        Err(ReleaseFundingError::Observation {
            stage: ReleaseFundingStage::Binding,
            ..
        })
    ));
}

#[test]
fn census_retains_exact_exhausted_history_across_sparse_pages() {
    let first = page((1..=32).map(|id| id * 3), Some(96));
    let last = page([u64::MAX], None);
    let expected = [wire(first.clone()), wire(last.clone())];
    let mut pages = Pages::new(root(), [2; 32]);
    assert!(!pages.push(first).unwrap());
    assert!(pages.push(last).unwrap());
    assert_eq!(
        pages.pages.into_iter().map(wire).collect::<Vec<_>>(),
        expected
    );
    assert!(Pages::new(root(), [2; 32]).push(page([], None)).unwrap());
}

#[test]
fn census_rejects_cursor_gaps_duplicates_and_incomplete_pages() {
    for wrong in [
        page([], Some(1)),
        page([1], Some(1)),
        page(1..=32, Some(33)),
        page([2, 1], None),
        page([1, 1], None),
    ] {
        assert_eq!(
            Pages::new(root(), [2; 32]).push(wrong),
            Err(ReleaseFundingStage::Pagination)
        );
    }
    let mut pages = Pages::new(root(), [2; 32]);
    pages.push(page(1..=32, Some(32))).unwrap();
    assert_eq!(
        pages.push(page([], None)),
        Err(ReleaseFundingStage::Pagination)
    );
    let mut pages = Pages::new(root(), [2; 32]);
    pages.push(page(1..=32, Some(32))).unwrap();
    let mut duplicate = page([33], None);
    duplicate.icp_refills[0].response.operation_id[..8].copy_from_slice(&1_u64.to_le_bytes());
    assert_eq!(pages.push(duplicate), Err(ReleaseFundingStage::Pagination));
}

#[test]
fn census_rejects_root_policy_participant_and_header_drift() {
    for change in [
        |p: &mut RootFundingReleaseResponse| p.fleet_subnet_root = Principal::anonymous(),
        |p: &mut RootFundingReleaseResponse| p.policy_hash = [9; 32],
        |p: &mut RootFundingReleaseResponse| {
            p.icp_refills[0].source_canister = Principal::anonymous();
        },
        |p: &mut RootFundingReleaseResponse| {
            p.icp_refills[0].target_canister = Principal::anonymous();
        },
    ] {
        let mut wrong = page([1], None);
        change(&mut wrong);
        assert_eq!(
            Pages::new(root(), [2; 32]).push(wrong),
            Err(ReleaseFundingStage::Binding)
        );
    }
    let mut pages = Pages::new(root(), [2; 32]);
    pages.push(page(1..=32, Some(32))).unwrap();
    let mut changed = page([33], None);
    changed.policy_generation += 1;
    assert_eq!(pages.push(changed), Err(ReleaseFundingStage::Binding));
}

#[test]
fn census_bounds_records_without_overflow_or_truncation() {
    assert_eq!(
        Pages::new(root(), [2; 32]).push(page(1..=33, None)),
        Err(ReleaseFundingStage::Budget)
    );
    let mut pages = Pages::new(root(), [2; 32]);
    for start in (1..=4096).step_by(PAGE_SIZE) {
        let end = start + 31;
        assert_eq!(
            pages.push(page(start..=end, (end < 4096).then_some(end))),
            Ok(end == 4096)
        );
    }
    assert_eq!(pages.operations.len(), MAXIMUM_REFILLS);
    assert_eq!(
        pages.push(page([4097], None)),
        Err(ReleaseFundingStage::Budget)
    );
}

#[test]
fn decoder_bounds_native_frames_and_aggregate_bytes() {
    let bytes = wire(page([1], None));
    let mut remaining = bytes.len();
    let decoded = decode(root(), &bytes, &mut remaining).unwrap();
    assert_eq!(wire(decoded), bytes);
    assert_eq!(remaining, 0);
    assert!(matches!(
        decode(root(), &bytes, &mut remaining),
        Err(ReleaseFundingError::Observation {
            stage: ReleaseFundingStage::Budget,
            ..
        })
    ));
    for invalid in [
        vec![0; RESPONSE_BYTES + 1],
        b"DIDL".to_vec(),
        ic_platform::candid::contract::encode_one("wrong reply").unwrap(),
    ] {
        let mut remaining = MAXIMUM_CENSUS_BYTES;
        assert!(matches!(
            decode(root(), &invalid, &mut remaining),
            Err(ReleaseFundingError::Observation {
                stage: ReleaseFundingStage::Decode,
                ..
            })
        ));
    }
    let native = canic_contracts::dto::funding_read::ReleasePage::try_from(page([], None)).unwrap();
    let wrong_method = ic_wire::native::encode_response(
        protocol::pool_inventory::METHOD_ID,
        &Ok::<_, CanicError>(native),
        protocol::funding_read::RESPONSE_LIMITS,
    )
    .unwrap();
    let mut trailing = bytes.clone();
    trailing.push(0);
    for invalid in [wrong_method, trailing, bytes[..bytes.len() - 1].to_vec()] {
        let mut remaining = MAXIMUM_CENSUS_BYTES;
        assert!(matches!(
            decode(root(), &invalid, &mut remaining),
            Err(ReleaseFundingError::Observation {
                stage: ReleaseFundingStage::Decode,
                ..
            })
        ));
    }
}

fn registry() -> FleetRegistry {
    let (_, mut registry) = inventory::tests::fixture();
    registry.fleet_subnet_roots[0].fleet_subnet_root = root();
    registry
}

pub(in crate::fleet_ensure::ops::release) fn coordinator_status(
    registry: &FleetRegistry,
) -> CoordinatorFundingStatusResponse {
    use canic_contracts::dto::fleet_coordinator::{
        CoordinatorFundingStatusResponse, CoordinatorFundingWindowStatusResponse,
        CoordinatorRootFundingStatusResponse,
    };
    CoordinatorFundingStatusResponse {
        coordinator: registry.authority.binding.coordinator,
        current_cycles: 1_000u128.into(),
        policy_generation: 1,
        funding_enabled: true,
        funding_profile: None,
        policy: None,
        fleet_window: None,
        historical_automatic_grants: 0,
        historical_automatic_cycles: 0u128.into(),
        automatic_grants: 0,
        automatic_cycles: 0u128.into(),
        rotation_checkpoint_count: 0,
        rotation_checkpoint_root_count: 0,
        rotation_checkpoint_root_capacity_remaining: 0,
        rotation: None,
        roots: registry
            .fleet_subnet_roots
            .iter()
            .map(|root| CoordinatorRootFundingStatusResponse {
                fleet_subnet_root: root.fleet_subnet_root,
                lifecycle_status: root.status,
                policy_hash: fleet_subnet_root_funding_policy_hash(&root.funding),
                policy: root.funding.root_funding.clone(),
                window: CoordinatorFundingWindowStatusResponse {
                    window_start_secs: 0,
                    spent_cycles: 0u128.into(),
                    reserved_cycles: 0u128.into(),
                },
                historical_automatic_grants: 0,
                historical_automatic_cycles: 0u128.into(),
                automatic_grants: 0,
                automatic_cycles: 0u128.into(),
                last_successful_grant_at_ns: None,
                current_operation: None,
                last_result: None,
            })
            .collect(),
    }
}

#[test]
fn coordinator_census_requires_complete_unique_registry_bound_roots() {
    let registry = registry();
    let status = coordinator_status(&registry);
    assert_eq!(coordinator::validate(&status, &registry), Ok(()));
    for change in [
        |s: &mut CoordinatorFundingStatusResponse| s.coordinator = Principal::anonymous(),
        |s: &mut CoordinatorFundingStatusResponse| s.roots.clear(),
        |s: &mut CoordinatorFundingStatusResponse| s.roots.push(s.roots[0].clone()),
        |s: &mut CoordinatorFundingStatusResponse| {
            s.roots[0].fleet_subnet_root = Principal::anonymous();
        },
        |s: &mut CoordinatorFundingStatusResponse| s.roots[0].policy_hash = [0; 32],
        |s: &mut CoordinatorFundingStatusResponse| {
            s.roots[0].policy.budget.maximum_cycles = 1u128.into();
        },
        |s: &mut CoordinatorFundingStatusResponse| {
            s.roots[0].lifecycle_status =
                canic_contracts::dto::fleet_registry::FleetSubnetRootStatus::Draining;
        },
    ] {
        let mut wrong = status.clone();
        change(&mut wrong);
        assert_eq!(
            coordinator::validate(&wrong, &registry),
            Err(ReleaseFundingStage::Binding)
        );
    }
    let mut registry = registry;
    let mut second = registry.fleet_subnet_roots[0].clone();
    second.fleet_subnet_root = Principal::from_slice(&[99]);
    registry.fleet_subnet_roots.push(second);
    let mut status = coordinator_status(&registry);
    status.roots.reverse();
    assert_eq!(coordinator::validate(&status, &registry), Ok(()));
    status.roots[1] = status.roots[0].clone();
    assert_eq!(
        coordinator::validate(&status, &registry),
        Err(ReleaseFundingStage::Binding)
    );
}

#[test]
fn assessment_keeps_coordinator_pending_work_and_completed_history_distinct() {
    use canic_contracts::dto::fleet_coordinator::{
        FleetFundingPolicyRotationStatusPhase, FleetFundingPolicyRotationStatusResponse,
    };
    use canic_contracts::dto::fleet_funding::{
        FleetRootFundingNoGrantReason, FleetRootFundingNoGrantReceipt, FleetRootFundingRequest,
        FleetRootFundingResponse,
    };
    let registry = registry();
    let request = FleetRootFundingRequest {
        operation_id: [9; 32],
        operation_sequence: 1,
        expected_registry: canic_contracts::dto::fleet_registry::FleetRegistryVersion {
            authority: registry.authority.clone(),
            revision: registry.revision,
            content_hash: [8; 32],
        },
        observed_balance: 100u128.into(),
        requested_cycles: 200u128.into(),
        policy_hash: [2; 32],
    };
    let mut coordinator = coordinator_status(&registry);
    coordinator.roots[0].current_operation = Some(request.clone());
    coordinator.roots[0].window.reserved_cycles = 200u128.into();
    coordinator.rotation = Some(FleetFundingPolicyRotationStatusResponse {
        operation_id: [7; 32],
        plan_digest: [6; 32],
        predecessor_generation: 1,
        successor_generation: 2,
        phase: FleetFundingPolicyRotationStatusPhase::Staging {
            staged_root_count: 0,
            expected_root_count: 1,
        },
    });
    let mut evidence = FleetReleaseFundingView {
        coordinator,
        roots: vec![FleetReleaseRootFundingView {
            root: root(),
            pages: vec![page([], None)],
        }],
    };
    let report = assess_funding(evidence.clone()).unwrap();
    assert_eq!(report.roots[0].coordinator_operations, [[9; 32]]);
    assert_eq!(report.coordinator_rotation_operation, Some([7; 32]));
    assert_eq!(
        report.evidence.coordinator.roots[0]
            .window
            .reserved_cycles
            .to_u128(),
        200
    );
    evidence.roots[0].pages[0].current_request = Some(request.clone());
    assert_eq!(
        assess_funding(evidence.clone()).unwrap().roots[0].coordinator_operations,
        [[9; 32]]
    );
    evidence.coordinator.roots[0].current_operation = None;
    evidence.coordinator.roots[0].last_result = Some(FleetRootFundingResponse::NoGrant(
        FleetRootFundingNoGrantReceipt {
            request,
            reason: FleetRootFundingNoGrantReason::RootRejected,
            decided_at_ns: 10,
        },
    ));
    // Root can still await a terminal decision's lost reply; retain that work.
    assert_eq!(
        assess_funding(evidence.clone()).unwrap().roots[0].coordinator_operations,
        [[9; 32]]
    );
    evidence.roots[0].pages[0].current_request = None;
    let completed = assess_funding(evidence).unwrap();
    assert!(completed.roots[0].coordinator_operations.is_empty());
    assert!(
        completed.evidence.coordinator.roots[0]
            .last_result
            .is_some()
    );
}

#[test]
fn native_census_retains_arbitrary_cycles_and_rejects_nonminimal_magnitudes() {
    use canic_contracts::protocol::authority::encode;
    use ic_platform::candid::contract::Nat;
    for amount in [
        None,
        Some(Nat::from(0u8)),
        Some(Nat::from(u128::MAX)),
        Some(Nat::from(u128::MAX) + Nat::from(1u8)),
    ] {
        let mut original = page([1], None);
        original.icp_refills[0].response.cycles_sent = amount;
        let expected = encode(&original).unwrap();
        let bytes = wire(original);
        let mut remaining = MAXIMUM_CENSUS_BYTES;
        let decoded = decode(root(), &bytes, &mut remaining).unwrap();
        assert_eq!(encode(decoded).unwrap(), expected);
    }
    let mut native =
        canic_contracts::dto::funding_read::ReleasePage::try_from(page([1], None)).unwrap();
    native.icp_refills[0].response.cycles_sent = Some(ic_wire::Bytes::new(vec![0, 1]).unwrap());
    let bytes = ic_wire::native::encode_response(
        protocol::funding_read::release::METHOD_ID,
        &Ok::<_, CanicError>(native),
        protocol::funding_read::RESPONSE_LIMITS,
    )
    .unwrap();
    let mut remaining = MAXIMUM_CENSUS_BYTES;
    assert!(matches!(
        decode(root(), &bytes, &mut remaining),
        Err(ReleaseFundingError::Observation {
            stage: ReleaseFundingStage::Decode,
            ..
        })
    ));
}

#[test]
fn coordinator_native_census_preserves_all_rotation_phases_and_receipts() {
    use canic_contracts::dto::{
        fleet_coordinator::{
            FleetFundingPolicyRotationStatusPhase as Phase,
            FleetFundingPolicyRotationStatusResponse as Rotation,
        },
        fleet_funding::FleetFundingPolicyRotationReceipt,
        fleet_registry::FleetRegistryVersion,
        funding_read::CoordinatorStatus,
    };
    let registry = registry();
    let version = FleetRegistryVersion {
        authority: registry.authority.clone(),
        revision: registry.revision,
        content_hash: [1; 32],
    };
    let completed = FleetFundingPolicyRotationReceipt {
        operation_id: [2; 32],
        plan_digest: [3; 32],
        predecessor_registry: version.clone(),
        successor_registry: version.clone(),
        predecessor_generation: 4,
        successor_generation: 5,
        affected_root_count: 1,
        retained_historical_automatic_grants: u64::MAX,
        retained_historical_automatic_cycles: u128::MAX.into(),
        successor_policy_set_hash: [6; 32],
        maximum_new_automatic_cycles: 7.into(),
        apply_operator_debit: 8.into(),
        completed_at_ns: 9,
    };
    for phase in [
        Phase::Staging {
            staged_root_count: 1,
            expected_root_count: 2,
        },
        Phase::PreparingRoots {
            prepared_root_count: 1,
            expected_root_count: 2,
        },
        Phase::ActivatingRoots {
            activated_root_count: 1,
            expected_root_count: 2,
            successor_registry: Box::new(version),
        },
        Phase::Completed(Box::new(completed)),
    ] {
        let mut expected = coordinator_status(&registry);
        expected.rotation = Some(Rotation {
            operation_id: [2; 32],
            plan_digest: [3; 32],
            predecessor_generation: 4,
            successor_generation: 5,
            phase,
        });
        let response: Result<CoordinatorStatus, CanicError> = Ok(expected.clone().into());
        let bytes = ic_wire::native::encode_response(
            protocol::funding_read::coordinator_status::METHOD_ID,
            &response,
            protocol::funding_read::RESPONSE_LIMITS,
        )
        .unwrap();
        let mut remaining = bytes.len();
        let native: CoordinatorStatus = decode_response(
            protocol::funding_read::coordinator_status::METHOD_ID,
            expected.coordinator,
            &bytes,
            &mut remaining,
        )
        .unwrap();
        assert_eq!(CoordinatorFundingStatusResponse::from(native), expected);
        assert_eq!(remaining, 0);
        assert!(matches!(
            decode_response::<CoordinatorStatus>(
                protocol::funding_read::coordinator_status::METHOD_ID,
                expected.coordinator,
                &bytes,
                &mut remaining
            ),
            Err(ReleaseFundingError::Observation {
                stage: ReleaseFundingStage::Budget,
                ..
            })
        ));
        assert!(matches!(
            decode_response::<CoordinatorStatus>(
                protocol::funding_read::root_status::METHOD_ID,
                expected.coordinator,
                &bytes,
                &mut bytes.len()
            ),
            Err(ReleaseFundingError::Observation {
                stage: ReleaseFundingStage::Decode,
                ..
            })
        ));
    }
}
