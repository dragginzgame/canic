//! Module: pic::fleet_registry::baseline::tests::native_funding
//!
//! Responsibility: qualify native supplementary funding against a real issued IC operation.
//! Boundary: the existing fixture injects an underforecast; production adapters own all effects.

mod artifact;

use super::*;
use canic_host::fleet_ensure::{
    model::{FundingPauseRecord, FundingReviewRecord},
    ops::{EnsurePaths, read_journal},
};

pub(super) use artifact::{bind_audit_root, uses_audit_root};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum Scenario {
    SyntheticMinimum,
    ChildClaim,
}

#[test]
pub(super) fn native_withdrawal_recovers_the_same_initial_child_claim() {
    assert_literal_zero_host_journey(FundingJourney::NativeChildFunding, 2);
}

#[test]
pub(super) fn issued_provisioning_recovers_native_withdrawal_and_receipt_observation() {
    assert_literal_zero_host_journey(FundingJourney::NativeFunding, 1);
}

/// Remove only the forecast native credit before the fixture issues any effect.
pub(super) fn omit_forecast_native_funding(plan: &mut FleetEnsurePlan, root: Principal) {
    let fee = plan
        .reviewed_desired
        .as_ref()
        .unwrap()
        .desired()
        .ledger_fee_cycles
        .parse::<Cycles>()
        .unwrap()
        .to_u128();
    let mut removed = 0;
    for canister in &mut plan.canisters {
        canister.actions.retain(|action| {
            if let EnsureAction::Fund {
                amount, principal, ..
            } = action
            {
                assert_eq!(principal, &root.to_text());
                plan.conservation.maximum_new_funding_cycles -= amount;
                plan.conservation.maximum_operator_debit_cycles -= amount + fee;
                plan.conservation.maximum_unavoidable_fee_cycles -= fee;
                removed += 1;
                false
            } else {
                true
            }
        });
    }
    assert_eq!(removed, 1, "one deliberate native underforecast");
}

#[expect(
    clippy::too_many_lines,
    reason = "the composed IC proof retains one operation across withdrawal, receipt loss and terminal conservation"
)]
pub(super) fn assert_issued_native_funding(input: &AutonomousFundingJourney<'_>) {
    let mut desired = input.desired.clone();
    // A deliberately high fixture minimum makes the underforecast reproducible
    // without changing the canonical Root, runtime reserve guard or recovery behavior.
    let root = desired
        .canisters
        .iter_mut()
        .find(|canister| {
            canister.kind == canic_host::fleet_ensure::model::DesiredCanisterKind::Root
                && canister.principal.as_deref() == Some(input.root.to_text().as_str())
        })
        .expect("exact generated Root");
    let minimum = if input.native_pause == Some(Scenario::ChildClaim) {
        burn_to(input, 20_000_000_000_000);
        10_000_000_000_000_u128.to_string()
    } else {
        (input.pic.cycle_balance(input.root) + 20_000_000_000_000).to_string()
    };
    root.minimum_cycles.clone_from(&minimum);
    root.initial_cycles = minimum;
    let source = desired_sha256(&desired);
    let platform = || {
        IcpEnsurePlatform::new(
            desired.clone(),
            input.icp_wrapper.to_str().unwrap(),
            input.adapter_root,
        )
        .with_local_replica(input.local_replica.clone())
    };
    let paths = EnsurePaths::under(input.adapter_root, &desired.environment, &desired.fleet);
    let state = canic_host::fleet_ensure::ops::read_state(&paths, &desired.fleet).unwrap();
    let mut initial = platform();
    let mut planned = fleet_ensure_workflow::plan(
        input.adapter_root,
        &desired,
        &source,
        &desired.fleet,
        1_800_000_000_000_000_001,
        &mut initial,
    )
    .unwrap();
    retain_issued_underfunded_fixture(input, &mut planned.plan, &state, &mut initial);
    let issued = read_journal(&paths).unwrap().unwrap();
    assert_eq!(issued.effects.last().unwrap().state, EffectState::Issued);
    let child = (input.native_pause == Some(Scenario::ChildClaim)).then(|| {
        // advance_time is a read followed by a write; the live gateway's automatic
        // clock must not advance between them while this fixture drives bootstrap.
        assert!(input.pic.auto_progress_enabled());
        input.pic.stop_progress();
        let claim = block_initial_child(input, &planned.plan, &issued);
        input.pic.auto_progress();
        assert!(input.pic.auto_progress_enabled());
        claim
    });
    let review = fleet_ensure_workflow::plan(
        input.adapter_root,
        &desired,
        &source,
        &desired.fleet,
        1_800_000_000_000_000_099,
        &mut initial,
    )
    .unwrap()
    .funding_review
    .expect("native underforecast review");
    let FundingPauseRecord::Native(pause) = &review.pause else {
        panic!("native review");
    };
    assert_eq!(pause.root_principal, input.root.to_text());
    assert_eq!(pause.operation_id, issued.operation_id);
    assert_eq!(
        pause.provisioning_action_sha256,
        issued.effects.last().unwrap().action_sha256
    );
    assert!(review.effect.is_none());
    assert_eq!(
        read_journal(&paths).unwrap().unwrap().effects,
        issued.effects
    );
    #[expect(
        clippy::result_large_err,
        reason = "the fixture preserves the production typed workflow error"
    )]
    let apply = |digest: &str, adapter: &mut IcpEnsurePlatform| {
        fleet_ensure_workflow::apply(
            input.adapter_root,
            &desired,
            &source,
            &desired.fleet,
            digest,
            adapter,
        )
    };
    assert!(matches!(
        apply(&planned.plan.plan_sha256, &mut initial),
        Err(EnsureWorkflowError::NativeFundingRequired { review_sha256 })
        if review_sha256 == review.review_sha256
    ));
    let before = ledger_account_balance(input.pic, input.cycles_ledger, input.operator);
    assert_eq!(withdrawals(input), 0);
    std::fs::write(input.adapter_root.join("lose-funding-response"), []).unwrap();
    std::fs::write(input.adapter_root.join("lose-funded-observation"), []).unwrap();
    let lost = apply(&review.review_sha256, &mut initial);
    assert!(
        matches!(lost, Err(EnsureWorkflowError::Platform(_))),
        "{lost:?}"
    );
    assert!(input.adapter_root.join("lost-funding-response").is_file());
    assert_eq!(withdrawals(input), 1);
    let after = ledger_account_balance(input.pic, input.cycles_ledger, input.operator);
    assert_eq!(
        before,
        after.clone() + Nat::from(pause.shortfall_cycles + pause.ledger_fee_cycles)
    );
    assert_retained(&paths, &issued, &review, EffectState::Intent);

    let mut recovered = platform();
    let lost_observation = apply(&review.review_sha256, &mut recovered);
    assert!(
        matches!(lost_observation, Err(EnsureWorkflowError::Platform(_))),
        "{lost_observation:?}"
    );
    assert!(input.adapter_root.join("lost-funded-observation").is_file());
    assert_eq!(
        withdrawals(input),
        1,
        "duplicate Ledger request cannot deposit twice"
    );
    assert_retained(&paths, &issued, &review, EffectState::Issued);
    assert_eq!(
        ledger_account_balance(input.pic, input.cycles_ledger, input.operator),
        after
    );

    let mut receipt_recovery = platform();
    let completed = apply(&review.review_sha256, &mut receipt_recovery).unwrap();
    assert!(completed.terminal);
    let terminal = read_journal(&paths).unwrap().unwrap();
    assert_eq!(terminal.operation_id, issued.operation_id);
    assert_eq!(
        terminal.initial_operator_cycles,
        issued.initial_operator_cycles
    );
    assert_eq!(
        terminal.initial_controlled_cycles,
        issued.initial_controlled_cycles
    );
    assert_eq!(
        terminal.initial_estate_funding_cycles_by_root,
        issued.initial_estate_funding_cycles_by_root
    );
    assert_eq!(
        &terminal.effects[..issued.effects.len() - 1],
        &issued.effects[..issued.effects.len() - 1]
    );
    assert_eq!(
        terminal.effects[issued.effects.len() - 1].action_sha256,
        pause.provisioning_action_sha256
    );
    assert_eq!(terminal.funding_reviews.len(), 1);
    let effect = terminal.funding_reviews[0].effect.as_ref().unwrap();
    assert_eq!(effect.state, EffectState::Applied);
    assert!(effect.receipt.is_some());
    assert_eq!(
        effect.post_cycles,
        effect
            .pre_cycles
            .map(|value| value - pause.shortfall_cycles - pause.ledger_fee_cycles)
    );
    assert_eq!(withdrawals(input), 1);
    let actual = completed.actual_conservation.as_ref().unwrap();
    assert_eq!(
        actual.observed_starting_cycles,
        issued.initial_controlled_cycles
    );
    assert_eq!(
        actual.received_new_funding_cycles,
        pause.shortfall_cycles + actual.estate_funding_cycles
    );
    assert_eq!(
        actual.estate_funding_cycles,
        planned.plan.conservation.estate_funding_domains[0].maximum_funding_cycles
    );
    assert_eq!(
        actual.operator_debit_cycles,
        actual.received_new_funding_cycles + actual.exact_unavoidable_fee_cycles
    );
    assert_eq!(
        actual.observed_starting_cycles
            + actual.operator_debit_cycles
            + actual.observed_net_cycle_credit_cycles,
        actual.final_controlled_cycles
            + actual.observed_net_cycle_debit_cycles
            + actual.exact_unavoidable_fee_cycles
            + actual.exact_estate_creation_fee_cycles,
    );
    let pool = root_pool_status_as(input.pic, input.root, input.operator);
    assert_eq!(
        (pool.workload, pool.ready, pool.failed, pool.pending_reset),
        (if child.is_some() { 2 } else { 1 }, 1, 0, 0)
    );
    if let Some((operation_id, canister)) = child {
        let recovered = child_status(input, operation_id);
        assert!(recovered.allocation.last_failure.is_none());
        assert_eq!(
            recovered.allocation.creation.unwrap().canister,
            Some(canister)
        );
    }
    let requests: u64 = input
        .pic
        .query_candid(input.cycles_ledger, "request_count", ())
        .unwrap();
    let mut replay_adapter = platform();
    let replay = apply(&planned.plan.plan_sha256, &mut replay_adapter).unwrap();
    assert!(replay.terminal);
    assert_eq!(replay.effects_applied, 0);
    assert_eq!(withdrawals(input), 1);
    assert_eq!(
        ledger_account_balance(input.pic, input.cycles_ledger, input.operator),
        after
    );
    let replay_requests: u64 = input
        .pic
        .query_candid(input.cycles_ledger, "request_count", ())
        .unwrap();
    assert_eq!(requests, replay_requests);
    assert_eq!(read_journal(&paths).unwrap().unwrap(), terminal);
    let replay_pool = root_pool_status_as(input.pic, input.root, input.operator);
    for before in &pool.entries {
        let after = replay_pool
            .entries
            .iter()
            .find(|entry| entry.canister_id == before.canister_id)
            .unwrap();
        assert_eq!(after.status, before.status);
        assert_eq!(after.creation_receipt, before.creation_receipt);
    }
}

fn burn_to(input: &AutonomousFundingJourney<'_>, retain: u128) {
    let burned: u128 = input.pic.update_candid_as_or_panic(
        input.root,
        input.operator,
        "audit_recovery_balance",
        (retain,),
    );
    assert!(burned > 0);
}

fn child_status(
    input: &AutonomousFundingJourney<'_>,
    operation_id: [u8; 32],
) -> canic_control_plane::dto::root::RootComponentChildOperationStatus {
    let response: Result<RootStatusResponseFragment, Error> = input
        .pic
        .query_candid_as(
            input.root,
            input.operator,
            canic::protocol::CANIC_ROOT_OPERATION_STATUS,
            (RootStatusRequestFragment::Operation(
                OperationStatusRequest { operation_id },
            ),),
        )
        .unwrap();
    let RootStatusResponseFragment::Operation(RootOperationStatusResponse::ProvisionChild(status)) =
        response.unwrap()
    else {
        panic!("child operation");
    };
    status
}

fn block_initial_child(
    input: &AutonomousFundingJourney<'_>,
    plan: &FleetEnsurePlan,
    journal: &canic_host::fleet_ensure::model::FleetEnsureJournalRecord,
) -> ([u8; 32], Principal) {
    assert!(!input.pic.auto_progress_enabled());
    let action = planned_actions(plan)
        .into_iter()
        .find(|action| action_sha256(action) == journal.effects.last().unwrap().action_sha256)
        .unwrap();
    let EnsureAction::FleetProtocol {
        action, principal, ..
    } = action
    else {
        panic!("provisioning")
    };
    let CurrentFleetProtocolAction::ProvisionComponents { request, .. } = action.as_ref() else {
        panic!("provisioning")
    };
    let coordinator = Principal::from_text(principal).unwrap();
    for attempt in 0..240 {
        let response: Result<RootStatusResponseFragment, Error> = input
            .pic
            .query_candid_as(
                input.root,
                input.operator,
                canic::protocol::CANIC_ROOT_OPERATION_STATUS,
                (RootStatusRequestFragment::Operation(
                    OperationStatusRequest {
                        operation_id: request.operation_id,
                    },
                ),),
            )
            .unwrap();
        if let Ok(RootStatusResponseFragment::Operation(
            RootOperationStatusResponse::ProvisionComponents(status),
        )) = response
            && status.component_count > 0
            && status.registry_committed_component_count == status.component_count
        {
            assert!(
                !status.root_runtime_active,
                "lower reserve before initial children finish"
            );
            break;
        }
        assert!(attempt < 239, "parent commit");
        input.pic.advance_time(Duration::from_secs(1));
        input.pic.tick();
    }
    burn_to(input, 800_000_000_000);
    for _ in 0..240 {
        let response: Result<CoordinatorOperationReadResponse, Error> = input
            .pic
            .query_candid_as(
                coordinator,
                input.operator,
                canic::protocol::CANIC_COORDINATOR_OPERATION_STATUS,
                (CoordinatorOperationReadRequest::Operation(
                    OperationStatusRequest {
                        operation_id: request.operation_id,
                    },
                ),),
            )
            .unwrap();
        let CoordinatorOperationReadResponse::Operation(
            CoordinatorOperationStatusResponse::ComponentProvisioning(status),
        ) = response.unwrap()
        else {
            panic!("provisioning status")
        };
        if let Some(origin) = status.pending_root_failure.and_then(|failure| failure.origin)
            && origin.stage == canic::dto::component_provisioning::ProvisioningFailureStage::ComponentChildAllocation {
            assert_eq!(origin.diagnostic_code, canic_core::diagnostics::codes::DEPLOYMENT_CYCLE_RESERVE_REQUIRED.raw_code().raw());
            assert_eq!(origin.target, input.root);
            let status = child_status(input, origin.operation_id);
            assert_eq!(status.allocation.phase, RootComponentAllocationPhase::Reserved);
            assert!(status.allocation.creation.is_none());
            assert!(status.allocation.installation.is_none());
            let claim = root_pool_status_as(input.pic, input.root, input.operator).entries.into_iter().find(|entry|
                matches!(&entry.status, CanisterPoolAssetStatus::Claimed { claim } if claim.operation_id == origin.operation_id)
            ).unwrap();
            return (origin.operation_id, claim.canister_id);
        }
        input.pic.advance_time(Duration::from_secs(1));
        input.pic.tick();
    }
    panic!("initial child must retain E163");
}

fn withdrawals(input: &AutonomousFundingJourney<'_>) -> u64 {
    input
        .pic
        .query_candid(input.cycles_ledger, "withdrawal_count", ())
        .unwrap()
}

fn assert_retained(
    paths: &EnsurePaths,
    issued: &canic_host::fleet_ensure::model::FleetEnsureJournalRecord,
    review: &FundingReviewRecord,
    state: EffectState,
) {
    let journal = read_journal(paths).unwrap().unwrap();
    assert_eq!(journal.effects, issued.effects);
    assert_eq!(
        journal.initial_operator_cycles,
        issued.initial_operator_cycles
    );
    assert_eq!(
        journal.initial_controlled_cycles,
        issued.initial_controlled_cycles
    );
    assert_eq!(journal.funding_reviews.len(), 1);
    assert_eq!(journal.funding_reviews[0].action, review.action);
    assert_eq!(
        journal.funding_reviews[0].review_sha256,
        review.review_sha256
    );
    let effect = journal.funding_reviews[0].effect.as_ref().unwrap();
    assert_eq!(effect.state, state);
    assert_eq!(effect.receipt.is_some(), state == EffectState::Issued);
}
