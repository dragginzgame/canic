//! Module: pic::fleet_registry::baseline::tests::sibling_funding
//!
//! Responsibility: qualify caller-scoped funding identities against a real shared Hub.
//! Boundary: disposable Shards drive the production attempt and RPC; no transfer is mocked.

use super::*;
use canic::dto::{
    observability::{CanisterObservabilityResponse, ChildFundingUsage},
    rpc::CyclesResponse,
};

const GRANT: u128 = 1_000_000_000_000;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "keep the ordered PocketIC scenario and its authority assertions together"
)]
pub(super) fn sibling_topups_retain_distinct_receipts_after_both_replies_are_lost() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let workspace =
        crate::workspace::discover().expect("discover Canic test workspace through Cargo");
    let config_path = workspace.join("apps/test/test-configs/fixture-sibling-funding.toml");
    let config = AppConfigSnapshot::load(&config_path).unwrap();
    let pic = build_pic();
    let coordinator = pic.create_canister();
    pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
    let root_wasm = crate::pic::artifacts::build_generated_fleet_wasm(
        &workspace,
        &config_path,
        "root",
        CanicWasmBuildProfile::Fast,
    );
    let components = build_component_fixture_wasms(
        &workspace,
        &config_path,
        "fleet-sibling-funding",
        &[
            ("user_hub", "canister_user_hub"),
            ("user_shard", "canister_user_shard"),
        ],
    );
    let fixture =
        install_fixture_root(&pic, coordinator, &config_path, &[], root_wasm, &components);
    super::state_cascade::activate_components(&pic, coordinator, &fixture, &config);
    let root = fixture.root_id;
    let workloads = root_pool_status(&pic, root)
        .entries
        .into_iter()
        .filter(|entry| matches!(entry.status, CanisterPoolAssetStatus::Workload { .. }))
        .map(|entry| entry.canister_id)
        .collect::<Vec<_>>();
    let hub = workloads.iter().copied().find(|&canister| {
        matches!(managed_binding_status(&pic, root, canister), ManagedCanisterBinding::Component(binding) if binding.role.as_str() == "user_hub")
    }).unwrap();
    let children = workloads.iter().copied().filter(|&canister| {
        matches!(managed_binding_status(&pic, root, canister), ManagedCanisterBinding::ComponentChild(binding) if binding.role.as_str() == "user_shard")
    }).collect::<Vec<_>>();
    let [first, second] = children.as_slice() else {
        panic!("exactly two configured siblings");
    };
    let (first, second) = (*first, *second);
    assert_ne!(first, second);
    let mut operations = Vec::new();
    for child in [first, second] {
        assert_eq!(usage(&pic, root, hub, child).accounted_cycles.to_u128(), 0);
        let (operation, response) = request(&pic, root, child, true);
        assert!(
            response.is_none(),
            "the fixture must discard each delivered reply"
        );
        operations.push(operation);
        assert_eq!(
            usage(&pic, root, hub, child).accounted_cycles.to_u128(),
            GRANT
        );
    }
    assert_ne!(operations[0], operations[1]);
    for (child, operation) in [first, second].into_iter().zip(&operations) {
        let before = pic.cycle_balance(child);
        let (retry, response) = request(&pic, root, child, false);
        assert_eq!(&retry, operation);
        assert_eq!(
            response,
            Some(CyclesResponse::Transferred {
                cycles_transferred: GRANT
            })
        );
        assert!(
            pic.cycle_balance(child) <= before,
            "replay cannot credit a second transfer"
        );
        let observed = usage(&pic, root, hub, child);
        assert_eq!(observed.accounted_cycles.to_u128(), GRANT);
        assert_eq!(observed.pending_operations, 0);
        assert_eq!(observed.reserved_cycles, Some(0.into()));
    }
    let collision: Result<CyclesResponse, Error> = ic_testkit::pic::update_wire(
        &pic,
        second,
        root,
        crate::pic::application_probe::TOPUP_COLLISION,
        &crate::pic::application_probe::TopupCollision {
            cycles: GRANT,
            operation_id: operations[0],
        },
    )
    .expect("update Wire transport");
    assert_eq!(
        collision.unwrap_err().code(),
        canic_contracts::diagnostics::codes::CODEC_CONFLICT.raw_code()
    );
    for child in [first, second] {
        assert_eq!(
            usage(&pic, root, hub, child).accounted_cycles.to_u128(),
            GRANT
        );
    }
    assert_funding_metrics(&pic, root, hub, &[first, second]);
    qualify_automatic_siblings(&pic, root, hub, [first, second]);
    qualify_terminal_diagnostic(&pic, root, hub, first);
}

fn request(
    pic: &PocketIc,
    root: Principal,
    child: Principal,
    discard: bool,
) -> ([u8; 32], Option<CyclesResponse>) {
    let response: Result<([u8; 32], Option<CyclesResponse>), Error> = ic_testkit::pic::update_wire(
        pic,
        child,
        root,
        crate::pic::application_probe::TOPUP_REQUEST,
        &crate::pic::application_probe::TopupRequest {
            cycles: GRANT,
            discard_reply: discard,
        },
    )
    .expect("update Wire transport");
    response.unwrap()
}

fn usage(pic: &PocketIc, root: Principal, hub: Principal, child: Principal) -> ChildFundingUsage {
    let response: Result<CanisterObservabilityResponse, Error> = ic_testkit::pic::query_wire(
        pic,
        hub,
        root,
        ic_testkit::pic::WireCallSpec {
            method: canic::protocol::child_funding::METHOD,
            method_id: canic::protocol::child_funding::METHOD_ID,
            request_limits: canic::protocol::child_funding::REQUEST_LIMITS,
            response_limits: canic::protocol::child_funding::RESPONSE_LIMITS,
        },
        &ic_platform::Principal::from(child),
    )
    .map(
        |reply: Result<canic::dto::child_funding::ChildFundingUsage, Error>| {
            reply.map(|value| CanisterObservabilityResponse::ChildFunding(value.into()))
        },
    )
    .unwrap();
    let CanisterObservabilityResponse::ChildFunding(usage) = response.unwrap() else {
        panic!("child funding response");
    };
    assert_eq!((usage.parent, usage.child), (hub, child));
    usage
}

fn qualify_terminal_diagnostic(pic: &PocketIc, root: Principal, hub: Principal, child: Principal) {
    let exhausted: Result<([u8; 32], Option<CyclesResponse>), Error> =
        ic_testkit::pic::update_wire(
            pic,
            child,
            root,
            crate::pic::application_probe::TOPUP_EXHAUSTION,
            &1_000_000_u128,
        )
        .expect("update Wire transport");
    assert_eq!(
        exhausted.unwrap_err().code(),
        canic_contracts::diagnostics::codes::PLATFORM_INSUFFICIENT_LIQUID_CYCLES.raw_code()
    );
    // Fund only this disposable fixture to observe the retained failure above reserve.
    pic.add_cycles(child, GRANT);
    let before = terminal_failure(pic, root, child);
    assert_eq!(before.parent, ic_platform::Principal::from(hub));
    assert_eq!(
        before.public_error_code,
        canic_contracts::diagnostics::codes::PLATFORM_INSUFFICIENT_LIQUID_CYCLES
            .raw_code()
            .raw()
    );
    assert_eq!(
        before.disposition,
        canic_contracts::values::cycles::CycleTopupFailureDisposition::Terminal
    );
    let burned: Result<u128, Error> = ic_testkit::pic::update_wire(
        pic,
        child,
        root,
        crate::pic::application_probe::RECOVERY_BALANCE,
        &1_u128,
    )
    .expect("update Wire transport");
    assert!(burned.unwrap() > 0);
    let rejected = pic
        .update_call(
            child,
            root,
            "test_topup_request",
            ic_wire::native::encode_request(
                crate::pic::application_probe::TOPUP_REQUEST.method_id,
                &crate::pic::application_probe::TopupRequest {
                    cycles: GRANT,
                    discard_reply: false,
                },
                crate::pic::application_probe::TOPUP_REQUEST.request_limits,
            )
            .unwrap(),
        )
        .unwrap_err();
    assert_eq!(
        rejected.error_code,
        ic_platform::pocket_ic::ErrorCode::CanisterOutOfCycles
    );
    assert_eq!(terminal_failure(pic, root, child), before);
}

fn terminal_failure(
    pic: &PocketIc,
    root: Principal,
    child: Principal,
) -> canic::dto::cycle_topups::CycleTopupFailure {
    fixture_topup_events(pic, root, child)
        .into_iter()
        .rev()
        .find_map(|event| {
            assert!(event.timestamp_secs > 0);
            event.parent_failure
        })
        .expect("retained protected failure")
}

fn assert_funding_metrics(pic: &PocketIc, root: Principal, hub: Principal, children: &[Principal]) {
    use canic::dto::{
        metrics::{MetricValue, MetricsKind},
        page::PageRequest,
        role::MetricsStatusRequest,
    };
    let response: Result<CanisterObservabilityResponse, Error> = ic_testkit::pic::query_wire(
        pic,
        hub,
        root,
        ic_testkit::pic::WireCallSpec {
            method: canic::protocol::protected_metrics::METHOD,
            method_id: canic::protocol::protected_metrics::METHOD_ID,
            request_limits: canic::protocol::protected_metrics::REQUEST_LIMITS,
            response_limits: canic::protocol::protected_metrics::RESPONSE_LIMITS,
        },
        &MetricsStatusRequest {
            kind: MetricsKind::Core,
            page: PageRequest {
                offset: 0,
                limit: 100,
            },
        },
    )
    .map(
        |reply: Result<canic::dto::page::Page<canic::dto::metric_page::MetricEntry>, Error>| {
            reply.map(|value| CanisterObservabilityResponse::Metrics(value.into()))
        },
    )
    .unwrap();
    let CanisterObservabilityResponse::Metrics(page) = response.unwrap() else {
        panic!("metrics response");
    };
    assert_eq!(page.entries.len() as u64, page.total);
    for child in children {
        for label in ["cycles_requested_by_child", "cycles_granted_to_child"] {
            let entry = page
                .entries
                .iter()
                .find(|entry| {
                    entry.principal == Some(*child) && entry.labels == ["cycles_funding", label]
                })
                .unwrap();
            assert!(matches!(entry.value, MetricValue::U128 { value, .. } if value == GRANT));
        }
        assert!(!page.entries.iter().any(|entry| {
            entry.principal == Some(*child)
                && entry
                    .labels
                    .get(1)
                    .is_some_and(|label| label == "cycles_denied_to_child")
        }));
    }
}

fn qualify_automatic_siblings(
    pic: &PocketIc,
    root: Principal,
    hub: Principal,
    children: [Principal; 2],
) {
    for child in children {
        let response: Result<(), Error> = ic_testkit::pic::update_wire(
            pic,
            child,
            root,
            crate::pic::application_probe::TOPUP_DEMAND,
            &1_900_000_000_000_u128,
        )
        .expect("update Wire transport");
        response.unwrap();
    }
    for _ in 0..120 {
        if children.iter().all(|child| {
            let funding = usage(pic, root, hub, *child);
            let granted = fixture_topup_events(pic, root, *child).iter().any(|event| {
                event
                    .transferred_cycles
                    .as_ref()
                    .is_some_and(|cycles| cycles.to_u128() == GRANT)
            });
            funding.accounted_cycles.to_u128() == 2 * GRANT
                && funding.pending_operations == 0
                && granted
        }) {
            break;
        }
        pic.advance_time(Duration::from_secs(1));
        pic.tick();
    }
    for child in children {
        assert_eq!(
            usage(pic, root, hub, child).accounted_cycles.to_u128(),
            2 * GRANT
        );
        assert!(pic.cycle_balance(child) > 2 * GRANT);
        let events = fixture_topup_events(pic, root, child);
        let grants = events
            .iter()
            .filter_map(|event| event.transferred_cycles.as_ref())
            .map(Cycles::to_u128)
            .collect::<Vec<_>>();
        assert_eq!(grants, [GRANT], "one production timer grant per sibling");
    }
}
