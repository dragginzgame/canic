//! Read-only field projection and exact source membership; no old executable authority is built.

use super::*;
use crate::fleet_ensure::ops::retained_contract::inspect_completed_source;
use canic_core::dto::fleet_registry::FleetSubnetRootStatus;
use std::path::Path;

fn registry() -> RegistryFields {
    let raw: serde_json::Value = serde_json::from_str(include_str!("../../fixture.json")).unwrap();
    serde_json::from_value(raw["state"]["active_registry"].clone()).unwrap()
}

fn expected() -> CompletedCoordinatorMembershipView {
    let raw = registry();
    CompletedCoordinatorMembershipView {
        fleet: raw.authority.binding.fleet,
        coordinator: raw.authority.binding.coordinator,
        coordinator_subnet: raw.authority.binding.coordinator_subnet,
        epoch: raw.authority.epoch,
        revision: raw.revision,
        roots: raw.fleet_subnet_roots,
    }
}

#[test]
fn registry_projection_preserves_original_identity_epoch_revision_and_roots() {
    let expected = expected();
    let bytes = candid::encode_one(Ok::<_, canic_core::dto::error::Error>(Response::Registry(
        registry(),
    )))
    .unwrap();
    let response: Result<Response, canic_core::dto::error::Error> =
        candid::decode_one(&bytes).unwrap();
    let Response::Registry(actual) = response.unwrap();
    let observed = project(actual, &expected).unwrap();
    assert_eq!(observed, expected);
    assert_eq!(observed.epoch(), 1);
    assert_eq!(observed.revision(), 3);
    assert!(!observed.roots().is_empty());
}

#[test]
fn altered_registry_identity_revision_membership_and_policy_reject() {
    for mutation in [
        "fleet",
        "coordinator",
        "subnet",
        "epoch",
        "revision",
        "missing",
        "duplicate",
        "replacement",
        "placement",
        "draining",
        "policy",
    ] {
        let mut actual = registry();
        match mutation {
            "fleet" => actual.authority.binding.fleet.app = "another_app".into(),
            "coordinator" => actual.authority.binding.coordinator = Principal::from_slice(&[99]),
            "subnet" => {
                actual.authority.binding.coordinator_subnet = Principal::from_slice(&[99]).into();
            }
            "epoch" => actual.authority.epoch += 1,
            "revision" => actual.revision += 1,
            "missing" => actual.fleet_subnet_roots.clear(),
            "duplicate" => actual
                .fleet_subnet_roots
                .push(actual.fleet_subnet_roots[0].clone()),
            "replacement" => {
                actual.fleet_subnet_roots[0].fleet_subnet_root = Principal::from_slice(&[99]);
            }
            "placement" => {
                actual.fleet_subnet_roots[0].placement_subnet = Principal::from_slice(&[99]).into();
            }
            "draining" => actual.fleet_subnet_roots[0].status = FleetSubnetRootStatus::Draining,
            "policy" => {
                actual.fleet_subnet_roots[0]
                    .limits
                    .canister_pool
                    .maximum_size += 1;
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(
                project(actual, &expected()),
                Err(CompletedCoordinatorError::Membership)
            ),
            "{mutation}"
        );
    }
}

#[test]
fn registry_contract_binds_required_fields_and_query_mode() {
    let mut types = candid::types::internal::TypeContainer::new();
    let request = types.add::<Request>();
    let response = types.add::<Result<Response, canic_core::dto::error::Error>>();
    let method = candid::types::TypeInner::Func(candid::types::Function {
        args: vec![request],
        rets: vec![response],
        modes: vec![FuncMode::Query],
    })
    .into();
    let actor = candid::types::TypeInner::Service(vec![(
        protocol::CANIC_COORDINATOR_REGISTRY.into(),
        method,
    )])
    .into();
    let text = candid::pretty::candid::compile(&types.env, &Some(actor));
    assert!(matches(&text).is_some());
    for changed in [
        text.replace(" query", ""),
        text.replace("revision", "wrong_revision"),
        text.replace("coordinator_subnet", "wrong_subnet"),
        text.replace("fleet_subnet_roots", "wrong_roots"),
    ] {
        assert_ne!(changed, text);
        assert!(matches(&changed).is_none());
    }
}

#[test]
#[ignore = "requires explicit read-only completed-source workspace"]
fn inspect_supplied_completed_coordinator_query_contract() {
    let workspace = std::env::var("CANIC_COMPLETED_SOURCE_WORKSPACE").unwrap();
    let environment = std::env::var("CANIC_COMPLETED_SOURCE_ENVIRONMENT").unwrap();
    let fleet = std::env::var("CANIC_COMPLETED_SOURCE_FLEET").unwrap();
    let source = inspect_completed_source(Path::new(&workspace), &environment, &fleet).unwrap();
    let coordinator = source.inventory.coordinator;
    let (name, _) = source
        .inventory
        .canisters
        .iter()
        .find(|(_, row)| row.principal == coordinator)
        .unwrap();
    verify_contract(coordinator, &source.source_protocols[name]).unwrap();
    assert_eq!(source.inventory.coordinator_registry.epoch(), 1);
    assert_eq!(source.inventory.coordinator_registry.revision(), 3);
}
