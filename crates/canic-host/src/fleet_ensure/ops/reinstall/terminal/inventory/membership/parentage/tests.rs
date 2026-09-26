//! Exact child membership, bounded pagination and retained source wire evidence.

use super::*;
use crate::fleet_ensure::ops::retained_contract::inspect_completed_source;
use std::path::Path;

fn child(id: u8) -> CanisterInfo {
    CanisterInfo {
        pid: Principal::from_slice(&[id]),
        role: "child".into(),
        parent_pid: Some(Principal::from_slice(&[99])),
        module_hash: None,
        created_at: 0,
    }
}

fn collector() -> Collector {
    Collector::new(
        Principal::from_slice(&[99]),
        [child(1), child(2)]
            .into_iter()
            .map(|child| (child.pid, child.role))
            .collect(),
    )
}

#[test]
fn bounded_pages_cover_exact_children_and_empty_leaves() {
    let mut actual = collector();
    assert!(
        !actual
            .push(Page {
                total: 2,
                entries: vec![child(1)]
            })
            .unwrap()
    );
    assert!(
        actual
            .push(Page {
                total: 2,
                entries: vec![child(2)]
            })
            .unwrap()
    );
    assert_eq!(actual.seen.len(), actual.expected.len());
    assert!(
        Collector::new(Principal::anonymous(), BTreeMap::new())
            .push(Page {
                total: 0,
                entries: vec![]
            })
            .unwrap()
    );
}

#[test]
fn missing_extra_duplicate_role_and_parent_changes_reject() {
    for mutation in [
        "missing",
        "extra",
        "duplicate",
        "role",
        "parent",
        "absent-parent",
        "changed-total",
    ] {
        let mut actual = collector();
        actual
            .push(Page {
                total: 2,
                entries: vec![child(1)],
            })
            .unwrap();
        let mut page = Page {
            total: 2,
            entries: vec![child(2)],
        };
        match mutation {
            "missing" => page.entries.clear(),
            "extra" => page.entries[0].pid = Principal::from_slice(&[3]),
            "duplicate" => page.entries[0] = child(1),
            "role" => page.entries[0].role = "other".into(),
            "parent" => page.entries[0].parent_pid = Some(Principal::anonymous()),
            "absent-parent" => page.entries[0].parent_pid = None,
            "changed-total" => page.total = 3,
            _ => unreachable!(),
        }
        assert!(
            matches!(
                actual.push(page),
                Err(CompletedParentageError::Membership { .. }
                    | CompletedParentageError::Pagination { .. })
            ),
            "{mutation}"
        );
    }
    let mut actual = collector();
    assert!(matches!(
        actual.push(Page {
            total: 2,
            entries: vec![child(1); usize::try_from(PAGE_SIZE).unwrap() + 1]
        }),
        Err(CompletedParentageError::Pagination { .. })
    ));
}

#[test]
fn child_directory_contract_requires_exact_query_and_fields() {
    let mut types = candid::types::internal::TypeContainer::new();
    let request = types.add::<Request>();
    let response = types.add::<Result<Response, canic_core::dto::error::Error>>();
    let method = candid::types::TypeInner::Func(candid::types::Function {
        args: vec![request],
        rets: vec![response],
        modes: vec![FuncMode::Query],
    })
    .into();
    let actor =
        candid::types::TypeInner::Service(vec![(protocol::CANIC_PUBLIC_STATUS.into(), method)])
            .into();
    let text = candid::pretty::candid::compile(&types.env, &Some(actor));
    assert!(matches(&text).is_some());
    for changed in [
        text.replace(" query", ""),
        text.replace("parent_pid", "parent"),
        text.replace("total", "count"),
        text.replace("limit", "maximum"),
    ] {
        assert_ne!(changed, text);
        assert!(matches(&changed).is_none());
    }
}

#[test]
#[ignore = "requires explicitly supplied read-only completed source records"]
fn supplied_source_applications_bind_exact_child_directory_contracts() {
    let workspace = std::env::var("CANIC_COMPLETED_SOURCE_WORKSPACE").unwrap();
    let environment = std::env::var("CANIC_COMPLETED_SOURCE_ENVIRONMENT").unwrap();
    let fleet = std::env::var("CANIC_COMPLETED_SOURCE_FLEET").unwrap();
    let source = inspect_completed_source(Path::new(&workspace), &environment, &fleet).unwrap();
    let mut applications = 0;
    for (name, entry) in &source.inventory.canisters {
        if entry.kind == DesiredCanisterKind::Component {
            verify_contract(entry.principal, &source.source_protocols[name]).unwrap();
            applications += 1;
        }
    }
    assert!(applications > 0);
}

#[test]
fn recorded_children_require_the_same_live_root_allocation() {
    use crate::fleet_ensure::{
        CompletedPoolAssetView, CompletedWorkloadAllocationView,
        ops::reinstall::terminal::inventory::tests::project_fixture,
    };
    let raw = serde_json::from_str(include_str!("../../fixture.json")).unwrap();
    let inventory = project_fixture(raw).unwrap();
    let parent_name = "root-0-pool-0";
    let parent = &inventory.canisters[parent_name];
    let child = &inventory.canisters["root-0-pool-20"];
    let root = &inventory.canisters["root-0"];
    let assets = [parent, child]
        .into_iter()
        .map(|entry| {
            (
                entry.principal,
                CompletedPoolAssetView {
                    kind: DesiredCanisterKind::Component,
                    allocation: Some(CompletedWorkloadAllocationView {
                        component: canic_core::ids::ComponentInstanceId::from_generated_bytes(
                            [1; 32],
                        ),
                        operation_id: [1; 32],
                    }),
                },
            )
        })
        .collect();
    let roots = BTreeMap::from([(
        "root-0".into(),
        CompletedRootMembershipView {
            root: root.principal,
            config: inventory.coordinator_registry.roots[0]
                .limits
                .canister_pool
                .clone(),
            completed_handoffs: 0,
            assets,
        },
    )]);
    let expected = expected_children(&inventory, &roots, parent_name, parent).unwrap();
    assert_eq!(
        expected.keys().copied().collect::<Vec<_>>(),
        vec![child.principal]
    );
    let mut leaf = parent.clone();
    leaf.protocol_binding
        .as_mut()
        .unwrap()
        .capabilities
        .remove(&RoleCapabilityKey::ChildProvisioning);
    assert!(matches!(
        expected_children(&inventory, &roots, parent_name, &leaf),
        Err(CompletedParentageError::Membership { .. })
    ));
    for mutation in ["component", "missing", "unallocated"] {
        let mut roots = roots.clone();
        let root = roots.get_mut("root-0").unwrap();
        match mutation {
            "component" => {
                root.assets
                    .get_mut(&child.principal)
                    .unwrap()
                    .allocation
                    .as_mut()
                    .unwrap()
                    .component =
                    canic_core::ids::ComponentInstanceId::from_generated_bytes([2; 32]);
            }
            "missing" => {
                root.assets.remove(&child.principal);
            }
            "unallocated" => root.assets.get_mut(&child.principal).unwrap().allocation = None,
            _ => unreachable!(),
        }
        assert!(matches!(
            expected_children(&inventory, &roots, parent_name, parent),
            Err(CompletedParentageError::Membership { .. })
        ));
    }
}
