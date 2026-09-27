//! Membership validation uses a reduced current record shape, without simulating IC calls.

use super::*;
use crate::fleet_ensure::model::FleetTerminalSourceRecord;
use serde_json::{Value, json};

fn fixture() -> Value {
    // Reduced from the supplied completed record: one hub, one descendant and one idle asset.
    // This fixture tests inventory projection, not semantic plan or receipt hashing.
    serde_json::from_str(include_str!("fixture.json")).unwrap()
}

fn receipts() -> CompletedReceiptAuditView {
    CompletedReceiptAuditView {
        documents: FleetTerminalSourceRecord {
            operation_id: "11".repeat(32),
            plan_sha256: "22".repeat(32),
            plan_document_sha256: "33".repeat(32),
            journal_document_sha256: "44".repeat(32),
            state_document_sha256: "55".repeat(32),
            phase_document_sha256: BTreeMap::new(),
        },
        effect_count: 1,
        phase_count: 1,
        source_operator: "operator".into(),
        cycles_ledger: "ledger".into(),
        initial_controlled_cycles: 1_000_000_000_000_000,
        initial_operator_cycles: 0,
        initial_estate_funding_cycles_by_root: BTreeMap::from([("root-0".into(), 0)]),
        recorded_funding_cycles: 0,
        recorded_operator_debit_cycles: 0,
        original_maximum_execution_burn_cycles: 500_000_000_000_000,
    }
}

pub(super) fn project_fixture(
    raw: Value,
) -> Result<CompletedEstateInventoryView, EnsureStateError> {
    project(
        receipts(),
        decode(raw["desired"].clone())?,
        decode(raw["state"].clone())?,
    )
}

#[test]
fn recorded_descendant_parentage_and_physical_pool_ownership_are_preserved() {
    let view = project_fixture(fixture()).unwrap();
    let root = &view.canisters["root-0"];
    let coordinator = &view.canisters["coordinator"];
    assert_ne!(root.subnet, coordinator.subnet);
    let descendant = &view.canisters["root-0-pool-20"];
    assert_eq!(descendant.kind, DesiredCanisterKind::Component);
    assert_eq!(descendant.parent.as_deref(), Some("root-0-pool-0"));
    assert_eq!(descendant.root.as_deref(), Some("root-0"));
    assert!(descendant.module_sha256.is_some());
    assert!(descendant.protocol_binding.is_some());
    let idle = &view.canisters["root-0-pool-10"];
    assert_eq!(idle.kind, DesiredCanisterKind::Pool);
    assert!(idle.module_sha256.is_none());
    assert_eq!(
        view.recorded_controlled_canister_cycles,
        view.canisters
            .values()
            .map(|canister| canister.recorded_cycles)
            .sum::<u128>()
    );
}

#[test]
fn missing_extra_substituted_and_duplicate_membership_rejects() {
    for mutation in [
        "missing",
        "extra",
        "substitute",
        "duplicate",
        "missing-balance",
        "extra-balance",
        "unknown-state",
    ] {
        let mut raw = fixture();
        match mutation {
            "missing" => {
                raw["state"]["topology"]
                    .as_object_mut()
                    .unwrap()
                    .remove("root-0-pool-20");
            }
            "extra" => {
                raw["state"]["principals"]["untracked"] =
                    raw["state"]["principals"]["root-0-pool-20"].clone();
            }
            "substitute" => {
                raw["state"]["principals"]["root-0-pool-20"] =
                    raw["state"]["principals"]["coordinator"].clone();
            }
            "duplicate" => {
                raw["desired"]["canisters"][5] = raw["desired"]["canisters"][4].clone();
            }
            "missing-balance" => {
                let id = raw["state"]["principals"]["coordinator"]
                    .as_str()
                    .unwrap()
                    .to_string();
                raw["state"]["retained_cycles_by_principal"]
                    .as_object_mut()
                    .unwrap()
                    .remove(&id);
            }
            "extra-balance" => {
                raw["state"]["retained_cycles_by_principal"]["untracked"] = json!(1);
            }
            "unknown-state" => {
                raw["state"]["unaccounted_canisters"] = json!([]);
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(
                project_fixture(raw),
                Err(EnsureStateError::InvalidTerminalSource)
            ),
            "{mutation}"
        );
    }
}

#[test]
fn inventory_cannot_overflow_or_replace_the_original_account_baselines() {
    let raw = fixture();
    let view = project_fixture(raw.clone()).unwrap();
    assert_eq!(
        view.receipts.initial_controlled_cycles,
        receipts().initial_controlled_cycles
    );
    assert_ne!(
        view.recorded_controlled_canister_cycles,
        view.receipts.initial_controlled_cycles
    );
    let mut state: CompletedStateEvidence = decode(raw["state"].clone()).unwrap();
    state
        .retained_cycles_by_principal
        .values_mut()
        .for_each(|amount| *amount = u128::MAX);
    assert!(matches!(
        project(receipts(), decode(raw["desired"].clone()).unwrap(), state),
        Err(EnsureStateError::InvalidTerminalSource)
    ));
    let mut wrong_accounts = receipts();
    wrong_accounts.initial_estate_funding_cycles_by_root =
        BTreeMap::from([("another-root".into(), 0)]);
    assert!(matches!(
        project(
            wrong_accounts,
            decode(raw["desired"].clone()).unwrap(),
            decode(raw["state"].clone()).unwrap()
        ),
        Err(EnsureStateError::InvalidTerminalSource)
    ));
}

#[test]
fn broken_parentage_modules_registry_and_subnet_bindings_reject() {
    for (pointer, value) in [
        (
            "/desired/bootstrap/recovery_controllers",
            json!(["aaaaa-aa"]),
        ),
        (
            "/state/topology/root-0-pool-20/parent",
            json!("root-0-pool-20"),
        ),
        (
            "/state/topology/root-0-pool-0/parent",
            json!("root-0-pool-20"),
        ),
        (
            "/state/topology/root-0-pool-20/parent",
            json!("coordinator"),
        ),
        (
            "/state/topology/root-0-pool-20/parent",
            json!("root-0-pool-10"),
        ),
        ("/state/topology/root-0-pool-20/parent", json!("unknown")),
        ("/state/topology/root-0-pool-20/module_hash", Value::Null),
        ("/state/topology/root-0-pool-20/role", json!("other_role")),
        (
            "/state/topology/root-0-pool-20/protocol_binding",
            Value::Null,
        ),
        (
            "/state/active_registry/authority/binding/fleet/app",
            json!("other_app"),
        ),
        (
            "/state/active_registry/fleet_subnet_roots/0/status",
            json!("Joining"),
        ),
        (
            "/desired/bootstrap/roots/0/canister_pool_imports/0",
            json!("store-0"),
        ),
        (
            "/desired/canisters/5/subnet",
            json!("pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae"),
        ),
        ("/desired/canisters/0/controllers/0", json!("2vxsx-fae")),
        ("/desired/canisters/0/presence", json!("absent")),
        ("/desired/canisters/5/parent", json!("coordinator")),
    ] {
        let mut raw = fixture();
        *raw.pointer_mut(pointer).unwrap() = value;
        assert!(
            matches!(
                project_fixture(raw),
                Err(EnsureStateError::InvalidTerminalSource)
            ),
            "{pointer}"
        );
    }
}

#[test]
#[ignore = "requires explicitly supplied read-only completed source records"]
fn inspect_supplied_completed_inventory_without_external_mutation() {
    let workspace = std::env::var_os("CANIC_COMPLETED_SOURCE_WORKSPACE").unwrap();
    let environment = std::env::var("CANIC_COMPLETED_SOURCE_ENVIRONMENT").unwrap();
    let fleet = std::env::var("CANIC_COMPLETED_SOURCE_FLEET").unwrap();
    let source = crate::fleet_ensure::ops::retained_contract::inspect_completed_source(
        std::path::Path::new(&workspace),
        &environment,
        &fleet,
    )
    .unwrap();
    let mut view = source.inventory;
    assert_eq!(
        source.source_protocols.len(),
        view.canisters
            .values()
            .filter(|canister| matches!(
                canister.kind,
                DesiredCanisterKind::Coordinator
                    | DesiredCanisterKind::Root
                    | DesiredCanisterKind::Store
                    | DesiredCanisterKind::Component
            ))
            .count()
    );
    assert!(!view.canisters.is_empty());
    assert!(
        view.canisters
            .values()
            .any(|canister| canister.kind == DesiredCanisterKind::Component)
    );
    assert!(view.recorded_controlled_canister_cycles > 0);
    // A genuine module/interface from the same finalized release is insufficient
    // when substituted for another infrastructure kind. Only this in-memory view changes.
    let root = view
        .canisters
        .values()
        .find(|entry| entry.kind == DesiredCanisterKind::Root)
        .unwrap()
        .clone();
    let coordinator = view
        .canisters
        .values_mut()
        .find(|entry| entry.kind == DesiredCanisterKind::Coordinator)
        .unwrap();
    coordinator.module_sha256 = root.module_sha256;
    coordinator.protocol_binding = root.protocol_binding;
    assert!(matches!(
        protocols::inspect(std::path::Path::new(&workspace), &view),
        Err(protocols::CompletedSourceProtocolError::Binding(
            crate::protocol_binding::ReleaseProtocolBindingError::ArtifactBindingMismatch { .. }
        ))
    ));
}
