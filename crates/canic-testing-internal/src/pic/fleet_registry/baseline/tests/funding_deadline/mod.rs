//! Module: pic::fleet_registry::baseline::tests::funding_deadline
//!
//! Responsibility: qualify the funding owner's response to real outgoing child grants.
//! Boundary: the audit Root only prepares balance; maintained commands execute all funding.

use super::*;
use canic::dto::{
    observability::{
        CanisterObservabilityRequest, CanisterObservabilityResponse, ChildFundingUsage,
    },
    rpc::{CyclesFundingPreflightResponse, CyclesResponse},
    runtime::CanisterTimerStatus,
    state::{FleetStateCommandResult, SetCyclesFundingRequest},
};

#[derive(CandidType)]
enum Command {
    SetCyclesFunding(SetCyclesFundingRequest),
}

#[derive(CandidType, Deserialize)]
enum Response {
    SetCyclesFunding(FleetStateCommandResult<bool>),
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one real grant journey binds deadline correction, upstream demand, replay and the retained child budget"
)]
pub(super) fn child_grant_refreshes_root_funding_deadline_without_repeating_credit() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
    let config_path = initial_shard_root_canister_config_path(&workspace);
    let config = AppConfigSnapshot::load(&config_path).unwrap();
    let pic = build_pic();
    let coordinator = pic.create_canister();
    pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
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
    let denied: Result<CanisterObservabilityResponse, Error> = pic
        .query_candid_as(
            root,
            Principal::from_slice(&[99; 29]),
            canic::protocol::CANIC_OBSERVABILITY,
            (CanisterObservabilityRequest::ChildFunding(child),),
        )
        .unwrap();
    assert!(denied.is_err());
    funding(&pic, root, false);
    let burned: u128 = pic.update_candid_as_or_panic(
        root,
        Principal::anonymous(),
        "audit_recovery_balance",
        (11_000_000_000_000_u128,),
    );
    assert!(burned > 0);
    // The unchanged pause records the prepared sample. Restoration therefore
    // schedules from stable headroom, not from the audit-only burn itself.
    funding(&pic, root, false);
    funding(&pic, root, true);
    let before = timer(&pic, root);
    let now = pic.get_time().as_nanos_since_unix_epoch();
    assert!(before.next_due_at_ns.unwrap() > now + 3_000_000_000_000);
    let prior_grants = root_funding_status(&pic, root).automatic_grants;
    let balance_before = pic.cycle_balance(child);
    let request = descendant_funding_request(&pic, 0x94);
    let granted = request_descendant_funding(&pic, root, child, request.clone());
    assert_eq!(granted, 2_000_000_000_000);
    let usage = child_usage(&pic, root, child);
    assert_eq!(usage.accounted_cycles.to_u128(), granted);
    assert_eq!(usage.pending_operations, 0);
    assert_eq!(usage.reserved_cycles, Some(0.into()));
    assert!(pic.cycle_balance(child) > balance_before);
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
        pic.advance_time(Duration::from_secs(1));
        pic.tick();
    }
    let funded = root_funding_status(&pic, root);
    assert_eq!(funded.automatic_grants, prior_grants + 1);
    assert!(pic.cycle_balance(root) > 10_000_000_000_000);
    let child_after = pic.cycle_balance(child);
    assert_eq!(
        request_descendant_funding(&pic, root, child, request),
        granted
    );
    assert!(
        pic.cycle_balance(child) <= child_after,
        "receipt replay cannot repeat the deposit"
    );
    assert_eq!(
        child_usage(&pic, root, child).accounted_cycles,
        usage.accounted_cycles
    );

    pic.advance_time(Duration::from_secs(2));
    pic.tick();
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
    let now = pic.get_time().as_nanos_since_unix_epoch();
    assert!(
        deadline > now + 3_000_000_000_000,
        "an internal grant must not become an estimated computation burn rate"
    );
    pic.advance_time(Duration::from_secs(2));
    pic.tick();
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
    let response: Result<CanisterObservabilityResponse, Error> = pic
        .query_candid_as(
            parent,
            Principal::anonymous(),
            canic::protocol::CANIC_OBSERVABILITY,
            (CanisterObservabilityRequest::ChildFunding(child),),
        )
        .unwrap();
    let CanisterObservabilityResponse::ChildFunding(value) = response.unwrap() else {
        panic!("exact child funding observation");
    };
    assert_eq!((value.parent, value.child), (parent, child));
    value
}

fn funding(pic: &PocketIc, root: Principal, enabled: bool) {
    let response: Result<Response, Error> = pic.update_candid_as_or_panic(
        root,
        Principal::anonymous(),
        canic::protocol::CANIC_ROOT_COMMAND,
        (Command::SetCyclesFunding(SetCyclesFundingRequest {
            enabled,
        }),),
    );
    let Response::SetCyclesFunding(response) = response.unwrap();
    assert_eq!(response.change.current, enabled);
    assert!(response.reconciliation_error.is_none());
    assert_eq!(response.propagation.unconfirmed_targets, 0);
}

fn timer(pic: &PocketIc, root: Principal) -> CanisterTimerStatus {
    let RootStatusResponseFragment::Runtime(runtime) =
        root_status(pic, root, RootStatusRequestFragment::Runtime).unwrap()
    else {
        panic!("runtime response");
    };
    runtime
        .timers
        .into_iter()
        .find(|timer| {
            timer.owner == "canic" && timer.subsystem == "cycles" && timer.name == "topup"
        })
        .expect("the sole native funding timer")
}
