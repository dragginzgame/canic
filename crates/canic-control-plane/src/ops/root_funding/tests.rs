//! Direct Root funding journal, acceptance, replay, and rejection evidence.

use super::*;
use crate::{
    storage::stable::root_funding::{RootFundingData, RootFundingStore},
    view::root_funding::RootFundingAcceptanceDisposition,
};
use canic_core::dto::fleet_funding::{
    FleetFundingPolicyRotationPlacementEvidence, FleetFundingPolicyRotationRootActivateRequest,
    FleetFundingPolicyRotationRootPlan, FleetFundingPolicyRotationRootPrepareRequest,
    FleetFundingPolicyUsage, FleetRootFundingAcceptanceRequest, FleetRootFundingNoGrantReason,
    FleetRootFundingNoGrantReceipt,
};
use canic_core::dto::fleet_registry::FleetSubnetRootStatus;
use canic_core::ids::SubnetId;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one end-to-end journal interruption and successor proof"
)]
fn root_request_acceptance_response_loss_and_successor_are_exact() {
    RootFundingStore::import(RootFundingData::default());
    RootFundingOps::commit_genesis(RootFundingOps::compile_genesis()).expect("funding genesis");
    let authority = authority();
    let request = RootFundingOps::prepare_request(&authority, 42_200_000_000, 10)
        .expect("prepare first request");
    assert_eq!(
        request,
        crate::test_support::root_funding_request_fixture(1)
    );

    let prepared = RootFundingStore::export();
    RootFundingStore::import(prepared.clone());
    assert_eq!(
        RootFundingOps::current_request(&authority).expect("restore current request"),
        Some(request.clone())
    );
    assert_eq!(
        RootFundingOps::prepare_request(&authority, 1, 99).expect("resume exact request"),
        request
    );
    assert_eq!(RootFundingStore::export(), prepared);

    let acceptance = acceptance_request(&request);
    assert_eq!(
        RootFundingOps::prepare_acceptance(
            &authority,
            &acceptance,
            request.requested_cycles.to_u128(),
            request.observed_balance.to_u128(),
        )
        .expect("fresh acceptance"),
        RootFundingAcceptanceDisposition::Fresh
    );
    let receipt = RootFundingOps::record_acceptance(&authority, &acceptance, 20)
        .expect("record accepted cycles");
    let accepted = RootFundingStore::export();
    let accepted_record = accepted.current.as_ref().expect("accepted funding state");
    assert_eq!(accepted_record.automatic_grants, 1);
    assert_eq!(accepted_record.automatic_cycles, request.requested_cycles);
    RootFundingStore::import(accepted.clone());

    assert_eq!(
        RootFundingOps::prepare_acceptance(
            &authority,
            &acceptance,
            request.requested_cycles.to_u128(),
            0,
        )
        .expect("response-loss replay"),
        RootFundingAcceptanceDisposition::Replay(Box::new(receipt.clone()))
    );
    assert_eq!(RootFundingStore::export(), accepted);

    assert_eq!(
        RootFundingOps::record_acceptance(&authority, &acceptance, 999).expect("acceptance replay"),
        receipt
    );
    assert_eq!(RootFundingStore::export(), accepted);

    let response = FleetRootFundingResponse::Granted(receipt);
    assert_eq!(
        RootFundingOps::record_response(&authority, response.clone(), 30)
            .expect("complete accepted operation"),
        response
    );
    let terminal = RootFundingStore::export();
    assert_eq!(
        RootFundingOps::record_response(&authority, response, 999)
            .expect("exact terminal response replay"),
        terminal
            .current
            .as_ref()
            .expect("funding state")
            .last
            .as_ref()
            .expect("last result")
            .response
    );
    assert_eq!(RootFundingStore::export(), terminal);
    assert_eq!(
        RootFundingOps::current_request(&authority).expect("terminal journal has no current"),
        None
    );

    let successor = RootFundingOps::prepare_request(&authority, 42_200_000_000, 40)
        .expect("prepare monotonic successor");
    assert_eq!(
        successor,
        crate::test_support::root_funding_request_fixture(2)
    );
    let journal = RootFundingOps::current_for_test(&authority).expect("valid journal");
    assert_eq!(
        journal
            .last
            .as_ref()
            .expect("retained predecessor")
            .request
            .operation_sequence,
        1
    );
    assert_eq!(
        journal
            .current
            .as_ref()
            .expect("current successor")
            .request
            .operation_sequence,
        2
    );
}

#[test]
fn root_acceptance_allows_subthreshold_refunds_but_rejects_relieved_balance() {
    RootFundingStore::import(RootFundingData::default());
    RootFundingOps::commit_genesis(RootFundingOps::compile_genesis()).expect("funding genesis");
    let authority = authority();
    let request =
        RootFundingOps::prepare_request(&authority, 42_200_000_000, 10).expect("prepare request");
    let acceptance = acceptance_request(&request);
    let durable = RootFundingStore::export();

    assert!(
        RootFundingOps::prepare_acceptance(
            &authority,
            &acceptance,
            request.requested_cycles.to_u128() - 1,
            request.observed_balance.to_u128(),
        )
        .is_err()
    );
    assert_eq!(
        RootFundingOps::prepare_acceptance(
            &authority,
            &acceptance,
            request.requested_cycles.to_u128(),
            authority.funding.root_funding.request_threshold.to_u128(),
        )
        .expect("subthreshold in-flight refund must preserve low-balance demand"),
        RootFundingAcceptanceDisposition::Fresh
    );
    assert!(
        RootFundingOps::prepare_acceptance(
            &authority,
            &acceptance,
            request.requested_cycles.to_u128(),
            authority.funding.root_funding.request_threshold.to_u128() + 1,
        )
        .is_err()
    );
    let mut wrong = acceptance;
    wrong.operation_id[0] ^= 1;
    assert!(
        RootFundingOps::prepare_acceptance(
            &authority,
            &wrong,
            wrong.granted_cycles.to_u128(),
            wrong.observed_balance.to_u128(),
        )
        .is_err()
    );
    assert_eq!(RootFundingStore::export(), durable);
}

#[test]
fn terminal_no_grant_advances_once_and_cannot_skip_sequence() {
    RootFundingStore::import(RootFundingData::default());
    RootFundingOps::commit_genesis(RootFundingOps::compile_genesis()).expect("funding genesis");
    let authority = authority();
    let request =
        RootFundingOps::prepare_request(&authority, 42_200_000_000, 10).expect("prepare request");
    let response = FleetRootFundingResponse::NoGrant(FleetRootFundingNoGrantReceipt {
        request,
        reason: FleetRootFundingNoGrantReason::CoordinatorReserveUnavailable,
        decided_at_ns: 20,
    });
    RootFundingOps::record_response(&authority, response, 21).expect("record no-grant result");
    let successor =
        RootFundingOps::prepare_request(&authority, 42_200_000_000, 30).expect("prepare successor");
    assert_eq!(successor.operation_sequence, 2);

    let mut corrupted = RootFundingStore::export();
    corrupted
        .current
        .as_mut()
        .expect("funding state")
        .current
        .as_mut()
        .expect("active operation")
        .request
        .operation_sequence = 3;
    RootFundingStore::import(corrupted);
    assert!(RootFundingOps::current_for_test(&authority).is_err());
}

#[test]
fn lifecycle_fence_rejects_a_fresh_request_before_mutation() {
    RootFundingStore::import(RootFundingData::default());
    RootFundingOps::commit_genesis(RootFundingOps::compile_genesis()).expect("funding genesis");
    let mut fenced = authority();
    fenced.status = FleetSubnetRootStatus::Draining;
    fenced.funding_eligible = false;
    let before = RootFundingStore::export();

    RootFundingOps::prepare_request(&fenced, 42_200_000_000, 10)
        .expect_err("funding fence must reject a fresh Root request");

    assert_eq!(RootFundingStore::export(), before);
}

#[test]
fn root_funding_status_projects_protected_policy_and_durable_usage() {
    RootFundingStore::import(RootFundingData::default());
    RootFundingOps::commit_genesis(RootFundingOps::compile_genesis()).expect("funding genesis");
    let authority = authority();
    let status = RootFundingOps::status(&authority, true, 42_200_000_000, 3_700)
        .expect("root funding status");

    assert_eq!(status.fleet_subnet_root, authority.fleet_subnet_root);
    assert_eq!(status.lifecycle_status, FleetSubnetRootStatus::Active);
    assert!(status.funding_eligible);
    assert!(status.cycles_funding_enabled);
    assert_eq!(status.current_cycles.to_u128(), 42_200_000_000);
    assert_eq!(status.root_policy, authority.funding.root_funding);
    assert_eq!(status.current_operation, None);
    assert_eq!(status.last_result, None);
    assert_eq!(status.automatic_grants, 0);
    assert_eq!(status.automatic_cycles.to_u128(), 0);
    assert!(status.latest_icp_refill.is_none());
}

#[test]
fn root_policy_rotation_replays_exact_terminal_activation_and_rejects_payload_drift() {
    RootFundingStore::import(RootFundingData::default());
    RootFundingOps::commit_genesis(RootFundingOps::compile_genesis()).expect("funding genesis");
    let predecessor = authority();
    let request = rotation_prepare_request(&predecessor);

    let prepared = RootFundingOps::prepare_policy_rotation(&predecessor, request.clone(), 10)
        .expect("prepare rotation");
    let prepared_state = RootFundingStore::export();
    assert!(prepared.prepared);
    assert!(!prepared.activated);
    assert_eq!(
        RootFundingOps::prepare_policy_rotation(&predecessor, request.clone(), 99)
            .expect("exact prepare replay"),
        prepared
    );
    assert_eq!(
        RootFundingOps::policy_rotation_prepare_replay(&predecessor, &request)
            .expect("active prepare replay lookup"),
        Some(prepared)
    );
    assert_eq!(RootFundingStore::export(), prepared_state);

    let mut successor = predecessor;
    successor.registry.revision += 1;
    successor.registry.content_hash = [92; 32];
    successor.funding.root_funding = request.root.proposed_policy.clone();
    assert!(
        RootFundingOps::current_for_test(&successor).is_err(),
        "ordinary authority validation must reject the protected-successor/predecessor-journal split"
    );
    assert_eq!(
        RootFundingOps::prepared_policy_rotation()
            .expect("prepared recovery remains available across the mixed authority boundary")
            .request,
        request
    );
    let activate = FleetFundingPolicyRotationRootActivateRequest {
        operation_id: request.operation_id,
        plan_digest: request.plan_digest,
        predecessor_registry: request.predecessor_registry.clone(),
        successor_registry: successor.registry.clone(),
        predecessor_generation: request.predecessor_generation,
        successor_generation: request.successor_generation,
        fleet_subnet_root: request.root.fleet_subnet_root,
    };
    let activated = RootFundingOps::complete_policy_rotation(
        &successor,
        activate.operation_id,
        activate.plan_digest,
        20,
    )
    .expect("complete rotation");
    assert!(activated.activated);
    let terminal = RootFundingStore::export();
    assert_eq!(
        RootFundingOps::completed_policy_rotation(&successor, &activate)
            .expect("terminal replay lookup"),
        Some(activated.clone())
    );
    assert_eq!(RootFundingStore::export(), terminal);
    assert_eq!(
        RootFundingOps::prepare_policy_rotation(&successor, request, 999)
            .expect("terminal prepare replay"),
        activated
    );
    assert_eq!(
        RootFundingOps::policy_rotation_prepare_replay(
            &successor,
            &rotation_prepare_request(&authority()),
        )
        .expect("terminal prepare replay lookup"),
        Some(activated)
    );

    let mut altered = activate;
    altered.successor_registry.content_hash[0] ^= 1;
    assert!(RootFundingOps::completed_policy_rotation(&successor, &altered).is_err());
    assert_eq!(RootFundingStore::export(), terminal);
}

#[test]
fn root_policy_rotation_retains_exhausted_usage_and_resets_only_successor_counters() {
    let predecessor = authority();
    let mut exhausted = RootFundingOps::compile_genesis();
    exhausted.automatic_grants = predecessor.funding.root_funding.maximum_automatic_grants;
    exhausted.automatic_cycles = predecessor
        .funding
        .root_funding
        .maximum_automatic_cycles
        .clone();
    RootFundingStore::import(RootFundingData {
        current: Some(exhausted),
    });
    let mut request = rotation_prepare_request(&predecessor);
    request.root.predecessor_usage.generation_automatic_grants =
        predecessor.funding.root_funding.maximum_automatic_grants;
    request.root.predecessor_usage.generation_automatic_cycles = predecessor
        .funding
        .root_funding
        .maximum_automatic_cycles
        .clone();
    RootFundingOps::prepare_policy_rotation(&predecessor, request.clone(), 10)
        .expect("prepare exhausted generation");

    let mut successor = predecessor.clone();
    successor.registry.revision += 1;
    successor.registry.content_hash = [95; 32];
    RootFundingOps::complete_policy_rotation(
        &successor,
        request.operation_id,
        request.plan_digest,
        20,
    )
    .expect("complete exhausted generation rotation");
    let current = RootFundingOps::current_for_test(&successor).expect("successor journal");
    assert_eq!(current.policy_generation, 2);
    assert_eq!(
        current.historical_automatic_grants,
        u64::from(predecessor.funding.root_funding.maximum_automatic_grants)
    );
    assert_eq!(
        current.historical_automatic_cycles,
        predecessor.funding.root_funding.maximum_automatic_cycles
    );
    assert_eq!(current.automatic_grants, 0);
    assert_eq!(current.automatic_cycles.to_u128(), 0);
}

fn authority() -> RootFundingAuthorityView {
    let request = crate::test_support::root_funding_request_fixture(1);
    RootFundingAuthorityView {
        registry: request.expected_registry,
        fleet_subnet_root: candid::Principal::from_slice(&[71; 29]),
        status: FleetSubnetRootStatus::Active,
        funding_eligible: true,
        funding: crate::test_support::fleet_subnet_root_funding_authority(),
    }
}

fn rotation_prepare_request(
    authority: &RootFundingAuthorityView,
) -> FleetFundingPolicyRotationRootPrepareRequest {
    FleetFundingPolicyRotationRootPrepareRequest {
        operation_id: [81; 32],
        plan_digest: [82; 32],
        predecessor_registry: authority.registry.clone(),
        predecessor_generation: 1,
        successor_generation: 2,
        root: FleetFundingPolicyRotationRootPlan {
            fleet_subnet_root: authority.fleet_subnet_root,
            predecessor_policy_hash: fleet_subnet_root_funding_policy_hash(&authority.funding),
            predecessor_usage: FleetFundingPolicyUsage {
                historical_automatic_grants: 0,
                historical_automatic_cycles: 0_u128.into(),
                generation_automatic_grants: 0,
                generation_automatic_cycles: 0_u128.into(),
            },
            proposed_policy: authority.funding.root_funding.clone(),
            placement: FleetFundingPolicyRotationPlacementEvidence {
                subnet: SubnetId::from_principal(candid::Principal::from_slice(&[83; 29])),
                node_count: 13,
                cost_multiplier_numerator: 1,
                cost_multiplier_denominator: 1,
                fiduciary: false,
                acknowledge_fiduciary_cost: false,
            },
        },
    }
}

fn acceptance_request(request: &FleetRootFundingRequest) -> FleetRootFundingAcceptanceRequest {
    FleetRootFundingAcceptanceRequest {
        operation_id: request.operation_id,
        operation_sequence: request.operation_sequence,
        expected_registry: request.expected_registry.clone(),
        observed_balance: request.observed_balance.clone(),
        granted_cycles: request.requested_cycles.clone(),
        policy_hash: request.policy_hash,
    }
}
