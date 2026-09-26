//! Bounded enumeration tests use passive pages, never simulated management effects.

use super::*;
use canic_core::{
    cdk::types::Cycles,
    dto::pool::{
        CanisterPoolAsset, CanisterPoolAssetOrigin, CanisterPoolClaim, CanisterPoolHandoff,
    },
    ids::ComponentInstanceId,
};

fn id(byte: u8) -> Principal {
    Principal::from_slice(&[byte])
}

fn asset(byte: u8, status: CanisterPoolAssetStatus) -> CanisterPoolAsset {
    CanisterPoolAsset {
        canister_id: id(byte),
        creation_receipt: None,
        cycles: Cycles::from(123_u128),
        origin: CanisterPoolAssetOrigin::Imported,
        status,
        added_at_ns: 1,
        updated_at_ns: 2,
    }
}

fn page() -> CanisterPoolResponse {
    CanisterPoolResponse {
        config: FleetSubnetCanisterPoolConfig {
            minimum_size: 1,
            maximum_size: 4,
            canister_cycles: Cycles::new(100),
            creation_execution_margin: Cycles::new(1),
        },
        tracked: 3,
        store: 1,
        store_deletion_pending: 0,
        pooled: 1,
        workload: 1,
        surplus: 0,
        ready: 1,
        pending_reset: 0,
        claimed: 0,
        recycling: 0,
        handing_off: 0,
        failed: 0,
        completed_handoffs: 0,
        pending_creation: None,
        pending_handoff: None,
        entries: vec![
            asset(1, CanisterPoolAssetStatus::Store),
            asset(
                2,
                CanisterPoolAssetStatus::Workload {
                    claim: CanisterPoolClaim {
                        component: ComponentInstanceId::from_generated_bytes([4; 32]),
                        operation_id: [5; 32],
                    },
                },
            ),
            asset(3, CanisterPoolAssetStatus::Ready),
        ],
        next_start_after: None,
    }
}

fn collector() -> Collector {
    Collector::new(
        id(9),
        BTreeMap::from([
            (id(1), DesiredCanisterKind::Store),
            (id(2), DesiredCanisterKind::Component),
            (id(3), DesiredCanisterKind::Pool),
        ]),
    )
}

#[test]
fn complete_pages_preserve_workload_ownership_and_exclude_cached_balances() {
    let mut first = page();
    let mut second = page();
    first.entries.truncate(2);
    first.next_start_after = Some(id(2));
    second.entries.drain(..2);
    let mut observed = collector();
    assert!(!observed.push(first).unwrap());
    assert!(observed.push(second).unwrap());
    let view = observed.finish().unwrap();
    assert_eq!(
        view.assets[&id(2)].allocation,
        Some(CompletedWorkloadAllocationView {
            component: ComponentInstanceId::from_generated_bytes([4; 32]),
            operation_id: [5; 32]
        })
    );
    let mut changed_cache = page();
    changed_cache.entries[0].cycles = Cycles::from(u128::MAX);
    let mut observed = collector();
    observed.push(changed_cache).unwrap();
    assert_eq!(observed.finish().unwrap(), view);
}

#[test]
fn omitted_extra_replaced_and_wrong_role_assets_reject() {
    for mutation in ["omitted", "extra", "replaced", "role"] {
        let mut input = page();
        match mutation {
            "omitted" => {
                input.entries.pop();
            }
            "extra" => {
                input.entries.push(asset(4, CanisterPoolAssetStatus::Ready));
                input.tracked += 1;
            }
            "replaced" => input.entries[2].canister_id = id(4),
            "role" => input.entries[2].status = CanisterPoolAssetStatus::Store,
            _ => unreachable!(),
        }
        let mut observed = collector();
        let result = observed.push(input).and_then(|_| observed.finish());
        assert!(
            matches!(
                result,
                Err(CompletedMembershipError::Membership { .. }
                    | CompletedMembershipError::Pagination { .. })
            ),
            "{mutation}"
        );
    }
}

#[test]
fn transient_pool_work_and_unfinished_handoffs_reject() {
    for mutation in ["pending", "failed", "claimed-count", "handoff"] {
        let mut input = page();
        match mutation {
            "pending" => input.entries[2].status = CanisterPoolAssetStatus::PendingReset,
            "failed" => input.failed = 1,
            "claimed-count" => input.claimed = 1,
            "handoff" => {
                input.pending_handoff = Some(CanisterPoolHandoff {
                    canister_id: id(3),
                    recipient: id(8),
                    prepared_at_ns: 3,
                });
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(
                collector().push(input),
                Err(CompletedMembershipError::Unsettled { .. })
            ),
            "{mutation}"
        );
    }
}

#[test]
fn pagination_rejects_empty_progress_duplicates_reordering_and_false_cursors() {
    for mutation in [
        "empty",
        "duplicate",
        "order",
        "cursor",
        "terminal-cursor",
        "oversized",
    ] {
        let mut input = page();
        match mutation {
            "empty" => {
                input.entries.clear();
                input.next_start_after = Some(id(2));
            }
            "duplicate" => input.entries[1] = input.entries[0].clone(),
            "order" => input.entries.swap(0, 1),
            "cursor" => {
                input.entries.truncate(2);
                input.next_start_after = Some(id(1));
            }
            "terminal-cursor" => input.next_start_after = Some(id(3)),
            "oversized" => {
                input.entries = vec![input.entries[0].clone(); usize::from(PAGE_SIZE) + 1];
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(
                collector().push(input),
                Err(CompletedMembershipError::Pagination { .. })
            ),
            "{mutation}"
        );
    }
}

#[test]
fn summaries_must_be_exact_and_stable_across_pages() {
    for mutation in ["config", "handoffs", "counts", "pooled", "surplus"] {
        let mut first = page();
        first.entries.truncate(1);
        first.next_start_after = Some(id(1));
        let mut second = page();
        second.entries.drain(..1);
        match mutation {
            "config" => second.config.maximum_size += 1,
            "handoffs" => second.completed_handoffs += 1,
            "counts" => second.workload += 1,
            "pooled" => second.pooled += 1,
            "surplus" => second.surplus += 1,
            _ => unreachable!(),
        }
        let mut observed = collector();
        observed.push(first).unwrap();
        assert!(
            matches!(
                observed.push(second),
                Err(CompletedMembershipError::Pagination { .. })
            ),
            "{mutation}"
        );
    }
    let mut input = page();
    input.workload = 0;
    let mut observed = collector();
    observed.push(input).unwrap();
    assert!(matches!(
        observed.finish(),
        Err(CompletedMembershipError::Pagination { .. })
    ));
}

#[test]
fn finishing_requires_terminal_page_and_does_not_accept_more_pages() {
    let mut input = page();
    input.entries.truncate(1);
    input.next_start_after = Some(id(1));
    let mut observed = collector();
    observed.push(input).unwrap();
    assert!(matches!(
        observed.finish(),
        Err(CompletedMembershipError::Pagination { .. })
    ));
    let mut observed = collector();
    observed.push(page()).unwrap();
    assert!(matches!(
        observed.push(page()),
        Err(CompletedMembershipError::Pagination { .. })
    ));
    assert!(matches!(
        require_fresh(Instant::now().checked_sub(Duration::from_secs(61)).unwrap()),
        Err(CompletedMembershipError::Expired)
    ));
}

#[test]
fn pagination_uses_wire_principal_bytes_even_when_lengths_differ() {
    let mut input = page();
    input.entries[0].canister_id = Principal::from_slice(&[0, 1]);
    let mut observed = collector();
    let kind = observed.expected.remove(&id(1)).unwrap();
    observed.expected.insert(input.entries[0].canister_id, kind);
    observed.push(input).unwrap();
    assert_eq!(observed.finish().unwrap().assets.len(), 3);
}

#[test]
#[ignore = "requires explicit read-only completed-source workspace"]
fn inspect_supplied_completed_pool_query_wire_contract() {
    let workspace = std::env::var("CANIC_COMPLETED_SOURCE_WORKSPACE").unwrap();
    let environment = std::env::var("CANIC_COMPLETED_SOURCE_ENVIRONMENT").unwrap();
    let fleet = std::env::var("CANIC_COMPLETED_SOURCE_FLEET").unwrap();
    let source = inspect_completed_source(Path::new(&workspace), &environment, &fleet).unwrap();
    for (name, binding) in &source.source_protocols {
        if source.inventory.canisters[name].kind != DesiredCanisterKind::Root {
            continue;
        }
        contract::verify(source.inventory.canisters[name].principal, binding).unwrap();
    }
}

#[test]
fn pool_contract_requires_exact_query_request_response_and_method() {
    let mut types = candid::types::internal::TypeContainer::new();
    let request = types.add::<Request>();
    let response = types.add::<Result<Response, canic_core::dto::error::Error>>();
    let method = candid::types::TypeInner::Func(candid::types::Function {
        args: vec![request],
        rets: vec![response],
        modes: vec![candid::types::FuncMode::Query],
    })
    .into();
    let actor =
        candid::types::TypeInner::Service(vec![(protocol::CANIC_ROOT_STATUS.into(), method)])
            .into();
    let text = candid::pretty::candid::compile(&types.env, &Some(actor));
    assert!(contract::matches(&text).is_some());
    for changed in [
        text.replace(" query", ""),
        text.replace("canic_root_status", "different_method"),
        text.replace("next_start_after", "not_the_cursor"),
        text.replace("start_after", "not_the_request_cursor"),
    ] {
        assert_ne!(changed, text);
        assert!(contract::matches(&changed).is_none());
    }
}
