//! Module: pic::fleet_registry::baseline::tests::funding_deadline
//!
//! Responsibility: qualify the funding owner's response to real outgoing child grants.
//! Boundary: the audit Root only prepares balance; maintained commands execute all funding.

use super::*;
use canic::dto::child_funding::ChildFundingUsage;
use canic::dto::rpc::CyclesFundingPreflightResponse;
use canic::dto::rpc::CyclesResponse;
use canic::dto::runtime::CanisterTimerStatus;
use canic::dto::state::SetCyclesFundingRequest;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one real grant journey binds deadline correction, upstream demand, replay and the retained child budget"
)]
pub(super) fn child_grant_refreshes_root_funding_deadline_without_repeating_credit() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let workspace =
        crate::workspace::discover().expect("discover Canic test workspace through Cargo");
    let config_path = initial_shard_root_canister_config_path(&workspace);
    let config = AppConfigSnapshot::load(&config_path).unwrap();
    let pic = build_pic();
    let coordinator = pic.create_canister().expect("PocketIC create_canister");
    pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES)
        .expect("PocketIC add_cycles");
    let fixture = install_fixture_root(
        &pic,
        coordinator,
        &config_path,
        &[],
        crate::pic::fleet_registry::build::build_child_reserve_root_wasm(),
        build_initial_shard_component_wasms(),
    );
    // Read-only grant accounting remains available before Root activation.
    assert_eq!(
        child_usage(&pic, fixture.root_id, Principal::from_slice(&[77])).pending_operations,
        0
    );
    super::state_cascade::activate_components(&pic, coordinator, &fixture, &config);
    let root = fixture.root_id;
    let child = root_pool_status(&pic, root)
        .entries
        .into_iter()
        .filter(|entry| matches!(entry.status, CanisterPoolAssetStatus::Workload { .. }))
        .find_map(
            |entry| match managed_binding_status(&pic, root, entry.canister_id) {
                ManagedCanisterBinding::Component(binding)
                    if binding.role.as_str() == "user_hub" =>
                {
                    Some(entry.canister_id)
                }
                _ => None,
            },
        )
        .expect("managed Hub with the configured 2T request cap and 3T total allowance");
    let initial_usage = child_usage(&pic, root, child);
    assert_eq!(initial_usage.accounted_cycles.to_u128(), 0);
    let denied: Result<canic::dto::child_funding::ChildFundingUsage, Error> =
        ic_testkit::pic::query_wire(
            &pic,
            root,
            Principal::from_slice(&[99; 29]),
            ic_testkit::pic::WireCallSpec {
                method: canic::protocol::child_funding::METHOD,
                method_id: canic::protocol::child_funding::METHOD_ID,
                request_limits: canic::protocol::child_funding::REQUEST_LIMITS,
                response_limits: canic::protocol::child_funding::RESPONSE_LIMITS,
            },
            &ic_platform::Principal::from(child),
        )
        .unwrap();
    assert!(denied.is_err());
    funding(&pic, root, false);
    let burned: u128 = ic_testkit::pic::update_wire(
        &pic,
        root,
        Principal::anonymous(),
        crate::pic::application_probe::AUDIT_ROOT_BALANCE,
        &11_000_000_000_000_u128,
    )
    .expect("update Candid harness call");
    assert!(burned > 0);
    // The unchanged pause records the prepared sample. Restoration therefore
    // schedules from stable headroom, not from the audit-only burn itself.
    funding(&pic, root, false);
    funding(&pic, root, true);
    let before = timer(&pic, root);
    let now = pic
        .get_time()
        .expect("PocketIC get_time")
        .as_nanos_since_unix_epoch();
    assert!(before.next_due_at_ns.unwrap() > now + 3_000_000_000_000);
    let prior_grants = root_funding_status(&pic, root).automatic_grants;
    let balance_before = pic.cycle_balance(child).expect("PocketIC cycle_balance");
    let request = descendant_funding_request(&pic, 0x94);
    let granted = request_descendant_funding(&pic, root, child, request.clone());
    assert_eq!(granted, 2_000_000_000_000);
    let usage = child_usage(&pic, root, child);
    assert_eq!(usage.accounted_cycles.to_u128(), granted);
    assert_eq!(usage.pending_operations, 0);
    assert_eq!(usage.reserved_cycles, Some(0.into()));
    super::replay_release::assert_funding_receipt(&pic, root, child, request.metadata.request_id);
    let _ = super::replay_release::collect_intents(&pic, coordinator);
    assert!(pic.cycle_balance(child).expect("PocketIC cycle_balance") > balance_before);
    let after = timer(&pic, root);
    assert!(
        after
            .next_due_at_ns
            .is_some_and(|deadline| Some(deadline) < before.next_due_at_ns)
            || after.executions_since_runtime_start > before.executions_since_runtime_start,
        "a real grant crossing the reserve must refresh the existing timer: before={before:?}, after={after:?}"
    );
    for _ in 0..30 {
        if root_funding_status(&pic, root).automatic_grants > prior_grants {
            break;
        }
        pic.advance_time(Duration::from_secs(1))
            .expect("PocketIC advance_time");
        pic.tick().expect("PocketIC tick");
    }
    let funded = root_funding_status(&pic, root);
    assert_eq!(funded.automatic_grants, prior_grants + 1);
    assert!(pic.cycle_balance(root).expect("PocketIC cycle_balance") > 10_000_000_000_000);
    let child_after = pic.cycle_balance(child).expect("PocketIC cycle_balance");
    assert_eq!(
        request_descendant_funding(&pic, root, child, request),
        granted
    );
    assert!(
        pic.cycle_balance(child).expect("PocketIC cycle_balance") <= child_after,
        "receipt replay cannot repeat the deposit"
    );
    assert_eq!(
        child_usage(&pic, root, child).accounted_cycles,
        usage.accounted_cycles
    );

    pic.advance_time(Duration::from_secs(2))
        .expect("PocketIC advance_time");
    pic.tick().expect("PocketIC tick");
    let earlier_deadline = timer(&pic, root).next_due_at_ns.unwrap();
    let remaining =
        request_descendant_funding(&pic, root, child, descendant_funding_request(&pic, 0x95));
    assert_eq!(remaining, 1_000_000_000_000);
    assert_eq!(
        child_usage(&pic, root, child).accounted_cycles.to_u128(),
        granted + remaining
    );
    let deadline = timer(&pic, root).next_due_at_ns.unwrap();
    assert!(
        deadline <= earlier_deadline,
        "a transfer cannot postpone an earlier safety check"
    );
    let now = pic
        .get_time()
        .expect("PocketIC get_time")
        .as_nanos_since_unix_epoch();
    assert!(
        deadline > now + 3_000_000_000_000,
        "an internal grant must not become an estimated computation burn rate"
    );
    pic.advance_time(Duration::from_secs(2))
        .expect("PocketIC advance_time");
    pic.tick().expect("PocketIC tick");
    assert!(matches!(
        descendant_funding_response(&pic, root, child, descendant_funding_request(&pic, 0x96)),
        CyclesResponse::PreflightRejected(CyclesFundingPreflightResponse::ChildBudgetExhausted {
            remaining_child_budget: 0,
            max_per_child: 3_000_000_000_000,
        })
    ));
    assert_eq!(
        root_funding_status(&pic, root).automatic_grants,
        prior_grants + 1
    );
}

fn child_usage(pic: &PocketIc, parent: Principal, child: Principal) -> ChildFundingUsage {
    let response: Result<canic::dto::child_funding::ChildFundingUsage, Error> =
        ic_testkit::pic::query_wire(
            pic,
            parent,
            Principal::anonymous(),
            ic_testkit::pic::WireCallSpec {
                method: canic::protocol::child_funding::METHOD,
                method_id: canic::protocol::child_funding::METHOD_ID,
                request_limits: canic::protocol::child_funding::REQUEST_LIMITS,
                response_limits: canic::protocol::child_funding::RESPONSE_LIMITS,
            },
            &ic_platform::Principal::from(child),
        )
        .expect("child funding Wire transport");
    let value: ChildFundingUsage = response.expect("child funding application result");
    assert_eq!((value.parent, value.child), (parent.into(), child.into()));
    value
}

fn funding(pic: &PocketIc, root: Principal, enabled: bool) {
    let response = crate::pic::protected_state::set_funding(
        pic,
        root,
        Principal::anonymous(),
        SetCyclesFundingRequest { enabled },
    )
    .unwrap();
    assert_eq!(response.change.current, enabled);
    assert!(response.reconciliation_error.is_none());
    assert_eq!(response.propagation.unconfirmed_targets, 0);
}

fn timer(pic: &PocketIc, root: Principal) -> CanisterTimerStatus {
    let runtime = root_runtime(pic, root);
    runtime
        .timers
        .into_iter()
        .find(|timer| {
            timer.owner == "canic" && timer.subsystem == "cycles" && timer.name == "topup"
        })
        .expect("the sole native funding timer")
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one installed external-credit journey binds retained demand, zero-transfer replay and later automatic funding"
)]
pub(super) fn external_credit_preserves_root_funding_observation_and_replay() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let workspace = crate::workspace::discover().unwrap();
    let config_path = initial_shard_root_canister_config_path(&workspace);
    let config = AppConfigSnapshot::load(&config_path).unwrap();
    let pic = build_pic();
    let coordinator = pic.create_canister().unwrap();
    pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES)
        .unwrap();
    let fixture = install_fixture_root(
        &pic,
        coordinator,
        &config_path,
        &[],
        crate::pic::fleet_registry::build::build_child_reserve_root_wasm(),
        build_initial_shard_component_wasms(),
    );
    super::state_cascade::activate_components(&pic, coordinator, &fixture, &config);
    let root = fixture.root_id;
    funding(&pic, root, false);
    let before = root_funding_status(&pic, root);
    assert!(before.current_operation.is_none());
    let treasury_before = crate::pic::canic::coordinator_funding(&pic, coordinator).unwrap();
    let low_balance = before.root_policy.request_threshold.to_u128() / 2;
    burn_to(&pic, root, low_balance);
    pic.stop_canister(coordinator, None).unwrap();
    funding(&pic, root, true);
    let pending = await_funding_state(&pic, root, |status| status.current_operation.is_some());
    let request = pending.current_operation.unwrap();
    assert!(request.observed_balance.to_u128() <= before.root_policy.request_threshold.to_u128());
    // The retained request has observed low balance; external credit arrives before acceptance.
    pic.add_cycles(root, before.root_policy.target_balance.to_u128())
        .unwrap();
    let credited = pic.cycle_balance(root).unwrap();
    pic.start_canister(coordinator, None).unwrap();
    let relieved = await_funding_state(&pic, root, |status| {
        matches!(&status.last_result, Some(FleetRootFundingResponse::NoGrant(receipt))
            if receipt.request == request && receipt.reason == canic::dto::fleet_funding::FleetRootFundingNoGrantReason::RootBalanceRelieved)
    });
    assert!(relieved.current_operation.is_none());
    assert_eq!(relieved.automatic_grants, before.automatic_grants);
    assert_eq!(relieved.automatic_cycles, before.automatic_cycles);
    assert_eq!(relieved.policy_hash, before.policy_hash);
    assert_eq!(relieved.policy_generation, before.policy_generation);
    // An outstanding call refund may arrive after the credit sample. The exact
    // grant counters above must stay unchanged, and no full grant may arrive.
    assert!(pic.cycle_balance(root).unwrap() < credited + request.requested_cycles.to_u128());
    let treasury = crate::pic::canic::coordinator_funding(&pic, coordinator).unwrap();
    assert_eq!(treasury.automatic_grants, treasury_before.automatic_grants);
    assert_eq!(treasury.automatic_cycles, treasury_before.automatic_cycles);
    let terminal = relieved.last_result.unwrap();
    assert_eq!(
        replay_funding(&pic, coordinator, root, request.clone()),
        terminal
    );
    assert_eq!(root_funding_status(&pic, root).last_result, Some(terminal));
    let observation = timer(&pic, root);
    assert!(
        observation.next_due_at_ns.is_some(),
        "relief must retain future observation"
    );
    assert_eq!(
        root_runtime(&pic, root)
            .timers
            .iter()
            .filter(|timer| {
                timer.owner == "canic" && timer.subsystem == "cycles" && timer.name == "topup"
            })
            .count(),
        1
    );
    // Several observations without renewed demand must not spend another grant.
    for _ in 0..3 {
        advance_to_funding_observation(&pic, root);
        assert_eq!(
            root_funding_status(&pic, root).automatic_grants,
            before.automatic_grants
        );
    }
    burn_to(&pic, root, low_balance);
    advance_to_funding_observation(&pic, root);
    let funded = await_funding_state(&pic, root, |status| {
        status.automatic_grants > before.automatic_grants
    });
    assert_eq!(funded.automatic_grants, before.automatic_grants + 1);
    let Some(FleetRootFundingResponse::Granted(receipt)) = funded.last_result else {
        panic!("renewed demand must receive one grant");
    };
    assert_eq!(
        receipt.request.operation_sequence,
        request.operation_sequence + 1
    );
    let renewed_request = FleetRootFundingRequest {
        operation_id: receipt.request.operation_id,
        operation_sequence: receipt.request.operation_sequence,
        expected_registry: receipt.request.expected_registry.clone(),
        observed_balance: receipt.request.observed_balance.clone(),
        requested_cycles: receipt.request.granted_cycles.clone(),
        policy_hash: receipt.request.policy_hash,
    };
    let balance_after = pic.cycle_balance(root).unwrap();
    assert_eq!(
        replay_funding(&pic, coordinator, root, renewed_request),
        FleetRootFundingResponse::Granted(receipt)
    );
    assert!(pic.cycle_balance(root).unwrap() <= balance_after);
    assert_eq!(
        root_funding_status(&pic, root).automatic_grants,
        before.automatic_grants + 1
    );
}

fn burn_to(pic: &PocketIc, root: Principal, balance: u128) {
    let burned: u128 = ic_testkit::pic::update_wire(
        pic,
        root,
        Principal::anonymous(),
        crate::pic::application_probe::AUDIT_ROOT_BALANCE,
        &balance,
    )
    .unwrap();
    assert!(burned > 0);
}

fn await_funding_state(
    pic: &PocketIc,
    root: Principal,
    ready: impl Fn(&RootFundingStatusResponse) -> bool,
) -> RootFundingStatusResponse {
    for _ in 0..64 {
        let status = root_funding_status(pic, root);
        if ready(&status) {
            return status;
        }
        pic.advance_time(Duration::from_mins(1)).unwrap();
        pic.tick().unwrap();
    }
    panic!(
        "funding did not converge: {:?}",
        root_funding_status(pic, root)
    );
}

fn advance_to_funding_observation(pic: &PocketIc, root: Principal) {
    let due = timer(pic, root)
        .next_due_at_ns
        .expect("retained funding observation");
    let now = pic.get_time().unwrap().as_nanos_since_unix_epoch();
    pic.advance_time(Duration::from_nanos(
        due.saturating_sub(now).saturating_add(1_000_000_000),
    ))
    .unwrap();
    for _ in 0..8 {
        pic.tick().unwrap();
    }
}

fn replay_funding(
    pic: &PocketIc,
    coordinator: Principal,
    root: Principal,
    request: FleetRootFundingRequest,
) -> FleetRootFundingResponse {
    use canic::protocol::funding_control as wire;
    let response: Result<canic::dto::funding_control::Response, Error> =
        ic_testkit::pic::update_wire(
            pic,
            coordinator,
            root,
            ic_testkit::pic::WireCallSpec {
                method: wire::request::METHOD,
                method_id: wire::request::METHOD_ID,
                request_limits: wire::REQUEST_LIMITS,
                response_limits: wire::RESPONSE_LIMITS,
            },
            &canic::dto::funding_control::Request::from(request),
        )
        .unwrap();
    response.unwrap().into()
}
